//! Motore: un thread possiede la sorgente, applica il trigger e pubblica i frame.

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use h6022_dsp::Waveform;
use h6022_usb::ChannelMode;

use crate::source::{SampleSource, VoltBlock};
use crate::trigger::Trigger;
use crate::types::{CoreError, CoreResult, EngineConfig, EngineStats, Frame, RunState, TriggerMode};

/// In modalità Auto, senza trigger per questo tempo il motore pubblica un frame non sincronizzato.
pub const AUTO_TIMEOUT: Duration = Duration::from_millis(100);
/// In modalità Roll il motore pubblica un frame a questo intervallo.
pub const ROLL_INTERVAL: Duration = Duration::from_millis(50);

/// Distanza minima tra due frame sincronizzati.
const MIN_FRAME_INTERVAL: Duration = Duration::from_millis(16);
/// Il buffer circolare per canale contiene questo numero di record.
const RING_RECORDS: usize = 4;
/// Coda di ogni iscritto: un ricevitore lento perde frame.
const SUBSCRIBER_QUEUE: usize = 2;
const RECORD_LENGTH_MIN: usize = 16;
const RECORD_LENGTH_MAX: usize = 16_777_216;

pub struct Engine;

impl Engine {
    /// Configura la sorgente e avvia il thread di acquisizione.
    pub fn start(mut source: Box<dyn SampleSource>, config: EngineConfig) -> CoreResult<EngineHandle> {
        validate(&config)?;
        source.configure(&config.acquisition)?;
        let shared = Arc::new(Mutex::new(Shared {
            config,
            state: state_for(config.mode, false, false),
            latest: None,
            subscribers: Vec::new(),
            stats: EngineStats::default(),
            closed: false,
        }));
        let (commands, inbox) = mpsc::channel();
        let worker = Worker::new(source, config, inbox, Arc::clone(&shared));
        let thread = thread::spawn(move || worker.run());
        Ok(EngineHandle {
            shared,
            commands,
            thread: Arc::new(Mutex::new(Some(thread))),
        })
    }
}

/// Stato letto dai client e aggiornato dal thread.
struct Shared {
    config: EngineConfig,
    state: RunState,
    latest: Option<Arc<Frame>>,
    subscribers: Vec<SyncSender<Arc<Frame>>>,
    stats: EngineStats,
    /// Vero quando il thread è terminato: i nuovi iscritti ricevono un canale chiuso.
    closed: bool,
}

/// Comandi dal handle al thread.
enum Command {
    /// Il thread risponde con l'esito sul canale di risposta.
    SetConfig(EngineConfig, Sender<CoreResult<()>>),
    Run,
    Pause,
    Shutdown,
}

