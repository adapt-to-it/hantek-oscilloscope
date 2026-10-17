"""Legge un frame dal server h6022 e lo disegna con matplotlib.

Uso: python example_plot.py [url]
"""

from __future__ import annotations

import sys

from h6022 import frames, time_axis

DEFAULT_URL = "ws://127.0.0.1:6022/ws"


def main() -> None:
    import matplotlib.pyplot as plt

    url = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_URL
    stream = frames(url=url, count=1)
    try:
        frame = next(stream)
    finally:
        stream.close()

    t_ms = time_axis(frame) * 1e3
    fig, ax = plt.subplots()
    ax.plot(t_ms, frame.ch1, label="CH1")
    if frame.ch2 is not None:
        ax.plot(t_ms, frame.ch2, label="CH2")
    ax.set_xlabel("tempo [ms]")
    ax.set_ylabel("tensione [V]")
    trigger = "trigger" if frame.triggered else "senza trigger"
    ax.set_title(f"frame {frame.sequence} ({trigger}, {frame.sample_rate_hz:.0f} Sa/s)")
    ax.grid(True)
    ax.legend()
    plt.show()


if __name__ == "__main__":
    main()
