//! Diagnostica: invia i comandi uno alla volta e stampa l'esito.
use h6022_usb::nusb_transport::NusbTransport;
use h6022_usb::{Transport, req, *};

fn main() {
    let t = NusbTransport::open(VID_FIRMWARE, PID_6022BE).expect("open");
    println!("id {:?}", t.id());
    for len in [8u16, 32, 64, 80] {
        println!("eeprom {len}: {:?}", t.control_in(req::EEPROM, CALIBRATION_EEPROM_OFFSET, 0, len).map(|v| v.len()));
    }
    for (name, r, d) in [("rate", req::SAMPLE_RATE, 1u8), ("nch", req::NUM_CHANNELS, 2), ("ch1", req::CH1_RANGE, 1), ("ch2", req::CH2_RANGE, 1), ("cpl", req::COUPLING, 0x11), ("cal", req::CAL_FREQ, 1)] {
        println!("{name}: {:?}", t.control_out(r, 0, 0, &[d]));
    }
}
