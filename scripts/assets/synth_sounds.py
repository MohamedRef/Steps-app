#!/usr/bin/env python3
"""Synthesize Steps' study sounds.

Nothing is sampled or downloaded, so there is no licence to track. The cues
follow Steps' picture of studying as a climb, two cues with three variations
each:

- step-N: a right answer is a light footstep up: a short run of rising
  wooden or plucked notes (marimba bar, kalimba tine, a soft swoop up).
- summit-N: the end of a session is reaching the top: a few notes climb and
  open onto one warm, held chord.

Renders 48 kHz mono, normalizes to about -18 LUFS (measured with ffmpeg's
EBU R128 filter; quiet enough for long study sessions) and writes 16-bit WAV
to assets/sounds/, which every web view plays. The loudness pipeline
(`loudness`, `finish`, `write_wav`) is adapted from batna-app main:
scripts/assets/synth_sounds.py; the instruments and the cues are Steps' own.

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


def hz(note: str) -> float:
    """'A4' -> 440.0; sharps as 'F#5'."""
    names = {"C": -9, "D": -7, "E": -5, "F": -4, "G": -2, "A": 0, "B": 2}
    semis = names[note[0]] + (1 if "#" in note else 0)
    octave = int(note[-1])
    return 440.0 * 2 ** ((semis + 12 * (octave - 4)) / 12)


def timeline(seconds: float) -> np.ndarray:
    return np.zeros(int(seconds * RATE))


def place(out: np.ndarray, x: np.ndarray, at: float, gain: float = 1.0) -> None:
    start = max(0, int(at * RATE))
    end = min(len(out), start + len(x))
    out[start:end] += gain * x[: end - start]


def fade_out(x: np.ndarray, seconds: float) -> np.ndarray:
    k = int(seconds * RATE)
    x = x.copy()
    x[-k:] *= np.linspace(1, 0, k) ** 2
    return x


def one_pole_lowpass(x: np.ndarray, cutoff: float) -> np.ndarray:
    a = np.exp(-2 * np.pi * cutoff / RATE)
    y = np.empty_like(x)
    acc = 0.0
    for i, v in enumerate(x):
        acc = (1 - a) * v + a * acc
        y[i] = acc
    return y


def marimba(f0: float, length: float, seed: int) -> np.ndarray:
    """A struck wooden bar: the tuned bar's inharmonic modes (1, 3.93, 9.2),
    the upper ones dying fast, plus a tiny felt-mallet knock."""
    rng = np.random.default_rng(seed)
    t = np.arange(int(length * RATE)) / RATE
    x = np.zeros_like(t)
    for ratio, amp, life in [(1.0, 1.0, 0.28), (3.93, 0.32, 0.05), (9.2, 0.08, 0.015)]:
        x += amp * np.sin(2 * np.pi * f0 * ratio * t + rng.uniform(0, 2 * np.pi)) * np.exp(-t / life)
    knock = one_pole_lowpass(rng.standard_normal(len(t)), 1800) * np.exp(-t / 0.004) * 0.6
    attack = np.clip(t / 0.002, 0, 1)
    return (x + knock) * attack


def kalimba(f0: float, length: float, seed: int) -> np.ndarray:
    """A plucked metal tine (Karplus-Strong with a soft pluck), with the
    tine's bright 'click' partial near 5.4x that fades within a breath."""
    rng = np.random.default_rng(seed)
    n = int(length * RATE)
    period = int(RATE / f0)
    buf = one_pole_lowpass(rng.uniform(-1, 1, period), 3000)
    out = np.empty(n)
    for i in range(n):
        out[i] = buf[i % period]
        buf[i % period] = 0.996 * 0.5 * (buf[i % period] + buf[(i + 1) % period])
    t = np.arange(n) / RATE
    out += 0.25 * np.sin(2 * np.pi * f0 * 5.4 * t) * np.exp(-t / 0.03)
    return out * np.clip(t / 0.003, 0, 1)


def swoop(f_from: float, f_to: float, length: float) -> np.ndarray:
    """A soft rounded tone that glides up a step as it sounds (a quick FM
    pluck): the 'hop' of a right answer."""
    t = np.arange(int(length * RATE)) / RATE
    glide = f_to + (f_from - f_to) * np.exp(-t / 0.025)
    phase = 2 * np.pi * np.cumsum(glide) / RATE
    index = 1.2 * np.exp(-t / 0.05)
    x = np.sin(phase + index * np.sin(2 * phase))
    return x * np.clip(t / 0.004, 0, 1) * np.exp(-t / 0.16)


def pad(freqs: list[float], length: float, attack: float, seed: int) -> np.ndarray:
    """A warm held chord: each note a few gently detuned soft partials with
    a slow vibrato, swelling in and fading long (the view from the top)."""
    rng = np.random.default_rng(seed)
    t = np.arange(int(length * RATE)) / RATE
    x = np.zeros_like(t)
    for f in freqs:
        for detune in (-0.6, 0.0, 0.7):
            vib = 1 + 0.0025 * np.sin(2 * np.pi * rng.uniform(4.2, 5.4) * t)
            ph = 2 * np.pi * np.cumsum((f + detune) * vib) / RATE + rng.uniform(0, 2 * np.pi)
            x += np.sin(ph) + 0.3 * np.sin(2 * ph) + 0.08 * np.sin(3 * ph)
    shape = np.clip(t / attack, 0, 1) ** 1.5 * np.exp(-np.maximum(t - attack, 0) / (length * 0.45))
    return x / (3 * len(freqs)) * shape


def climb(notes: list[str], step: float, voice) -> np.ndarray:
    """Notes rising one after another, each a footstep up."""
    out = timeline(step * (len(notes) - 1) + 0.6)
    for i, n in enumerate(notes):
        place(out, voice(hz(n), 0.6, i), i * step, 0.8 + 0.2 * i / max(1, len(notes) - 1))
    return fade_out(out, 0.08)


def summit(notes: list[str], step: float, voice, chord: list[str], length: float, attack: float, seed: int) -> np.ndarray:
    """A short climb that opens onto a held chord at the top."""
    arrive = step * len(notes)
    out = timeline(arrive + length)
    for i, n in enumerate(notes):
        place(out, voice(hz(n), 0.9, seed * 10 + i), i * step, 0.55)
    place(out, pad([hz(c) for c in chord], length, attack, seed), arrive - attack * 0.6, 1.0)
    return fade_out(out, 0.5)


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


VARIANTS = {
    # right answer: a light footstep up
    "step-1": lambda: climb(["D5", "E5", "A5"], 0.065, marimba),  # wooden steps, pentatonic
    "step-2": lambda: climb(["F5", "C6"], 0.075, kalimba),  # two plucked tines, a fifth up
    "step-3": lambda: swoop(hz("G5") * 0.94, hz("D6"), 0.5),  # one soft hop up
    # end of session: reaching the top
    "summit-1": lambda: summit(["C5", "E5", "G5", "C6"], 0.11, marimba, ["C4", "G4", "D5", "E5"], 2.4, 0.25, 1),
    "summit-2": lambda: summit(["D5", "E5", "A5"], 0.13, kalimba, ["D4", "A4", "E5"], 2.6, 0.3, 2),
    "summit-3": lambda: summit(["F4"], 0.2, marimba, ["F3", "C4", "A4", "E5"], 2.8, 0.7, 3),  # a slow sunrise swell
}


def main() -> None:
    DEST.mkdir(parents=True, exist_ok=True)
    for name, make in VARIANTS.items():
        finish(name, make())


if __name__ == "__main__":
    main()
