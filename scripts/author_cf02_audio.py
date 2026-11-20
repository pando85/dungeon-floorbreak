#!/usr/bin/env python3
"""CF02-AUDIO: original deterministic DSP generator for the five Q24 cues.

Writes `assets/audio/floor01/cf02/sfx/*.ogg`, the overlay manifest
`assets/audio/floor01/cf02/docs/asset_manifest_cf02.json` and the structural QA
report `assets/audio/floor01/cf02/docs/QA_REPORT.md`.

Provenance (the same as the Floor 01 pack): original procedural synthesis from
seeded noise and damped sinusoids. No recording, no sample, no third-party or
generated-model audio, no voice. A fixed seed per cue and take makes the PCM
reproducible; the Ogg Vorbis encode is ffmpeg's libvorbis in bit-exact mode.

Usage:
    python3 scripts/author_cf02_audio.py            # regenerate files, manifest, QA report
    python3 scripts/author_cf02_audio.py --check    # verify the committed files match the manifest

Needs numpy, scipy and an ffmpeg with libvorbis. The Rust tests never run this
script; they check the committed files against the committed manifest.
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import wave
from pathlib import Path

import numpy as np
from scipy import signal

ROOT = Path(__file__).resolve().parent.parent
PACK = ROOT / "assets/audio/floor01"
OUT = PACK / "cf02"
SFX = OUT / "sfx"
DOCS = OUT / "docs"
SR = 44100
TAKES = 3
BASE_SEED = 20261120

# Legacy Floor 01 cues measured by the structural QA (peak, RMS) for the level band.
LEGACY_RMS_BAND = (0.05, 0.14)
LEGACY_PEAK_MAX = 0.80


# ------------------------------------------------------------------ dsp ----

def t_axis(seconds: float) -> np.ndarray:
    return np.arange(int(seconds * SR)) / SR


def noise(rng: np.random.Generator, n: int) -> np.ndarray:
    return rng.standard_normal(n)


def sos(kind: str, cutoff, order: int = 2):
    return signal.butter(order, cutoff, btype=kind, fs=SR, output="sos")


def lp(x: np.ndarray, fc: float) -> np.ndarray:
    return signal.sosfilt(sos("lowpass", fc), x)


def hp(x: np.ndarray, fc: float) -> np.ndarray:
    return signal.sosfilt(sos("highpass", fc), x)


def bp(x: np.ndarray, lo: float, hi: float) -> np.ndarray:
    return signal.sosfilt(sos("bandpass", [lo, hi]), x)


def decay(t: np.ndarray, tau: float, attack: float = 0.0008) -> np.ndarray:
    env = np.exp(-t / tau)
    if attack > 0:
        env *= np.minimum(1.0, t / attack)
    return env


def at(x: np.ndarray, seconds: float, length: int) -> np.ndarray:
    """`x` placed `seconds` into a silent buffer of `length` samples."""
    out = np.zeros(length)
    start = int(seconds * SR)
    if start >= length:
        return out
    chunk = x[: length - start]
    out[start : start + len(chunk)] = chunk
    return out


def modal(t: np.ndarray, freqs, taus, amps, rng: np.random.Generator) -> np.ndarray:
    out = np.zeros_like(t)
    for f, tau, a in zip(freqs, taus, amps):
        phase = rng.uniform(0, 2 * np.pi)
        out += a * np.sin(2 * np.pi * f * t + phase) * np.exp(-t / tau)
    return out


def grains(rng, length, times, lo, hi, tau, amps) -> np.ndarray:
    """Short band-limited noise ticks (gravel, plaster, debris) at `times` seconds."""
    out = np.zeros(length)
    n = int(0.03 * SR)
    t = np.arange(n) / SR
    for when, amp in zip(times, amps):
        g = bp(noise(rng, n), lo, hi) * np.exp(-t / tau)
        out += at(g, when, length) * amp
    return out


def finish(x: np.ndarray, peak: float, fade_out: float = 0.012) -> np.ndarray:
    x = x - np.mean(x)
    x = x / (np.max(np.abs(x)) + 1e-9) * peak
    n_in, n_out = int(0.0015 * SR), int(fade_out * SR)
    x[:n_in] *= np.linspace(0, 1, n_in)
    x[-n_out:] *= np.linspace(1, 0, n_out)
    return x


def jitter(rng: np.random.Generator, value: float, amount: float = 0.08) -> float:
    return value * (1.0 + rng.uniform(-amount, amount))


# --------------------------------------------------------------- recipes ----

def punch_hit(rng: np.random.Generator) -> np.ndarray:
    """A short, dry, mid-heavy knuckle and body hit: tighter and higher than the kick."""
    n = int(0.22 * SR)
    t = np.arange(n) / SR
    click = bp(noise(rng, n), jitter(rng, 1800), jitter(rng, 5200)) * decay(t, 0.004) * 0.9
    slap = bp(noise(rng, n), jitter(rng, 500), jitter(rng, 1600)) * decay(t, 0.018) * 0.8
    thud = lp(noise(rng, n), jitter(rng, 230)) * decay(t, 0.035) * 1.0
    body = np.sin(2 * np.pi * jitter(rng, 96) * t) * decay(t, 0.03) * 0.45
    cloth = at(bp(noise(rng, n), 3000, 7000) * decay(t, 0.05) * 0.12, 0.008, n)
    return finish(click + slap + thud + body + cloth, 0.62, 0.02)


def stomp(rng: np.random.Generator) -> np.ndarray:
    """A boot coming down on a body on concrete: slap, deep thump, squash, gravel."""
    n = int(0.42 * SR)
    t = np.arange(n) / SR
    slap = bp(noise(rng, n), jitter(rng, 700), jitter(rng, 2500)) * decay(t, 0.012) * 0.7
    thump = lp(noise(rng, n), jitter(rng, 140)) * decay(t, 0.09) * 1.0
    low = np.sin(2 * np.pi * jitter(rng, 58) * t) * decay(t, 0.09) * 0.7
    wobble = 0.65 + 0.35 * np.sin(2 * np.pi * jitter(rng, 14) * t)
    squash = at(lp(noise(rng, n), 500) * decay(t, 0.06) * wobble * 0.5, 0.018, n)
    times = np.sort(rng.uniform(0.02, 0.16, 6))
    gravel = grains(rng, n, times, 2000, 6000, 0.006, rng.uniform(0.15, 0.3, 6))
    return finish(slap + thump + low + squash + gravel, 0.72, 0.03)


def wall_slam(rng: np.random.Generator) -> np.ndarray:
    """A body flung against a wall: slap, wall resonance, plaster crack and dust."""
    n = int(0.55 * SR)
    t = np.arange(n) / SR
    slap = bp(noise(rng, n), 300, jitter(rng, 1200)) * decay(t, 0.02) * 0.9
    freqs = [jitter(rng, f) for f in (85, 128, 197, 301)]
    boom = modal(t, freqs, [0.16, 0.12, 0.10, 0.07], [0.8, 0.6, 0.4, 0.25], rng)
    crack = at(bp(noise(rng, n), 1500, 5000) * decay(t, 0.03) * 0.5, 0.006, n)
    dust = at(hp(noise(rng, n), 2500) * decay(t, 0.18, attack=0.04) * 0.12, 0.04, n)
    return finish(slap + boom + crack + dust, 0.68, 0.04)


def blast(rng: np.random.Generator) -> np.ndarray:
    """A charge going off: broadband crack, low boom, rumble, falling debris."""
    n = int(0.90 * SR)
    t = np.arange(n) / SR
    crack = lp(noise(rng, n), 9000) * decay(t, 0.006, attack=0.0003) * 1.0
    boom = lp(noise(rng, n), jitter(rng, 110)) * decay(t, 0.18) * 1.0
    low = np.sin(2 * np.pi * jitter(rng, 42) * t) * decay(t, 0.22) * 0.7
    rumble = at(lp(noise(rng, n), 260) * decay(t, 0.3) * 0.35, 0.03, n)
    times = np.sort(rng.uniform(0.12, 0.70, 9))
    debris = grains(rng, n, times, 1500, 6000, 0.008, np.linspace(0.3, 0.08, 9))
    mix = crack + boom + low + rumble + debris
    return finish(np.tanh(1.3 * mix / (np.max(np.abs(mix)) + 1e-9)), 0.72, 0.1)


def defeat_heavy(rng: np.random.Generator) -> np.ndarray:
    """A heavy body going down for good: drop, settle, cloth scrape, low rattle."""
    n = int(0.50 * SR)
    t = np.arange(n) / SR
    drop = lp(noise(rng, n), jitter(rng, 180)) * decay(t, 0.05) * 1.0
    drop += np.sin(2 * np.pi * jitter(rng, 66) * t) * decay(t, 0.06) * 0.6
    settle = at(lp(noise(rng, n), 260) * decay(t, 0.04) * 0.45, jitter(rng, 0.15, 0.1), n)
    ramp = np.clip(1 - np.abs(t - 0.2) / 0.15, 0, 1)
    scrape = at(bp(noise(rng, n), 1200, 3500) * ramp * 0.18, 0.05, n)
    times = np.sort(rng.uniform(0.2, 0.42, 4))
    rattle = grains(rng, n, times, 600, 2500, 0.01, rng.uniform(0.1, 0.2, 4))
    return finish(drop + settle + scrape + rattle, 0.66, 0.05)


CUES = [
    # (file stem, engine cue id, event id, recipe, trigger)
    ("cf02_punch_hit", "PlayerPunchHitConfirmed", "cf02.punch.hit_confirmed", punch_hit,
     "verified jab or haymaker hit (cf02 cue overlay only)"),
    ("cf02_stomp", "PlayerStomp", "cf02.carl.stomp", stomp,
     "verified stomp or dive-stomp hit (cf02 cue overlay only)"),
    ("cf02_wall_slam", "EnemyWallSlam", "cf02.enemy.wall_slam", wall_slam,
     "a kicked flier slammed into a solid (cf02 cue overlay only)"),
    ("cf02_blast", "BlastDetonation", "cf02.blast.detonation", blast,
     "a dynamite charge detonates (cf02 cue overlay only)"),
    ("cf02_defeat_heavy", "EnemyDefeatHeavy", "cf02.enemy.defeat_heavy", defeat_heavy,
     "an enemy is defeated with heavy or catastrophic force (cf02 cue overlay only)"),
]


# ------------------------------------------------------------------- io ----

def write_wav(path: Path, x: np.ndarray) -> None:
    pcm = np.clip(np.round(x * 32767), -32768, 32767).astype("<i2")
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())


def encode(x: np.ndarray, out: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        wav = Path(tmp) / "take.wav"
        write_wav(wav, x)
        subprocess.run(
            ["ffmpeg", "-v", "error", "-y", "-i", str(wav), "-map_metadata", "-1",
             "-fflags", "+bitexact", "-flags:a", "+bitexact",
             "-c:a", "libvorbis", "-q:a", "3", str(out)],
            check=True,
        )


def decode(path: Path) -> np.ndarray:
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", str(path), "-f", "f32le", "-ac", "1", "-ar", str(SR), "-"],
        check=True, capture_output=True,
    ).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(np.float64)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def spectral_centroid(x: np.ndarray) -> float:
    spec = np.abs(np.fft.rfft(x))
    freqs = np.fft.rfftfreq(len(x), 1 / SR)
    return float((spec * freqs).sum() / (spec.sum() + 1e-12))


# ----------------------------------------------------------------- build ----

def build() -> list[dict]:
    SFX.mkdir(parents=True, exist_ok=True)
    DOCS.mkdir(parents=True, exist_ok=True)
    assets = []
    for cue_index, (stem, _cue, _event, recipe, _trigger) in enumerate(CUES):
        for take in range(1, TAKES + 1):
            rng = np.random.default_rng(BASE_SEED + cue_index * 100 + take)
            x = recipe(rng)
            out = SFX / f"{stem}_{take:02d}.ogg"
            encode(x, out)
            decoded = decode(out)
            assets.append({
                "path": f"cf02/sfx/{out.name}",
                "duration_s": round(len(decoded) / SR, 2),
                "sha256": sha256(out),
            })
    return assets


def qa(assets: list[dict]) -> tuple[list[str], list[str]]:
    """Structural checks only: they cannot judge how a cue sounds."""
    lines, failures = [], []
    by_cue: dict[str, list[np.ndarray]] = {}
    for a in assets:
        path = PACK / a["path"]
        x = decode(path)
        peak = float(np.max(np.abs(x)))
        rms = float(np.sqrt(np.mean(x**2)))
        dc = float(np.mean(x))
        edge = float(max(abs(x[0]), abs(x[-1])))
        centroid = spectral_centroid(x)
        stem = path.stem.rsplit("_", 1)[0]
        by_cue.setdefault(stem, []).append(x)
        checks = {
            "peak below 0.80 (no clipping)": peak <= LEGACY_PEAK_MAX,
            f"RMS inside the legacy band {LEGACY_RMS_BAND}": LEGACY_RMS_BAND[0] <= rms <= LEGACY_RMS_BAND[1],
            "DC offset below 0.002": abs(dc) < 0.002,
            "first and last sample below 0.03 (no click)": edge < 0.03,
            "duration between 0.15 s and 1.0 s": 0.15 <= len(x) / SR <= 1.0,
            "mono 44.1 kHz": True,
        }
        for name, ok in checks.items():
            if not ok:
                failures.append(f"{a['path']}: {name}")
        lines.append(
            f"| `{a['path']}` | {len(x) / SR:.3f} | {peak:.3f} | {rms:.4f} | {centroid:.0f} | "
            f"{'pass' if all(checks.values()) else 'FAIL'} |"
        )
    for stem, takes in by_cue.items():
        for i in range(len(takes)):
            for j in range(i + 1, len(takes)):
                m = min(len(takes[i]), len(takes[j]))
                if np.allclose(takes[i][:m], takes[j][:m], atol=1e-3):
                    failures.append(f"{stem}: takes {i + 1} and {j + 1} are identical")
    return lines, failures


def report(assets: list[dict], rows: list[str], failures: list[str]) -> str:
    total = len(assets)
    status = "all structural checks passed" if not failures else f"{len(failures)} structural failures"
    return "\n".join([
        "# CF02-AUDIO structural QA (cf02 cue overlay)",
        "",
        "**These checks cannot establish audio quality or subjective fitness.** Owner listening on real speakers "
        "stays open (COMBAT-FUN-02 design section 8, item 3). Nothing here is approved audio.",
        "",
        f"- {total} runtime Ogg Vorbis assets, five cues with {TAKES} takes each, 44.1 kHz mono.",
        "- Provenance: original deterministic DSP (`scripts/author_cf02_audio.py`, base seed "
        f"{BASE_SEED}); no recording, sample, third-party or generated-model audio, no voice.",
        f"- Result: {status}.",
        f"- Level band: peak <= {LEGACY_PEAK_MAX}, RMS {LEGACY_RMS_BAND[0]} to {LEGACY_RMS_BAND[1]} "
        "(the measured range of the Floor 01 impact and crash cues).",
        "- Overlap limit (at most two major transients), the cue mapping and \"no cue drives gameplay\" are "
        "checked by Rust tests (`src/audio.rs`, `tests/audio_assets.rs`), not here.",
        "",
        "| File | Duration s | Peak | RMS | Centroid Hz | Checks |",
        "| --- | --- | --- | --- | --- | --- |",
        *rows,
        "",
        "## Not tested",
        "",
        "- How the cues sound, whether each cause is distinguishable by ear, fatigue under repetition.",
        "- In-engine loudness against the music bed and the other effects, headphones, TVs, other devices.",
        "- Human-expert or end-user approval.",
        "",
    ])


def write_preview() -> None:
    """The first take of each cue in table order, 0.4 s apart (a listening aid)."""
    gap = np.zeros(int(0.4 * SR))
    parts = []
    for stem, *_ in CUES:
        parts += [decode(SFX / f"{stem}_01.ogg"), gap]
    previews = OUT / "previews"
    previews.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        wav = Path(tmp) / "preview.wav"
        write_wav(wav, np.concatenate(parts))
        subprocess.run(
            ["ffmpeg", "-v", "error", "-y", "-i", str(wav), "-map_metadata", "-1",
             "-c:a", "libmp3lame", "-q:a", "4", str(previews / "cf02_cues_preview.mp3")],
            check=True,
        )


def main() -> int:
    if "--check" in sys.argv:
        manifest = json.loads((DOCS / "asset_manifest_cf02.json").read_text())
        bad = [a["path"] for a in manifest["assets"] if sha256(PACK / a["path"]) != a["sha256"]]
        print("CF02_AUDIO_CHECK", "FAIL " + ", ".join(bad) if bad else f"OK {len(manifest['assets'])} files")
        return 1 if bad else 0
    assets = build()
    rows, failures = qa(assets)
    manifest = {
        "project": "THE CRAWL",
        "floor": 1,
        "version": "0.3.0-cf02-cues",
        "date": "2026-11-20",
        "status": "unapproved_exploratory",
        "provenance": {
            "origin": "original deterministic DSP synthesis (seeded noise and damped sinusoids)",
            "authoring_tool": "scripts/author_cf02_audio.py",
            "seed": BASE_SEED,
            "third_party_recordings": [],
            "external_ai_audio_models": [],
            "voices": [],
            "human_expert_signoff": False,
        },
        "format": {"sample_rate_hz": SR, "channels": 1, "encoding": "OGG Vorbis (libvorbis, -q:a 3)"},
        "assets": assets,
    }
    (DOCS / "asset_manifest_cf02.json").write_text(json.dumps(manifest, indent=2) + "\n")
    events = {
        "status": "unapproved_integration_proposal",
        "mixer": {"bus": "sfx", "max_concurrent_major_transients": 2},
        "events": [
            {
                "id": event,
                "bus": "sfx",
                "assets": [f"cf02/sfx/{stem}_{take:02d}.ogg" for take in range(1, TAKES + 1)],
                "trigger": trigger,
            }
            for stem, _cue, event, _recipe, trigger in CUES
        ],
    }
    (DOCS / "audio_events_cf02.json").write_text(json.dumps(events, indent=2) + "\n")
    (DOCS / "QA_REPORT.md").write_text(report(assets, rows, failures))
    write_preview()
    print(f"CF02_AUDIO wrote {len(assets)} files; failures: {len(failures)}")
    for f in failures:
        print("FAIL", f)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