/// Controllo del motore, clonabile e condivisibile tra thread.
#[derive(Clone)]
pub struct EngineHandle {
    shared: Arc<Mutex<Shared>>,
    commands: Sender<Command>,
    thread: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl EngineHandle {
    /// Applica una nuova configurazione. Se cambia `acquisition`, riconfigura la sorgente.
    pub fn set_config(&self, config: EngineConfig) -> CoreResult<()> {
        let (reply, answer) = mpsc::channel();
        self.commands
            .send(Command::SetConfig(config, reply))
            .map_err(|_| CoreError::Stopped)?;
        answer.recv().map_err(|_| CoreError::Stopped)?
    }

    pub fn config(&self) -> EngineConfig {
        lock(&self.shared).config
    }

    /// Ultimo frame pubblicato.
    pub fn latest_frame(&self) -> Option<Arc<Frame>> {
        lock(&self.shared).latest.clone()
    }

    /// Canale che riceve ogni nuovo frame. Un ricevitore lento perde frame, non blocca il motore.
    pub fn subscribe(&self) -> Receiver<Arc<Frame>> {
        let (tx, rx) = mpsc::sync_channel(SUBSCRIBER_QUEUE);
        let mut shared = lock(&self.shared);
        if !shared.closed {
            shared.subscribers.push(tx);
        }
        rx
    }

    /// Riarma la modalità Single, o riprende dopo `pause`.
    pub fn run(&self) {
        let _ = self.commands.send(Command::Run);
    }

    /// Ferma la pubblicazione dei frame; l'acquisizione continua.
    pub fn pause(&self) {
        let _ = self.commands.send(Command::Pause);
    }

    pub fn state(&self) -> RunState {
        lock(&self.shared).state
    }

    pub fn stats(&self) -> EngineStats {
        lock(&self.shared).stats
    }

    /// Ferma il thread e la sorgente. Le chiamate successive restituiscono `CoreError::Stopped`.
    pub fn shutdown(&self) -> CoreResult<()> {
        let thread = lock(&self.thread).take().ok_or(CoreError::Stopped)?;
        let _ = self.commands.send(Command::Shutdown);
        let _ = thread.join();
        Ok(())
    }
}

/// Thread di acquisizione: possiede la sorgente, il trigger e il buffer.
struct Worker {
    source: Box<dyn SampleSource>,
    inbox: Receiver<Command>,
    shared: Arc<Mutex<Shared>>,
    config: EngineConfig,
    trigger: Trigger,
    /// Campioni scalati dalla sonda, uno buffer per canale (ch2 vuoto in modalità a un canale).
    /// Il primo elemento di `ring[i]` ha indice assoluto `ring_start()`.
    ring: [VecDeque<f32>; 2],
    /// Campioni ricevuti dall'ultima configurazione: indice del prossimo campione.
    received: u64,
    /// Indice assoluto del primo campione del segmento corrente.
    segment_start: u64,
    /// Trigger in attesa che il buffer contenga il record completo.
    pending: VecDeque<u64>,
    paused: bool,
    /// Modalità Single: vero dopo il primo frame, fino a `run`.
    single_done: bool,
    /// Istante dell'ultimo frame, per la distanza minima tra frame.
    last_frame: Option<Instant>,
    /// Ultimo frame di qualsiasi tipo: riferimento per i timer di Auto e Roll.
    quiet_since: Instant,
    window_start: Instant,
    window_samples: u64,
    block: VoltBlock,
}

impl Worker {
    fn new(
        source: Box<dyn SampleSource>,
        config: EngineConfig,
        inbox: Receiver<Command>,
        shared: Arc<Mutex<Shared>>,
    ) -> Worker {
        let now = Instant::now();
        Worker {
            source,
            inbox,
            shared,
            config,
            trigger: Trigger::new(config.trigger, hz_of(&config)),
            ring: [VecDeque::new(), VecDeque::new()],
            received: 0,
            segment_start: 0,
            pending: VecDeque::new(),
            paused: false,
            single_done: false,
            last_frame: None,
            quiet_since: now,
            window_start: now,
            window_samples: 0,
            block: VoltBlock::default(),
        }
    }

    fn run(mut self) {
        loop {
            if !self.drain_commands() {
                break;
            }
            match self.source.read(&mut self.block) {
                Ok(()) => {
                    if !self.process_block() {
                        break;
                    }
                }
                // Nessun dato ancora: si riprova.
                Err(CoreError::Device(h6022_usb::Error::Timeout)) => {}
                Err(_) => break,
            }
            self.timed_publish(Instant::now());
        }
        let _ = self.source.stop();
        let mut shared = lock(&self.shared);
        shared.state = RunState::Stopped;
        shared.subscribers.clear();
        shared.closed = true;
    }

    /// Esegue i comandi in coda. Restituisce `false` quando il thread deve terminare.
    fn drain_commands(&mut self) -> bool {
        loop {
            match self.inbox.try_recv() {
                Ok(Command::SetConfig(config, reply)) => {
                    let _ = reply.send(self.apply_config(config));
                }
                Ok(Command::Run) => {
                    self.paused = false;
                    self.single_done = false;
                    self.pending.clear();
                    self.quiet_since = Instant::now();
                    self.refresh_state();
                }
                Ok(Command::Pause) => {
                    self.paused = true;
                    self.pending.clear();
                    self.refresh_state();
                }
                Ok(Command::Shutdown) | Err(TryRecvError::Disconnected) => return false,
                Err(TryRecvError::Empty) => return true,
            }
        }
    }

    fn apply_config(&mut self, config: EngineConfig) -> CoreResult<()> {
        validate(&config)?;
        if config.acquisition != self.config.acquisition {
            self.source.configure(&config.acquisition)?;
            self.ring[0].clear();
            self.ring[1].clear();
            self.received = 0;
            self.segment_start = 0;
            self.pending.clear();
            self.trigger = Trigger::new(config.trigger, hz_of(&config));
            lock(&self.shared).stats.last_segment_samples = 0;
        } else if config.trigger != self.config.trigger {
            self.pending.clear();
            self.trigger = Trigger::new(config.trigger, hz_of(&config));
        }
        self.config = config;
        lock(&self.shared).config = config;
        self.refresh_state();
        Ok(())
    }

