"""Client Python per il WebSocket binario `/ws` di h6022-server.

Ogni messaggio binario contiene un frame: intestazione little-endian di 40 byte
seguita dai campioni f32 di CH1 e, se presenti, di CH2.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from typing import Iterator
from urllib.parse import urlsplit, urlunsplit, urlencode

import numpy as np

MAGIC = b"H6022WS1"
# magia, sequence, sample_rate_hz, t0_s, campioni per canale, canali, triggered, riservati
HEADER_FMT = "<8sQddIBB2x"
HEADER_LEN = struct.calcsize(HEADER_FMT)  # 40


@dataclass
class Frame:
    sequence: int
    sample_rate_hz: float
    t0_s: float
    triggered: bool
    ch1: np.ndarray
    ch2: np.ndarray | None


def decode_frame(data: bytes) -> Frame:
    """Decodifica un messaggio binario del server. Solleva ValueError se non valido."""
    if len(data) < HEADER_LEN:
        raise ValueError(f"frame troncato: {len(data)} byte, intestazione di {HEADER_LEN}")

    magic, seq, sample_rate, t0, n, channels, triggered = struct.unpack_from(HEADER_FMT, data)
    if magic != MAGIC:
        raise ValueError(f"magia non valida: {magic!r}")
    if channels not in (1, 2):
        raise ValueError(f"numero di canali non valido: {channels}")
    if triggered not in (0, 1):
        raise ValueError(f"campo triggered non valido: {triggered}")

    expected = HEADER_LEN + n * channels * 4
    if len(data) != expected:
        raise ValueError(f"lunghezza errata: {len(data)} byte, attesi {expected}")

    samples = np.frombuffer(data, dtype="<f4", count=n * channels, offset=HEADER_LEN).astype(np.float32)
    ch1 = samples[:n]
    ch2 = samples[n:] if channels == 2 else None

    return Frame(
        sequence=seq,
        sample_rate_hz=sample_rate,
        t0_s=t0,
        triggered=bool(triggered),
        ch1=ch1,
        ch2=ch2,
    )


def time_axis(frame: Frame) -> np.ndarray:
    """Asse dei tempi in secondi, uno per campione di CH1."""
    n = frame.ch1.shape[0]
    return frame.t0_s + np.arange(n, dtype=np.float64) / frame.sample_rate_hz


def frames(
    url: str = "ws://127.0.0.1:6022/ws",
    max_fps: int = 30,
    count: int | None = None,
) -> Iterator[Frame]:
    """Riceve frame dal server. Si ferma dopo `count` frame, se indicato."""
    from websockets.sync.client import connect

    if not 1 <= max_fps <= 120:
        raise ValueError(f"max_fps fuori intervallo 1..120: {max_fps}")
    if count is not None and count <= 0:
        return

    parts = urlsplit(url)
    sep = "&" if parts.query else ""
    query = parts.query + sep + urlencode({"max_fps": max_fps})
    full_url = urlunsplit(parts._replace(query=query))

    received = 0
    with connect(full_url) as ws:
        while count is None or received < count:
            message = ws.recv()
            if isinstance(message, str):
                raise ValueError("messaggio testuale inatteso dal server")
            yield decode_frame(message)
            received += 1
