"""Synthetic chromatogram-only MRM .wiff + .wiff.scan whose blocks are sparse (scheduled) while the method has no `sMRM` stream to say so
(crates/mzlab-lettori/tests/sciex_sonda.rs).

    python3 tools/wiff_mrm_rado.py OUT [SAMPLES]

Writes OUT/mrmrado.wiff(.scan), no Log: SAMPLES samples (default 1), each one MRM experiment of 136 transitions over 40 cycles, every block
sparse (`-(136.01)`, then cps of the acquired transitions and `-(k + 0.01)` for k skipped ones). Built with the Writer of wiff_riferimento.py; the
numbers are invented, no laboratory data is used.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wiff_riferimento import Writer, header, ion_spray, mrm_method  # noqa: E402

TRANSITIONS = 136
CYCLES = 40


def blocco(c: int) -> list:
    """Cycle `c`: the transitions within 12 of `3.5 * c` were acquired (window of a made-up schedule), the others skipped."""
    out, salto = [-(TRANSITIONS + 0.01)], 0
    for j in range(TRANSITIONS):
        if abs(j - 3.5 * c) <= 12:
            if salto:
                out.append(-(salto + 0.01))
                salto = 0
            out.append(100.0 + 7.0 * j + c)
        else:
            salto += 1
    if salto:
        out.append(-(salto + 0.01))
    return out


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    samples = int(sys.argv[2]) if len(sys.argv) > 2 else 1
    trs = [(200.0 + j, 100.0 + j / 2, 1.0 + j / 40, f"t{j}", 30.0, 10.0, 20.0, 5.0) for j in range(TRANSITIONS)]
    method = {"Experiment0/ExperimentHeader": header(4, 0.0), "Experiment0/MassRangeEx/MassRangeEx": mrm_method(trs),
              "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
    w = Writer()
    for n in range(1, samples + 1):
        blocks = []
        for c in range(CYCLES):
            b = blocco(c)
            blocks.append(("mrm", b, 3000.0 * c, sum(x for x in b if x > 0)))
        w.sample(n, blocks, method)
    w.save(out, "mrmrado")


if __name__ == "__main__":
    main()