    /// Elabora un blocco dalla sorgente. Restituisce `false` se il blocco è incoerente.
    fn process_block(&mut self) -> bool {
        let dual = self.is_dual();
        let n = self.block.ch1.len();
        if dual && self.block.ch2.len() != n {
            return false;
        }
        let [probe1, probe2] = self.config.probe;
        // Il fattore di sonda si applica sul posto: la sorgente riscrive il blocco a ogni lettura.
        scale(&mut self.block.ch1, probe1);
        if dual {
            scale(&mut self.block.ch2, probe2);
        }
        // Ogni tratto tra due interruzioni è continuo: trigger e frame non le attraversano.
        let block = std::mem::take(&mut self.block);
        let ch2 = |from: usize, to: usize| -> &[f32] { if dual { &block.ch2[from..to] } else { &[] } };
        let mut triggers = 0;
        let mut from = 0;
        for &at in &block.breaks {
            let at = at.clamp(from, n);
            triggers += self.feed(&block.ch1[from..at], ch2(from, at));
            self.begin_segment();
            from = at;
        }
        triggers += self.feed(&block.ch1[from..], ch2(from, n));
        self.block = block;
        self.account(n, triggers, Instant::now());
        true
    }

    /// Campioni consecutivi: trigger, buffer e frame pronti. Restituisce i trigger trovati.
    fn feed(&mut self, s1: &[f32], s2: &[f32]) -> usize {
        let triggers = self.trigger.feed(s1, s2, self.received);
        let cap = RING_RECORDS * self.config.record_length;
        push_capped(&mut self.ring[0], s1, cap);
        if !s2.is_empty() {
            push_capped(&mut self.ring[1], s2, cap);
        }
        self.received += s1.len() as u64;

        if self.collects_triggers() {
            self.pending.extend(triggers.iter().copied());
        }
        self.drain_pending(Instant::now());
        triggers.len()
    }

    /// Dopo un'interruzione i campioni precedenti non sono più contigui ai successivi:
    /// il buffer e i trigger pendenti ripartono da zero, il trigger si riarma.
    /// I trigger pendenti contano come tagliati: il loro record non può più completarsi.
    fn begin_segment(&mut self) {
        let length = self.received - self.segment_start;
        self.segment_start = self.received;
        let mut shared = lock(&self.shared);
        shared.stats.segments += 1;
        shared.stats.triggers_cut += self.pending.len() as u64;
        if length > 0 {
            shared.stats.last_segment_samples = length;
        }
        drop(shared);
        self.ring[0].clear();
        self.ring[1].clear();
        self.pending.clear();
        self.trigger.reset();
    }

    /// Genera i frame dei trigger pendenti il cui record è completo.
    fn drain_pending(&mut self, now: Instant) {
        let len = self.config.record_length;
        let pre = self.pre();
        let hz = self.hz();
        while let Some(t) = self.pending.front().copied() {
            let start = t as i64 - pre as i64;
            if (self.received as i64) < start + len as i64 {
                break;
            }
            self.pending.pop_front();
            let ring_start = self.ring_start() as i64;
            // Il campione t - pre non è più nel buffer, o il trigger è troppo vicino al precedente.
            if start < ring_start {
                continue;
            }
            if self.last_frame.is_some_and(|last| now.duration_since(last) < MIN_FRAME_INTERVAL) {
                continue;
            }
            let (ch1, ch2) = self.cut((start - ring_start) as usize, len);
            let t0 = -(pre as f64) / hz;
            self.publish(ch1, ch2, t0, true, Some(t), now);
            if self.config.mode == TriggerMode::Single {
                self.single_done = true;
                self.pending.clear();
                self.refresh_state();
                break;
            }
        }
    }

    /// Frame a intervalli di tempo: Auto senza trigger, Roll sempre.
    fn timed_publish(&mut self, now: Instant) {
        if self.paused || self.single_done {
            return;
        }
        let len = self.config.record_length;
        let avail = self.ring[0].len();
        let hz = self.hz();
        match self.config.mode {
            TriggerMode::Auto => {
                if avail < len || now.duration_since(self.quiet_since) < AUTO_TIMEOUT {
                    return;
                }
                let t0 = -(self.pre() as f64) / hz;
                let (ch1, ch2) = self.cut(avail - len, len);
                self.publish(ch1, ch2, t0, false, None, now);
            }
            TriggerMode::Roll => {
                if avail == 0 || now.duration_since(self.quiet_since) < ROLL_INTERVAL {
                    return;
                }
                let n = avail.min(len);
                let t0 = -(n as f64) / hz;
                let (ch1, ch2) = self.cut(avail - n, n);
                self.publish(ch1, ch2, t0, false, None, now);
            }
            TriggerMode::Normal | TriggerMode::Single => {}
        }
    }

