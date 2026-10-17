"""Test di h6022.decode_frame e time_axis. Nessuna connessione di rete."""

from __future__ import annotations

import struct
import unittest

import numpy as np

from h6022 import MAGIC, decode_frame, time_axis


def build_frame(
    ch1: list[float],
    ch2: list[float] | None = None,
    seq: int = 7,
    sample_rate: float = 1000.0,
    t0: float = 0.0,
    triggered: int = 1,
) -> bytes:
    """Costruisce un messaggio binario secondo il formato di ws.rs (little-endian)."""
    n = len(ch1)
    channels = 1 if ch2 is None else 2
    header = struct.pack("<8sQddIBB2x", MAGIC, seq, sample_rate, t0, n, channels, triggered)
    body = struct.pack(f"<{n}f", *ch1)
    if ch2 is not None:
        body += struct.pack(f"<{n}f", *ch2)
    return header + body


class DecodeFrameTest(unittest.TestCase):
    def test_un_canale(self) -> None:
        data = build_frame([0.5, -1.25, 2.0, 0.0], seq=42, sample_rate=2000.0, t0=-0.001, triggered=0)

        frame = decode_frame(data)

        self.assertEqual(frame.sequence, 42)
        self.assertEqual(frame.sample_rate_hz, 2000.0)
        self.assertEqual(frame.t0_s, -0.001)
        self.assertFalse(frame.triggered)
        self.assertEqual(frame.ch1.dtype, np.float32)
        np.testing.assert_array_equal(frame.ch1, np.array([0.5, -1.25, 2.0, 0.0], dtype=np.float32))
        self.assertIsNone(frame.ch2)

    def test_due_canali(self) -> None:
        data = build_frame([1.0, 2.0, 3.0], [-1.0, -2.0, -3.0], triggered=1)

        frame = decode_frame(data)

        self.assertTrue(frame.triggered)
        np.testing.assert_array_equal(frame.ch1, np.array([1.0, 2.0, 3.0], dtype=np.float32))
        self.assertIsNotNone(frame.ch2)
        np.testing.assert_array_equal(frame.ch2, np.array([-1.0, -2.0, -3.0], dtype=np.float32))

    def test_magia_errata(self) -> None:
        data = bytearray(build_frame([1.0, 2.0]))
        data[0:8] = b"NOTVALID"

        with self.assertRaises(ValueError):
            decode_frame(bytes(data))

    def test_dati_troncati_nell_intestazione(self) -> None:
        data = build_frame([1.0, 2.0])

        with self.assertRaises(ValueError):
            decode_frame(data[:20])

    def test_dati_troncati_nei_campioni(self) -> None:
        data = build_frame([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])

        with self.assertRaises(ValueError):
            decode_frame(data[:-4])


class TimeAxisTest(unittest.TestCase):
    def test_asse_in_secondi(self) -> None:
        frame = decode_frame(build_frame([0.0, 0.0, 0.0, 0.0], sample_rate=1000.0, t0=0.001))

        axis = time_axis(frame)

        np.testing.assert_allclose(axis, [0.001, 0.002, 0.003, 0.004])

    def test_asse_con_due_canali(self) -> None:
        frame = decode_frame(build_frame([0.0, 0.0], [0.0, 0.0], sample_rate=500.0, t0=0.0))

        axis = time_axis(frame)

        self.assertEqual(axis.shape, (2,))
        np.testing.assert_allclose(axis, [0.0, 0.002])


if __name__ == "__main__":
    unittest.main()
