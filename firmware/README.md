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