    fn publish(
        &mut self,
        ch1: Vec<f32>,
        ch2: Option<Vec<f32>>,
        t0: f64,
        triggered: bool,
        trigger_sample: Option<u64>,
        now: Instant,
    ) {
        let hz = self.hz();
        let mut shared = lock(&self.shared);
        let frame = Arc::new(Frame {
            sequence: shared.stats.frames,
            ch1: Waveform {
                samples: ch1,
                sample_rate: hz,
                t0,
            },
            ch2: ch2.map(|samples| Waveform {
                samples,
                sample_rate: hz,
                t0,
            }),
            triggered,
            trigger_sample,
            config: self.config,
        });
        shared.stats.frames += 1;
        shared.latest = Some(Arc::clone(&frame));
        shared.subscribers.retain(|tx| match tx.try_send(Arc::clone(&frame)) {
            Ok(()) | Err(TrySendError::Full(_)) => true,
            Err(TrySendError::Disconnected(_)) => false,
        });
        drop(shared);
        self.last_frame = Some(now);
        self.quiet_since = now;
    }

    /// Copia `n` campioni per canale a partire dall'indice `from` del buffer.
    fn cut(&self, from: usize, n: usize) -> (Vec<f32>, Option<Vec<f32>>) {
        let ch1: Vec<f32> = self.ring[0].range(from..from + n).copied().collect();
        let ch2 = if self.is_dual() {
            let samples: Vec<f32> = self.ring[1].range(from..from + n).copied().collect();
            Some(samples)
        } else {
            None
        };
        (ch1, ch2)
    }

    fn account(&mut self, samples: usize, triggers: usize, now: Instant) {
        self.window_samples += samples as u64;
        let mut shared = lock(&self.shared);
        shared.stats.samples += samples as u64;
        shared.stats.triggers += triggers as u64;
        let elapsed = now.duration_since(self.window_start);
        if elapsed >= Duration::from_secs(1) {
            shared.stats.sample_rate_measured = self.window_samples as f64 / elapsed.as_secs_f64();
            self.window_samples = 0;
            self.window_start = now;
        }
    }

    fn refresh_state(&self) {
        lock(&self.shared).state = state_for(self.config.mode, self.paused, self.single_done);
    }

    fn is_dual(&self) -> bool {
        matches!(self.config.acquisition.mode, ChannelMode::Dual)
    }

    /// I trigger contano sempre, ma generano frame solo se il motore pubblica.
    fn collects_triggers(&self) -> bool {
        !self.paused && !self.single_done && self.config.mode != TriggerMode::Roll
    }

    /// Indice assoluto del primo campione nel buffer.
    fn ring_start(&self) -> u64 {
        self.received - self.ring[0].len() as u64
    }

    fn pre(&self) -> usize {
        pre_samples(&self.config)
    }

    fn hz(&self) -> f64 {
        hz_of(&self.config)
    }
}

fn scale(samples: &mut [f32], factor: f32) {
    if factor != 1.0 {
        samples.iter_mut().for_each(|v| *v *= factor);
    }
}

fn push_capped(ring: &mut VecDeque<f32>, samples: &[f32], cap: usize) {
    // Scarta prima i campioni in eccesso: se il blocco supera cap, solo la sua coda entra nel buffer.
    let samples = &samples[samples.len().saturating_sub(cap)..];
    let excess = (ring.len() + samples.len()).saturating_sub(cap);
    ring.drain(..excess.min(ring.len()));
    ring.extend(samples.iter().copied());
}

fn state_for(mode: TriggerMode, paused: bool, single_done: bool) -> RunState {
    if paused || single_done {
        return RunState::Stopped;
    }
    match mode {
        TriggerMode::Normal | TriggerMode::Single => RunState::Armed,
        TriggerMode::Auto | TriggerMode::Roll => RunState::Running,
    }
}

fn validate(config: &EngineConfig) -> CoreResult<()> {
    if !(RECORD_LENGTH_MIN..=RECORD_LENGTH_MAX).contains(&config.record_length) {
        return Err(CoreError::InvalidConfig(format!(
            "record_length {} fuori da {RECORD_LENGTH_MIN}..={RECORD_LENGTH_MAX}",
            config.record_length
        )));
    }
    if !(0.0..=1.0).contains(&config.pretrigger) {
        return Err(CoreError::InvalidConfig(format!(
            "pretrigger {} fuori da 0..=1",
            config.pretrigger
        )));
    }
    if !config.probe.iter().all(|&p| p > 0.0 && p.is_finite()) {
        return Err(CoreError::InvalidConfig(format!(
            "attenuazione sonda non valida: {:?}",
            config.probe
        )));
    }
    Ok(())
}

fn pre_samples(config: &EngineConfig) -> usize {
    (f64::from(config.pretrigger) * config.record_length as f64).round() as usize
}

fn hz_of(config: &EngineConfig) -> f64 {
    f64::from(config.acquisition.sample_rate.hz())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
