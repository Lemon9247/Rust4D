#!/usr/bin/env python3
"""Generate the two tiny CC0 WAV cues for the Rust4D Lua demo.

Run once to (re)create the committed assets:

    python3 games/demo/assets/generate_cues.py

Produces:
  * blip.wav  — a short 880 Hz sine "enter" cue with a fast exponential decay
  * tone.wav  — a low 220 Hz sine "exit" cue with a soft envelope

Both are 22050 Hz mono, ~0.4 s, well under 50 KB, public-domain (synthesized
from a pure math function — no third-party audio). Uses only the Python stdlib
(`wave`, `struct`, `math`) so it runs anywhere Python 3 does.
"""

import math
import struct
import wave
from pathlib import Path

RATE = 22050      # Hz, mono
AMPLITUDE = 0.6   # 0..1

def write_wav(path: Path, samples: list[float]) -> None:
    with wave.open(str(path), "w") as w:
        w.setnchannels(1)
        w.setsampwidth(2)  # 16-bit
        w.setframerate(RATE)
        frames = b"".join(
            struct.pack("<h", int(max(-1.0, min(1.0, s)) * 32767)) for s in samples
        )
        w.writeframes(frames)

def blip() -> list[float]:
    """880 Hz sine, 0.35 s, fast exponential decay (bright 'enter' ping)."""
    dur = 0.35
    n = int(RATE * dur)
    freq = 880.0
    out = []
    for i in range(n):
        t = i / RATE
        env = math.exp(-t * 9.0)          # quick decay
        out.append(AMPLITUDE * env * math.sin(2.0 * math.pi * freq * t))
    return out

def tone() -> list[float]:
    """220 Hz sine, 0.45 s, soft attack + release (mellow 'exit' tone)."""
    dur = 0.45
    n = int(RATE * dur)
    freq = 220.0
    attack = 0.02
    release = 0.18
    out = []
    for i in range(n):
        t = i / RATE
        if t < attack:
            env = t / attack
        elif t > dur - release:
            env = max(0.0, (dur - t) / release)
        else:
            env = 1.0
        # Slight decay through the sustain for warmth.
        env *= math.exp(-t * 2.5)
        out.append(AMPLITUDE * 0.8 * env * math.sin(2.0 * math.pi * freq * t))
    return out

def main() -> None:
    out_dir = Path(__file__).resolve().parent
    out_dir.mkdir(parents=True, exist_ok=True)
    write_wav(out_dir / "blip.wav", blip())
    write_wav(out_dir / "tone.wav", tone())
    for name in ("blip.wav", "tone.wav"):
        p = out_dir / name
        print(f"wrote {p} ({p.stat().st_size} bytes)")

if __name__ == "__main__":
    main()