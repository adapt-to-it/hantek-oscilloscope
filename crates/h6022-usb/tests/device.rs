//! Test di `Hantek6022` e `Stream` con un trasporto e una sorgente bulk simulati.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use h6022_usb::{
    AcquisitionConfig, BulkSource, Block, Channel, ChannelMode, Coupling, Error, Hantek6022,
    Result, SampleRate, StreamConfig, StreamStats, Transport, VoltageRange, BULK_EP_IN,
    CALIBRATION_EEPROM_LEN, CALIBRATION_EEPROM_OFFSET, FIRMWARE_VERSION, FirmwareInfo, req,
};

/// Operazione registrata dal trasporto simulato, nell'ordine di esecuzione.
#[derive(Debug, PartialEq)]
enum Event {
    ControlIn { request: u8, value: u16, index: u16, len: u16 },
    ControlOut { request: u8, value: u16, index: u16, data: Vec<u8> },
    OpenBulk { endpoint: u8, transfer_size: usize, queue_depth: usize },
}

fn out(request: u8, data: &[u8]) -> Event {
    Event::ControlOut {
        request,
        value: 0,
        index: 0,
        data: data.to_vec(),
    }
}

#[derive(Default)]
struct Shared {
    log: RefCell<Vec<Event>>,
    /// Richiesta di controllo che fallisce, se impostata.
    fail: Cell<Option<u8>>,
    /// Trasferimenti bulk da restituire, in ordine.
    chunks: RefCell<VecDeque<Vec<u8>>>,
    /// Se vero, START non mette il marcatore di avvio in testa a `chunks`:
    /// il test lo inserisce da sé.
    manual_start_marker: Cell<bool>,
}

struct Fake {
    shared: Rc<Shared>,
}

struct FakeBulk {
    shared: Rc<Shared>,
}

impl Transport for Fake {
    type Bulk = FakeBulk;

    fn control_out(&self, request: u8, value: u16, index: u16, data: &[u8]) -> Result<()> {
        self.shared.log.borrow_mut().push(Event::ControlOut {
            request,
            value,
            index,
            data: data.to_vec(),
        });
        if self.shared.fail.get() == Some(request) {
            return Err(Error::Usb("guasto simulato".to_string()));
        }
        // Come il firmware 3.02: ogni START apre lo stream con il marcatore di sequenza 0.
        // Il marcatore entra nel primo trasferimento; senza trasferimenti non arriva nulla.
        if request == req::START_STOP
            && data == [1]
            && !self.shared.manual_start_marker.get()
            && let Some(first) = self.shared.chunks.borrow_mut().front_mut()
        {
            first.splice(0..0, marker(0));
        }
        Ok(())
    }

    fn control_in(&self, request: u8, value: u16, index: u16, len: u16) -> Result<Vec<u8>> {
        self.shared.log.borrow_mut().push(Event::ControlIn {
            request,
            value,
            index,
            len,
        });
        Ok(vec![0x80; usize::from(len)])
    }

    fn open_bulk(&self, endpoint: u8, transfer_size: usize, queue_depth: usize) -> Result<FakeBulk> {
        self.shared.log.borrow_mut().push(Event::OpenBulk {
            endpoint,
            transfer_size,
            queue_depth,
        });
        Ok(FakeBulk {
            shared: Rc::clone(&self.shared),
        })
    }
}

impl BulkSource for FakeBulk {
    fn read_into(&mut self, out: &mut Vec<u8>, _timeout: Duration) -> Result<usize> {
        let next = self.shared.chunks.borrow_mut().pop_front();
        match next {
            Some(chunk) => {
                out.extend_from_slice(&chunk);
                Ok(chunk.len())
            }
            None => Err(Error::Timeout),
        }
    }
}

/// Dispositivo con la sequenza di inizializzazione già registrata e svuotata.
fn device() -> (Hantek6022<Fake>, Rc<Shared>) {
    device_with_version(0x0100)
}

fn device_with_version(firmware_version: u16) -> (Hantek6022<Fake>, Rc<Shared>) {
    let shared = Rc::new(Shared::default());
    let dev = Hantek6022::with_transport(
        Fake {
            shared: Rc::clone(&shared),
        },
        firmware_version,
    )
    .expect("with_transport");
    shared.log.borrow_mut().clear();
    (dev, shared)
}

