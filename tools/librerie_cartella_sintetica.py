#!/usr/bin/env python3
"""Builds a SYNTHETIC folder of spectral libraries for the «Load folder…» benchmark (no real data).
One big NIST-format MSP (default 110 000 short spectra), four small mixes (21-51 spectra), a few empty/unsupported library files and many files of other
types, like a real MassBank download folder. Usage: python3 tools/librerie_cartella_sintetica.py OUT_DIR [--n 110000] [--altri 730]"""
import argparse, random
from pathlib import Path


def msp_block(i: int, rng: random.Random) -> str:
    neg = rng.random() < 0.3
    prec = round(rng.uniform(150, 900), 4)
    npk = rng.randint(4, 12)
    mz = sorted(round(rng.uniform(50, prec - 3), 4) for _ in range(npk))
    peaks = "\n".join(f"{m} {round(rng.random() * 1000 + 10, 1)}" for m in mz)
    return (f"Name: Synthetic {i}\nPrecursorMZ: {prec}\nPrecursor_type: {'[M-H]-' if neg else '[M+H]+'}\n"
            f"Ion_mode: {'Negative' if neg else 'Positive'}\nCollision_energy: 30\nInstrument_type: Orbitrap\nNum Peaks: {npk}\n{peaks}\n\n")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("out"); ap.add_argument("--n", type=int, default=110_000); ap.add_argument("--altri", type=int, default=730)
    a = ap.parse_args()
    out = Path(a.out); out.mkdir(parents=True, exist_ok=True)
    rng = random.Random(7)
    with open(out / "sint_big.msp", "w", encoding="utf-8", buffering=1 << 20) as f:
        for i in range(a.n):
            f.write(msp_block(i, rng))
    for k, n in enumerate((21, 33, 44, 51)):
        (out / f"mix_{k + 1}.msp").write_text("".join(msp_block(i, rng) for i in range(n)), encoding="utf-8")
    (out / "vuota.msp").write_text("")
    (out / "senza_picchi.msp").write_text("Name: x\nPrecursorMZ: 300.1\nNum Peaks: 0\n\n")
    (out / "rotta.json").write_text("{not json")
    for i in range(a.altri):
        (out / f"nota_{i:04d}.txt").write_text(f"synthetic note {i}\n")
    print(f"{out}: {a.n} + 149 spectra, {a.altri + 3} other files")


if __name__ == "__main__":
    main()
