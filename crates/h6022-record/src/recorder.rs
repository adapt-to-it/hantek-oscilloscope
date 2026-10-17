//! Registrazione in un thread che legge i frame dal motore.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use h6022_core::{EngineHandle, Frame};

use crate::{Format, FrameWriter, RecordLimit, RecordStatus, RecordSummary, create};

/// Intervallo con cui il thread controlla il flag di stop mentre aspetta un frame.
const POLL: Duration = Duration::from_millis(100);

/// Stato condiviso tra il thread di registrazione e chi lo interroga.
#[derive(Debug, Default)]
struct Progress {
    frames: u64,
    running: bool,
    error: Option<String>,
}

pub struct Recorder {
    path: PathBuf,
    format: Format,
    progress: Arc<Mutex<Progress>>,
    stop: Arc<AtomicBool>,
    /// `None` solo dopo `stop`, che consuma il Recorder.
    thread: Option<JoinHandle<Box<dyn FrameWriter>>>,
}

impl Recorder {
    /// Crea il file, si iscrive al motore e scrive ogni frame in un thread proprio.
    pub fn start(engine: &EngineHandle, path: &Path, format: Format, limit: RecordLimit) -> io::Result<Recorder> {
        let writer = create(path, format)?;
        let frames = engine.subscribe();
        let progress = Arc::new(Mutex::new(Progress {
            running: true,
            ..Progress::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let progress = Arc::clone(&progress);
            let stop = Arc::clone(&stop);
            thread::spawn(move || run(writer, frames, limit, &stop, &progress))
        };
        Ok(Recorder {
            path: path.to_path_buf(),
            format,
            progress,
            stop,
            thread: Some(thread),
        })
    }

    pub fn status(&self) -> RecordStatus {
        let progress = lock(&self.progress);
        RecordStatus {
            path: self.path.clone(),
            format: self.format,
            frames: progress.frames,
            running: progress.running,
            error: progress.error.clone(),
        }
    }

    /// Ferma il thread, completa il file e restituisce il riepilogo.
    pub fn stop(mut self) -> io::Result<RecordSummary> {
        self.stop.store(true, Ordering::SeqCst);
        let handle = self.thread.take().expect("il thread è presente fino a stop");
        let writer = handle
            .join()
            .map_err(|_| io::Error::other("il thread di registrazione è terminato in modo anomalo"))?;
        let summary = writer.finish();
        let error = lock(&self.progress).error.clone();
        match error {
            Some(message) => Err(io::Error::other(message)),
            None => summary,
        }
    }
}

impl Drop for Recorder {
    /// Segnala lo stop senza attendere il thread: il file non viene completato.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Corpo del thread: scrive i frame finché non arriva lo stop, un limite o un errore.
/// Restituisce lo scrittore, che `stop` completa.
fn run(
    mut writer: Box<dyn FrameWriter>,
    frames: Receiver<Arc<Frame>>,
    limit: RecordLimit,
    stop: &AtomicBool,
    progress: &Mutex<Progress>,
) -> Box<dyn FrameWriter> {
    let started = Instant::now();
    while !stop.load(Ordering::SeqCst) {
        let written = lock(progress).frames;
        if limit.max_frames.is_some_and(|max| written >= max) {
            break;
        }
        if limit.max_seconds.is_some_and(|max| started.elapsed().as_secs_f64() >= max) {
            break;
        }
        match frames.recv_timeout(POLL) {
            Ok(frame) => match writer.write_frame(&frame) {
                Ok(()) => lock(progress).frames += 1,
                Err(err) => {
                    lock(progress).error = Some(err.to_string());
                    break;
                }
            },
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    lock(progress).running = false;
    writer
}

fn lock(progress: &Mutex<Progress>) -> MutexGuard<'_, Progress> {
    progress.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{config, scratch};
    use h6022_core::sim::SimulatedSource;
    use h6022_core::{Engine, TriggerMode};

    #[test]
    fn si_ferma_al_limite_di_frame() {
        let dir = scratch("recorder");
        let path = dir.join("rec.csv");
        let engine = Engine::start(
            Box::new(SimulatedSource::bench_default()),
            config(TriggerMode::Roll, true, 1000),
        )
        .unwrap();
        let limit = RecordLimit {
            max_frames: Some(3),
            max_seconds: None,
        };
        let recorder = Recorder::start(&engine, &path, Format::Csv, limit).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while recorder.status().running {
            assert!(Instant::now() < deadline, "la registrazione non si è fermata al limite");
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(recorder.status().frames, 3);
        assert!(recorder.status().error.is_none());
        let summary = recorder.stop().unwrap();
        assert_eq!(summary.frames, 3);
        engine.shutdown().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