#[test]
fn with_transport_reads_calibration_then_sends_defaults() {
    let shared = Rc::new(Shared::default());
    let dev = Hantek6022::with_transport(
        Fake {
            shared: Rc::clone(&shared),
        },
        0x0100,
    )
    .expect("with_transport");

    assert_eq!(
        *shared.log.borrow(),
        vec![
            Event::ControlIn {
                request: req::EEPROM,
                value: CALIBRATION_EEPROM_OFFSET,
                index: 0,
                len: CALIBRATION_EEPROM_LEN,
            },
            out(req::SAMPLE_RATE, &[1]),
            out(req::NUM_CHANNELS, &[2]),
            out(req::CH1_RANGE, &[1]),
            out(req::CH2_RANGE, &[1]),
            out(req::COUPLING, &[0x11]),
            out(req::COUPLING, &[0x11]),
        ]
    );
    assert_eq!(dev.firmware_version(), 0x0100);
    assert_eq!(
        dev.config(),
        AcquisitionConfig {
            sample_rate: SampleRate(1),
            mode: ChannelMode::Dual,
            range: [VoltageRange::V5, VoltageRange::V5],
            coupling: [Coupling::Dc, Coupling::Dc],
        }
    );
}

#[test]
fn with_transport_fails_if_a_default_command_fails() {
    let shared = Rc::new(Shared::default());
    shared.fail.set(Some(req::NUM_CHANNELS));
    let result = Hantek6022::with_transport(
        Fake {
            shared: Rc::clone(&shared),
        },
        0x0100,
    );
    assert!(matches!(result, Err(Error::Usb(_))));
}

#[test]
fn read_eeprom_passes_offset_and_length() {
    let (dev, shared) = device();
    let bytes = dev.read_eeprom(0x10, 4).expect("read_eeprom");
    assert_eq!(bytes, vec![0x80; 4]);
    assert_eq!(
        *shared.log.borrow(),
        vec![Event::ControlIn {
            request: req::EEPROM,
            value: 0x10,
            index: 0,
            len: 4,
        }]
    );
}

#[test]
fn set_sample_rate_sends_known_id_and_updates_config() {
    let (mut dev, shared) = device();
    dev.set_sample_rate(SampleRate(2)).expect("set_sample_rate");
    assert_eq!(dev.config().sample_rate, SampleRate(2));
    assert_eq!(*shared.log.borrow(), vec![out(req::SAMPLE_RATE, &[2])]);
}

#[test]
fn set_sample_rate_rejects_unknown_id_without_sending() {
    let (mut dev, shared) = device();
    let result = dev.set_sample_rate(SampleRate(200));
    assert!(matches!(result, Err(Error::InvalidArgument(_))));
    assert!(shared.log.borrow().is_empty());
    assert_eq!(dev.config().sample_rate, SampleRate(1));
}

#[test]
fn set_voltage_range_uses_channel_request_and_updates_that_channel() {
    let (mut dev, shared) = device();
    dev.set_voltage_range(Channel::Ch2, VoltageRange::V1)
        .expect("set_voltage_range");
    assert_eq!(*shared.log.borrow(), vec![out(req::CH2_RANGE, &[5])]);
    assert_eq!(
        dev.config().range,
        [VoltageRange::V5, VoltageRange::V1]
    );
}

#[test]
fn failed_voltage_range_keeps_config() {
    let (mut dev, shared) = device();
    shared.fail.set(Some(req::CH1_RANGE));
    assert!(dev.set_voltage_range(Channel::Ch1, VoltageRange::V0_5).is_err());
    assert_eq!(dev.config().range, [VoltageRange::V5, VoltageRange::V5]);
}

#[test]
fn set_channel_mode_sends_discriminant() {
    let (mut dev, shared) = device();
    dev.set_channel_mode(ChannelMode::Single).expect("set_channel_mode");
    assert_eq!(dev.config().mode, ChannelMode::Single);
    assert_eq!(*shared.log.borrow(), vec![out(req::NUM_CHANNELS, &[1])]);
}

