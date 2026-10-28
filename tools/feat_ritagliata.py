"""Synthetic series cut in RT, for the parity of the cells not measured (`CELL_NA`) between the `.wasm` and the native engine.

    python3 tools/feat_ritagliata.py --out DIR [--taglio 3:10] [--seme 2]

The series `piccolo` of `feat_mirato.py`, then the files `t05min` and `t20min` are cut to the scans between LO and HI minutes (a valve that sends the flow to
the mass spectrometer only for a part of the run): the other files stay whole. Writes the `.mzML` and `.mzidx` of DIR (reference only: never in the site)."""
import argparse
import re
import sys
from pathlib import Path

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI))
import feat_mirato                                       # noqa: E402
from mzlab.reader import mzidx                           # noqa: E402
from mzlab.reader.mzml import Run                        # noqa: E402

CUT = ("t05min", "t20min")
SPECTRUM = re.compile(r"<spectrum .*?</spectrum>", re.S)
START = re.compile(r'scan start time" value="([\d.]+)"')


def cut(text: str, lo: float, hi: float) -> str:
    """The mzML without the spectra whose scan start time (minutes) is outside [lo, hi]; the chromatograms stay."""
    kept = [s for s in SPECTRUM.findall(text) if lo <= float(START.search(s).group(1)) <= hi]
    head, rest = text.split("<spectrumList", 1)
    tail = rest.split("</spectrumList>", 1)[1]
    open_tag = re.match(r"[^>]*>", rest).group(0)
    open_tag = re.sub(r'count="\d+"', f'count="{len(kept)}"', open_tag)
    return head + "<spectrumList" + open_tag + "".join(kept) + "</spectrumList>" + tail


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", required=True)
    ap.add_argument("--taglio", default="3:10")
    ap.add_argument("--seme", type=int, default=2)
    a = ap.parse_args()
    out = Path(a.out)
    lo, hi = (float(x) for x in a.taglio.split(":"))
    feat_mirato.genera(a.seme, "piccolo", 1.0, out)
    for stem in CUT:
        f = out / f"SIN_{stem}.mzML"
        f.write_text(cut(f.read_text(encoding="utf-8"), lo, hi), encoding="utf-8")
        mzidx.build(Run(f, index_path=out / f"SIN_{stem}.mzidx"), out / f"SIN_{stem}.mzidx")


if __name__ == "__main__":
    main()
