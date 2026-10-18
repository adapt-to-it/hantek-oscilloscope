# Hantek Oscilloscope

Open-source software for Hantek USB oscilloscopes. It currently supports the Hantek 6022BE.
It runs on macOS, and Windows and Linux builds are in progress.

It is a modern alternative to the original Hantek software and to OpenHantek. A single Rust
engine drives the device. A desktop app, a command-line tool, an MCP server for AI agents and a
Python API all use that engine.

![Time-domain view](docs/screenshots/tauri-tempo.png)

## Features

- **Acquisition**: two channels, 8 bit, up to 48 MS/s. Continuous streaming, block capture and
  roll mode for slow time bases.
- **Software trigger**: edge, pulse width, runt, window, slope, timeout, two-channel pattern,
  logical AND/OR and A → B sequence. Auto, normal and single modes.
- **Measurements**: Vpp, Vmax, Vmin, mean, RMS, frequency, period, duty cycle, rise and fall
  time, overshoot, phase between channels, with statistics.
- **Analysis**: cursors, FFT with Hann, Blackman-Harris and flat-top windows, spectrogram, math
  channels, filters, XY mode, color persistence, sin(x)/x interpolation.
- **Protocol decoders**: UART with automatic baud-rate detection, I2C, CAN, 1-Wire, PWM and servo,
  IR (NEC, RC5), Manchester.
- **Pattern recognition**: signal classification, waveform search, packet segmentation.
- **Recording and export**: CSV, NumPy `.npy`, WAV, VCD.
- **Automation**: MCP server over streamable HTTP, binary WebSocket for Python and numpy.
- **Interface** in English, Italian, French and Spanish. Press `i` to show an explanation of
  any control.

| Spectrum | XY mode |
|---|---|
| ![Spectrum](docs/screenshots/tauri-spettro.png) | ![XY](docs/screenshots/tauri-xy.png) |

## Supported devices

| Device | Status |
|---|---|
| Hantek 6022BE | Supported |
| Hantek 6022BL | Planned |

The software loads its own firmware into the device RAM at every connection. The device keeps no
firmware, so unplugging it restores the original state.

## Platforms

| OS | Status |
|---|---|
| macOS (Apple Silicon and Intel) | Tested |
| Windows | In progress |
| Linux | In progress |

The USB layer uses [nusb](https://github.com/kevinmehall/nusb) and does not depend on libusb.

## Build from source

Requirements: Rust 1.88 or later, Node.js 20 or later.

```bash
cd ui && npm install && cd ..
```

To run the desktop app in development mode, from the repository root:

```bash
ui/node_modules/.bin/tauri dev
```

To build the installable package:

```bash
ui/node_modules/.bin/tauri build
```

To build and use the command-line tool:

```bash
cargo run --release -p h6022-cli -- info
```

The CLI offers the `info`, `capture`, `bench`, `eeprom`, `measure` and `serve` commands.

## MCP server

The app and `h6022 serve` start an MCP server on `127.0.0.1`. An AI agent can configure the
oscilloscope, capture, measure, decode and record. The available tools are `device_status`,
`get_config`, `set_config`, `run`, `stop`, `capture`, `snapshot`, `measure`, `spectrum`,
`decode`, `analyze_signal`, `classify_signal`, `find_pattern`, `find_packets`,
`start_recording`, `stop_recording` and `export_logic`.

## Python

`python/h6022.py` receives the samples over WebSocket as numpy arrays. See
`python/example_plot.py`.

## Repository layout

```
crates/
  h6022-usb      firmware loading, vendor commands, EEPROM, bulk streaming
  h6022-core     trigger, time base, auto/normal/single/roll modes, simulator
  h6022-dsp      measurements, FFT, filters, math channels
  h6022-decode   serial decoders, pulse analysis, pattern search
  h6022-record   recording to CSV, NumPy, WAV
  h6022-server   MCP and WebSocket server
  h6022-view     display logic shared by the views
  h6022-cli      command-line tool
  h6022-app      desktop app (Tauri)
ui/              Svelte interface
firmware/        FX2LP firmware (C, SDCC)
python/          Python client
```

## License

The Rust code and the interface are licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option.

The firmware in `firmware/` derives from
[Ho-Ro/Hantek6022API](https://github.com/Ho-Ro/Hantek6022API) and
[fx2lib](https://github.com/djmuhlestein/fx2lib), and keeps its LGPL-2.1-or-later license.
See [firmware/README.md](firmware/README.md).

## Disclaimer

This project is not affiliated with or endorsed by Hantek. Hantek is a trademark of its owner.