#[test]
fn set_coupling_always_sends_both_channels() {
    let (mut dev, shared) = device();
    dev.set_coupling(Channel::Ch1, Coupling::Ac).expect("ch1 ac");
    dev.set_coupling(Channel::Ch2, Coupling::Ac).expect("ch2 ac");
    dev.set_coupling(Channel::Ch1, Coupling::Dc).expect("ch1 dc");
    assert_eq!(
        *shared.log.borrow(),
        vec![
            out(req::COUPLING, &[0x10]),
            out(req::COUPLING, &[0x00]),
            out(req::COUPLING, &[0x01]),
        ]
    );
    assert_eq!(dev.config().coupling, [Coupling::Dc, Coupling::Ac]);
}

#[test]
fn failed_coupling_keeps_config() {
    let (mut dev, shared) = device();
    shared.fail.set(Some(req::COUPLING));
    assert!(dev.set_coupling(Channel::Ch2, Coupling::Ac).is_err());
    assert_eq!(dev.config().coupling, [Coupling::Dc, Coupling::Dc]);
}

#[test]
fn calibration_frequency_codes() {
    let cases = [
        (1000, 1),
        (20_000, 20),
        (100_000, 100),
        (32, 103),
        (50, 105),
        (100, 110),
        (1600, 216),
        (5500, 255),
    ];
    for (hz, code) in cases {
        let (mut dev, shared) = device();
        dev.set_calibration_frequency(hz)
            .unwrap_or_else(|e| panic!("{hz} Hz: {e}"));
        assert_eq!(
            shared.log.borrow().last(),
            Some(&out(req::CAL_FREQ, &[code])),
            "{hz} Hz"
        );
    }
}

#[test]
fn calibration_frequency_rejects_out_of_range_without_sending() {
    for hz in [0, 31, 1050, 100_001] {
        let (mut dev, shared) = device();
        assert!(
            matches!(
                dev.set_calibration_frequency(hz),
                Err(Error::InvalidArgument(_))
            ),
            "{hz} Hz"
        );
        assert!(shared.log.borrow().is_empty(), "{hz} Hz");
    }
}

#[test]
fn start_stream_opens_bulk_before_start_command() {
    let (mut dev, shared) = device();
    let _stream = dev.start_stream(StreamConfig::default()).expect("start_stream");
    assert_eq!(
        *shared.log.borrow(),
        vec![
            Event::OpenBulk {
                endpoint: BULK_EP_IN,
                transfer_size: 256 * 1024,
                queue_depth: 8,
            },
            out(req::START_STOP, &[1]),
        ]
    );
}

#[test]
fn stop_sends_stop_command() {
    let (mut dev, shared) = device();
    dev.stop().expect("stop");
    assert_eq!(*shared.log.borrow(), vec![out(req::START_STOP, &[0])]);
}

#[test]
fn stream_config_default_values() {
    let config = StreamConfig::default();
    assert_eq!(config.transfer_size, 256 * 1024);
    assert_eq!(config.queue_depth, 8);
    assert_eq!(config.timeout, Duration::from_secs(1));
}

#[test]
fn stream_config_for_rate_lasts_about_40_ms_in_whole_packets() {
    // 100 kS/s, due canali: 200 kB/s × 40 ms = 8000 byte → 16 pacchetti.
    let config = StreamConfig::for_rate(SampleRate(110), ChannelMode::Dual);
    assert_eq!(config.transfer_size, 16 * 512);
    // 20 kS/s, un canale: 800 byte → 2 pacchetti.
    let config = StreamConfig::for_rate(SampleRate(102), ChannelMode::Single);
    assert_eq!(config.transfer_size, 2 * 512);
    // 48 MS/s: limitato a 256 KB.
    let config = StreamConfig::for_rate(SampleRate(48), ChannelMode::Single);
    assert_eq!(config.transfer_size, 256 * 1024);
    assert_eq!(config.queue_depth, StreamConfig::default().queue_depth);
    assert_eq!(config.timeout, StreamConfig::default().timeout);
}

#[test]
fn megabytes_per_second_is_zero_without_elapsed_time() {
    let stats = StreamStats {
        bytes: 1_000_000,
        transfers: 1,
        segments: 0,
        elapsed: Duration::ZERO,
    };
    assert_eq!(stats.megabytes_per_second(), 0.0);
}

#[test]
fn megabytes_per_second_divides_by_decimal_megabytes() {
    let stats = StreamStats {
        bytes: 2_000_000,
        transfers: 1,
        segments: 0,
        elapsed: Duration::from_secs(2),
    };
    assert_eq!(stats.megabytes_per_second(), 1.0);
}

