#!/usr/bin/env python3
"""Synthesize Steps's study sounds (chunk 20; PARITY.md 2.1).

Nothing is sampled or downloaded, so there is no licence to track. Two cues,
three variations each:

- chime-N: a soft, short rising chime for a right answer.
- calm-N: a calm tone when a session ends (a struck bowl, not a fanfare).

Renders 48 kHz mono, normalizes to about -18 LUFS (measured with ffmpeg's
EBU R128 filter; a little quieter than Batna's, for long study sessions) and
writes 16-bit WAV to assets/sounds/, which every web view plays.
Adapted from batna-app main: scripts/assets/synth_sounds.py.

    pip install numpy      # and ffmpeg on PATH
    python3 scripts/assets/synth_sounds.py
"""
from __future__ import annotations

import re
import subprocess
import wave
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "assets" / "sounds"
RATE = 48_000
TARGET_LUFS = -18.0


def envelope(n: int, attack: float, decay: float) -> np.ndarray:
    t = np.arange(n) / RATE
    rise = np.clip(t / attack, 0, 1) ** 2
    return rise * np.exp(-t / decay)


def fade_out(x: np.ndarray, seconds: float) -> np.ndarray:
    k = int(seconds * RATE)
    x = x.copy()
    x[-k:] *= np.linspace(1, 0, k)
    return x


def chime(notes: list[float], gap: float, partials: list[tuple[float, float, float]], tail: float, seed: int) -> np.ndarray:
    """Quick rising struck notes with a soft attack; the last one rings longer."""
    rng = np.random.default_rng(seed)
    n = int((gap * (len(notes) - 1) + tail) * RATE)
    t = np.arange(n) / RATE
    out = np.zeros(n)
    for i, f0 in enumerate(notes):
        start = int(i * gap * RATE)
        m = n - start
        tt = t[:m]
        life = tail if i == len(notes) - 1 else gap * 2.2
        note = np.zeros(m)
        for ratio, amp, decay in partials:
            note += amp * np.sin(2 * np.pi * f0 * ratio * tt + rng.uniform(0, 2 * np.pi)) * envelope(m, 0.004, life * decay)
        out[start:] += note * (0.75 if i < len(notes) - 1 else 1.0)
    return fade_out(out, 0.06)


def bowl(base: float, length: float, partials: list[tuple[float, float, float]], beat: float, seed: int) -> np.ndarray:
    """A struck singing bowl: near-harmonic partials, each a pair of slightly
    detuned sines (the slow beating that makes a bowl sound calm), a soft
    mallet attack and a long, even decay."""
    rng = np.random.default_rng(seed)
    n = int(length * RATE)
    t = np.arange(n) / RATE
    out = np.zeros(n)
    for ratio, amp, life in partials:
        f = base * ratio
        for detune in (-beat / 2, beat / 2):
            out += 0.5 * amp * np.sin(2 * np.pi * (f + detune) * t + rng.uniform(0, 2 * np.pi)) * envelope(n, 0.02, length * 0.3 * life)
    return fade_out(out, 0.4)


def write_wav(path: Path, x: np.ndarray) -> None:
    pcm = (np.clip(x, -1, 1) * 32767).astype("<i2")
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(pcm.tobytes())


def loudness(x: np.ndarray, tmp: Path) -> float:
    """Integrated loudness in LUFS; short sounds are padded so the meter can gate."""
    padded = np.concatenate([x, np.zeros(max(0, RATE - len(x)))])
    write_wav(tmp, padded * 0.5)
    res = subprocess.run(["ffmpeg", "-hide_banner", "-nostats", "-i", str(tmp), "-af", "ebur128", "-f", "null", "-"], capture_output=True, text=True)
    value = float(re.findall(r"I:\s+(-?[\d.]+) LUFS", res.stderr)[-1])
    return value - 20 * np.log10(0.5)


def finish(name: str, x: np.ndarray) -> None:
    x = x / np.max(np.abs(x))
    tmp = DEST / f".{name}.tmp.wav"
    x = x * 10 ** ((TARGET_LUFS - loudness(x, tmp)) / 20)
    peak = np.max(np.abs(x))
    if peak > 0.89:  # keep 1 dB of headroom even if that leaves it a little quieter
        x *= 0.89 / peak
    wav = DEST / f"{name}.wav"
    write_wav(wav, x)
    print(f"{wav.relative_to(ROOT)}: {len(x) / RATE:.2f} s, {loudness(x, tmp):.1f} LUFS")
    tmp.unlink(missing_ok=True)


BELL = [(1, 1, 0.35), (2, 0.22, 0.2), (3, 0.06, 0.12)]
VARIANTS = {
    # right answer: short, soft, rising
    "chime-1": lambda: chime([660, 880], 0.09, BELL, 0.45, 1),  # two notes, a fourth up
    "chime-2": lambda: chime([784, 1175], 0.08, [(1, 1, 0.4), (2.76, 0.22, 0.16), (5.4, 0.06, 0.08)], 0.55, 2),  # glassy, a fifth up
    "chime-3": lambda: chime([523, 659, 784], 0.07, BELL, 0.5, 3),  # a quick major triad
    # end of session: one calm struck tone
    "calm-1": lambda: bowl(220, 2.8, [(1, 1, 1), (2.71, 0.35, 0.6), (5.1, 0.12, 0.35)], 1.2, 1),  # warm bowl
    "calm-2": lambda: bowl(174, 3.2, [(1, 1, 1), (2.76, 0.25, 0.55), (5.4, 0.08, 0.3)], 0.8, 2),  # deeper, slower beat
    "calm-3": lambda: bowl(262, 2.4, [(1, 1, 1), (2, 0.3, 0.6), (3, 0.12, 0.4), (4.2, 0.05, 0.25)], 1.6, 3),  # brighter, rounder
}


def main() -> None:
    DEST.mkdir(parents=True, exist_ok=True)
    for name, make in VARIANTS.items():
        finish(name, make())


if __name__ == "__main__":
    main()
