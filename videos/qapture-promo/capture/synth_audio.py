"""Sintetiza la música de fondo y los efectos del video (determinista, sin red).

Uso: python synth_audio.py <carpeta_salida>
"""
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import lfilter

SR = 44100
OUT = Path(sys.argv[1])
OUT.mkdir(parents=True, exist_ok=True)
rng = np.random.default_rng(7)

BPM = 100
BEAT = 60 / BPM
LENGTH = 53.0


def t_axis(seconds):
    return np.arange(int(seconds * SR)) / SR


def midi(n):
    return 440.0 * 2 ** ((n - 69) / 12)


def lowpass(x, cutoff):
    # Un polo, suficiente para suavizar.
    a = np.exp(-2 * np.pi * cutoff / SR)
    return lfilter([1 - a], [1, -a], x)


def env_adsr(n, a, d, s, r):
    e = np.ones(n) * s
    na, nd, nr = int(a * SR), int(d * SR), int(r * SR)
    e[:na] = np.linspace(0, 1, na, endpoint=False)
    e[na:na + nd] = np.linspace(1, s, nd, endpoint=False)
    e[-nr:] *= np.linspace(1, 0, nr)
    return e


def place(buf, sig, at):
    i = int(at * SR)
    j = min(len(buf), i + len(sig))
    buf[i:j] += sig[: j - i]


def pad_chord(notes, seconds):
    t = t_axis(seconds)
    sig = np.zeros_like(t)
    for n in notes:
        for det in (-0.08, 0.0, 0.08):
            f = midi(n + det)
            # Diente de sierra suave por síntesis aditiva (pocas armónicas).
            for k in range(1, 7):
                sig += np.sin(2 * np.pi * f * k * t + k) / (k * 1.4)
    sig /= np.max(np.abs(sig)) + 1e-9
    return sig * env_adsr(len(t), 0.9, 0.5, 0.8, 1.2)


def pluck(n, seconds=0.6):
    t = t_axis(seconds)
    f = midi(n)
    sig = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(2 * np.pi * 2 * f * t) + 0.12 * np.sin(2 * np.pi * 3 * f * t)
    return sig * np.exp(-t * 7)


def kick():
    t = t_axis(0.45)
    f = 50 + 90 * np.exp(-t * 28)
    ph = 2 * np.pi * np.cumsum(f) / SR
    return np.sin(ph) * np.exp(-t * 9)


def hat():
    t = t_axis(0.08)
    n = rng.standard_normal(len(t))
    n = n - lowpass(n, 6000)
    return n * np.exp(-t * 60) * 0.5


# --- Música -----------------------------------------------------------------
music = np.zeros(int(LENGTH * SR))
# Progresión luminosa: Fmaj7 – G – Am7 – Cmaj7 (2 compases cada uno = 8 tiempos)
prog = [[53, 57, 60, 64], [55, 59, 62, 67], [57, 60, 64, 67], [48, 55, 59, 64]]
bar = 4 * BEAT
chord_len = 2 * bar
t0 = 0.0
k = 0
while t0 < LENGTH:
    notes = prog[k % len(prog)]
    place(music, 0.16 * lowpass(pad_chord(notes, chord_len + 1.0), 1800), t0)
    # Bajo
    place(music, 0.22 * lowpass(pad_chord([notes[0] - 12], chord_len + 0.5), 400), t0)
    # Arpegio a corcheas desde que aparece la marca (~7.5 s)
    steps = int(chord_len / (BEAT / 2))
    arp = notes + [notes[1] + 12, notes[2] + 12]
    for s in range(steps):
        at = t0 + s * BEAT / 2
        if at >= 7.4 and at < LENGTH - 2.5:
            place(music, 0.07 * pluck(arp[(s * 3) % len(arp)] + 12), at)
    t0 += chord_len
    k += 1

# Pulso: bombo suave y hi-hat desde la marca hasta antes del cierre
b = 7.4
while b < LENGTH - 4:
    place(music, 0.30 * kick(), b)
    place(music, 0.10 * hat(), b + BEAT / 2)
    b += BEAT

# Fade in / out
fi, fo = int(1.5 * SR), int(3.0 * SR)
music[:fi] *= np.linspace(0, 1, fi)
music[-fo:] *= np.linspace(1, 0, fo)
music = lowpass(music, 9000)
music /= np.max(np.abs(music)) + 1e-9
music *= 0.8
stereo = np.stack([music, np.roll(music, 220)], axis=1)
sf.write(OUT / "music.wav", stereo, SR)


# --- Efectos ----------------------------------------------------------------
def save(name, sig, peak=0.8):
    sig = sig / (np.max(np.abs(sig)) + 1e-9) * peak
    sf.write(OUT / f"{name}.wav", sig, SR)


def whoosh(seconds=0.7):
    t = t_axis(seconds)
    n = rng.standard_normal(len(t))
    # Barrido de filtro: mezcla progresiva de dos pasabajos.
    lo, hi = lowpass(n, 500), lowpass(n, 4000)
    mix = np.clip(t / seconds, 0, 1)
    sig = lo * (1 - mix) + hi * mix
    shape = np.sin(np.pi * np.clip(t / seconds, 0, 1)) ** 2
    return sig * shape


def click():
    t = t_axis(0.05)
    return (np.sin(2 * np.pi * 2400 * t) * 0.6 + rng.standard_normal(len(t)) * 0.4) * np.exp(-t * 140)


def shutter():
    t = t_axis(0.25)
    a = rng.standard_normal(len(t)) * np.exp(-t * 60)
    b = np.zeros_like(t)
    i = int(0.09 * SR)
    b[i:] = rng.standard_normal(len(t) - i) * np.exp(-(t[i:] - 0.09) * 45) * 0.8
    return lowpass(a + b, 7000)


def impact():
    t = t_axis(1.6)
    boom = np.sin(2 * np.pi * np.cumsum(40 + 60 * np.exp(-t * 6)) / SR) * np.exp(-t * 2.5)
    shimmer = sum(np.sin(2 * np.pi * midi(n) * t) for n in (77, 81, 84, 89)) * np.exp(-t * 2.2) * 0.25
    return boom + shimmer


def riser(seconds=1.6):
    t = t_axis(seconds)
    f = 300 + 1500 * (t / seconds) ** 2
    tone = np.sin(2 * np.pi * np.cumsum(f) / SR) * 0.3
    n = lowpass(rng.standard_normal(len(t)), 3000)
    return (tone + n) * (t / seconds) ** 2


def pop():
    t = t_axis(0.12)
    f = 900 - 500 * t / 0.12
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t * 35)


save("whoosh", whoosh())
save("click", click(), 0.6)
save("shutter", shutter())
save("impact", impact())
save("riser", riser())
save("pop", pop(), 0.6)
print("ok")