#[test]
fn dual_mode_splits_interleaved_bytes_with_carry() {
    let (mut dev, shared) = device();
    // A 48 MS/s l'inizio dello stream non ha campioni da scartare.
    dev.set_sample_rate(SampleRate(48)).expect("set_sample_rate");
    shared.chunks.borrow_mut().push_back(vec![1, 2, 3]);
    shared.chunks.borrow_mut().push_back(vec![4, 5, 6]);
    let mut stream = dev.start_stream(StreamConfig::default()).expect("start_stream");
    let mut block = Block::default();

    stream.next_block(&mut block).expect("first block");
    assert_eq!(block.first_sample, 0);
    assert_eq!(block.ch1, vec![1]);
    assert_eq!(block.ch2, vec![2]);

    stream.next_block(&mut block).expect("second block");
    assert_eq!(block.first_sample, 1);
    assert_eq!(block.ch1, vec![3, 5]);
    assert_eq!(block.ch2, vec![4, 6]);

    let stats = stream.stats();
    // I byte contano anche il marcatore di avvio, che passa sull'USB.
    assert_eq!(stats.bytes, 512 + 6);
    assert_eq!(stats.transfers, 2);
}

#[test]
fn single_mode_puts_all_bytes_in_ch1() {
    let (mut dev, shared) = device();
    dev.set_sample_rate(SampleRate(48)).expect("set_sample_rate");
    dev.set_channel_mode(ChannelMode::Single).expect("set_channel_mode");
    shared.chunks.borrow_mut().push_back(vec![1, 2, 3]);
    shared.chunks.borrow_mut().push_back(vec![4, 5]);
    let mut stream = dev.start_stream(StreamConfig::default()).expect("start_stream");
    assert_eq!(stream.mode(), ChannelMode::Single);
    let mut block = Block::default();

    stream.next_block(&mut block).expect("first block");
    assert_eq!(block.first_sample, 0);
    assert_eq!(block.ch1, vec![1, 2, 3]);
    assert!(block.ch2.is_empty());

    stream.next_block(&mut block).expect("second block");
    assert_eq!(block.first_sample, 3);
    assert_eq!(block.ch1, vec![4, 5]);
    assert!(block.ch2.is_empty());
}

#[test]
fn next_block_reports_timeout_when_no_transfer_arrives() {
    let (mut dev, _shared) = device();
    let mut stream = dev.start_stream(StreamConfig::default()).expect("start_stream");
    let mut block = Block::default();
    assert!(matches!(
        stream.next_block(&mut block),
        Err(Error::Timeout)
    ));
}

#[test]
fn old_firmware_has_no_info_and_no_overflow_counter() {
    let (dev, shared) = device_with_version(0x0210);
    assert_eq!(dev.firmware_info().expect("firmware_info"), None);
    assert_eq!(dev.overflow_count().expect("overflow_count"), None);
    assert!(shared.log.borrow().is_empty());
}

#[test]
fn firmware_info_and_overflow_count_parse_little_endian_replies() {
    let (dev, shared) = device_with_version(FIRMWARE_VERSION);
    // Il trasporto simulato risponde con byte 0x80.
    assert_eq!(
        dev.firmware_info().expect("firmware_info"),
        Some(FirmwareInfo {
            version: 0x8080,
            capabilities: 0x80,
        })
    );
    assert_eq!(dev.overflow_count().expect("overflow_count"), Some(0x8080));
    assert_eq!(
        *shared.log.borrow(),
        vec![
            Event::ControlIn {
                request: req::INFO,
                value: 0,
                index: 0,
                len: 4,
            },
            Event::ControlIn {
                request: req::OVERFLOWS,
                value: 0,
                index: 0,
                len: 2,
            },
        ]
    );
}

/// Pacchetto marcatore del firmware con numero di sequenza.
fn marker(seq: u16) -> Vec<u8> {
    let mut packet = vec![0u8; 512];
    packet[..8].copy_from_slice(b"H6022SEG");
    packet[8..10].copy_from_slice(&seq.to_le_bytes());
    packet
}

/// Pacchetto di dati: coppie CH1, CH2 con CH1 = indice della coppia, CH2 = 255 - indice.
fn data_packet() -> Vec<u8> {
    (0..256u16).flat_map(|i| [i as u8, 255 - i as u8]).collect()
}

