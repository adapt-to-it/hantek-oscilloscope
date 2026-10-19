# Firmware

This folder contains the Cypress FX2LP firmware that the host loads into the Hantek 6022BE at
every connection.

- `dso6022be/` derives from the `Firmware/DSO6022BE` folder of
  [Ho-Ro/Hantek6022API](https://github.com/Ho-Ro/Hantek6022API). `UPSTREAM_COMMIT` records the
  upstream commit. Copyright (C) 2009 Ubixum, Inc., (C) 2015 Jochen Hoenicke, and later
  contributors.
- `fx2lib/` is [fx2lib](https://github.com/djmuhlestein/fx2lib), Copyright (C) Ubixum, Inc.

Both are licensed under the GNU Lesser General Public License, version 2.1 or later. See
`fx2lib/COPYING` and `fx2lib/COPYING.LESSER`. The MIT/Apache-2.0 license of the rest of the
repository does not apply to this folder.

The Rust driver embeds `dso6022be/dso6022be-firmware.hex` as a separate binary and sends it to
the device. To rebuild the firmware, install SDCC and run `make` in `dso6022be/`.

## Changes from upstream

- Version 3.00 (`bcdDevice` 0x0300).
- Request 0xe7 (IN, 4 bytes) returns the version and the capability bits.
- Request 0xe8 (IN, 2 bytes) returns how many times the endpoint FIFO became full since
  sampling started. A full FIFO means lost samples. Requests 0xe7 and 0xe8 do not stop sampling.
- The 640 kS/s waveform (id 164) now runs the GPIF at 48 MHz. Upstream ran it at 30 MHz,
  so it sampled at 400 kS/s.