#[test]
fn marker_packet_becomes_a_break_at_48_msps() {
    let (mut dev, shared) = device();
    dev.set_sample_rate(SampleRate(48)).expect("set_sample_rate");
    let transfer = [data_packet(), marker(1), data_packet()].concat();
    shared.chunks.borrow_mut().push_back(transfer);
    let mut stream = dev.start_stream(StreamConfig::default()).expect("start_stream");
    let mut block = Block::default();

    stream.next_block(&mut block).expect("block");
    assert_eq!(block.ch1.len(), 512);
    assert_eq!(block.ch2.len(), 512);
    assert_eq!(block.breaks, vec![256]);
    assert_eq!(block.ch1[256], 0);
    assert_eq!(block.ch2[256], 255);
    assert_eq!(stream.stats().segments, 1);
}

#[test]
fn segment_start_drops_stale_samples_at_24_msps_and_below() {
    let (mut dev, shared) = device();
    dev.set_sample_rate(SampleRate(24)).expect("set_sample_rate");
    // Il marcatore chiude il primo trasferimento: lo scarto continua nel successivo.
    shared.chunks.borrow_mut().push_back([data_packet(), marker(1)].concat());
    shared.chunks.borrow_mut().push_back(data_packet());
    let mut stream = dev.start_stream(StreamConfig::default()).expect("start_stream");
    let mut block = Block::default();

    // Anche l'inizio dello stream è un inizio di segmento.
    stream.next_block(&mut block).expect("first block");
    assert_eq!(block.ch1.len(), 251);
    assert_eq!((block.ch1[0], block.ch2[0]), (5, 250));
    assert_eq!(block.breaks, vec![251]);

    stream.next_block(&mut block).expect("second block");
    assert!(block.breaks.is_empty());
    assert_eq!(block.ch1.len(), 251);
    assert_eq!((block.ch1[0], block.ch2[0]), (5, 250));
    assert_eq!(block.first_sample, 251);
}

#[test]
fn consecutive_markers_give_a_single_break() {
    let (mut dev, shared) = device();
    dev.set_sample_rate(SampleRate(48)).expect("set_sample_rate");
    let transfer = [data_packet(), marker(1), marker(2), data_packet()].concat();
    shared.chunks.borrow_mut().push_back(transfer);
    let mut stream = dev.start_stream(StreamConfig::default()).expect("start_stream");
    let mut block = Block::default();

    stream.next_block(&mut block).expect("block");
    assert_eq!(block.breaks, vec![256]);
    assert_eq!(block.ch1.len(), 512);
    assert_eq!(stream.stats().segments, 2);
}

#[test]
fn data_before_the_start_marker_is_dropped() {
    let (mut dev, shared) = device();
    dev.set_sample_rate(SampleRate(48)).expect("set_sample_rate");
    shared.manual_start_marker.set(true);
    // Un trasferimento rimasto in coda: pacchetti vecchi, poi il marcatore di avvio.
    let stale = vec![0xaa; 2048];
    shared
        .chunks
        .borrow_mut()
        .push_back([stale, marker(0), data_packet()].concat());
    let mut stream = dev.start_stream(StreamConfig::default()).expect("start_stream");
    let mut block = Block::default();

    stream.next_block(&mut block).expect("block");
    assert_eq!(block.ch1.len(), 256);
    assert_eq!((block.ch1[0], block.ch2[0]), (0, 255));
    assert!(block.breaks.is_empty());
    assert_eq!(stream.stats().segments, 0);
    assert_eq!(block.first_sample, 0);
}

#[test]
fn transfers_before_the_start_marker_give_empty_blocks() {
    let (mut dev, shared) = device();
    dev.set_sample_rate(SampleRate(48)).expect("set_sample_rate");
    shared.manual_start_marker.set(true);
    shared.chunks.borrow_mut().push_back(vec![0xaa; 1024]);
    shared.chunks.borrow_mut().push_back([marker(0), data_packet()].concat());
    let mut stream = dev.start_stream(StreamConfig::default()).expect("start_stream");
    let mut block = Block::default();

    stream.next_block(&mut block).expect("stale block");
    assert!(block.ch1.is_empty());
    stream.next_block(&mut block).expect("block");
    assert_eq!(block.ch1.len(), 256);
    assert_eq!(block.first_sample, 0);
}
