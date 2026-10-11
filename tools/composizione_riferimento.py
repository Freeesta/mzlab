"""Reference answers of mzlab/chem/composition.py (compose without obs/frags, parse_elements) for crates/mzlab-core/tests/composizione_parity.rs.

    python3 tools/composizione_riferimento.py OUT.json

m/z values come from generic formulas (no compound names): [{kind, args, ok | err}].
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.chem.composition import compose, parse_elements  # noqa: E402
from mzlab.chem.elements import ADDUCT_SHIFT, mass, parse_formula  # noqa: E402
from mzlab.i18n import UserError  # noqa: E402

BASES = ["C6H12O6", "C9H10Cl2N2O", "C12H10ClNO2S", "C8H9NO2", "C15H12N2O", "C20H25N3O", "C7H8N4O2", "C10H14N5O7P", "C13H24N4O3S", "CH4N2O", "C30H50O2", "C5H5N5"]


def run(fn):
    try:
        return {"ok": json.loads(json.dumps(fn()))}
    except UserError as e:
        return {"err": e.key, "params": {k: str(v) for k, v in e.params.items()}}
    except Exception as e:  # noqa: BLE001
        return {"err": "python:" + type(e).__name__}


def main() -> None:
    out = []
    for txt in ["C:0-13,H:0-25,N:0-4,13C:0-2", "C:1-5;H:2-8", "Na:1", "C : 0 - 3 , O:2", "", "C", "C:", "Xx:1-2", "C:5-2", "13C:0-1,15N:0-2,34S:0-1", "C:0-3,C:1-2", "c:1-2", "H:3"]:
        out.append({"kind": "parse", "text": txt, **run(lambda: {k: list(v) for k, v in parse_elements(txt).items()})})
    cases = []
    for f in BASES:
        m = mass(parse_formula(f))
        for adduct in ("[M+H]+", "[M+Na]+", "[M-H]-", "[M+NH4]+", "[M+HCOO]-"):
            cases.append((m + ADDUCT_SHIFT[adduct], dict(ion=adduct)))
    for mz in (150.0, 285.1234, 364.05, 123.456):
        for kw in (dict(), dict(tol=2.0), dict(tol=3.0, unit="mda"), dict(rdb=(0.0, 20.0)), dict(nitrogen="even"), dict(nitrogen="odd"), dict(rules=["HC", "LEWIS", "SENIOR"]),
                   dict(elements="C:0-20,H:0-40,N:0-5,O:0-8"), dict(elements="C:0-20,H:0-40,N:0-5,O:0-8,S:0-2,Cl:0-3,Br:0-1", max_results=50),
                   dict(ion="M+."), dict(ion="[M]+"), dict(ion="[M]-"), dict(ion="[M+K]+"), dict(ion="[M+Cl]-"), dict(z=2), dict(parent="C20H26N3O2"),
                   dict(m1=0.2, m2=0.03), dict(elements="C:0-30,H:0-50,13C:0-2,15N:0-1,N:0-4,O:0-6", tol=5.0), dict(ion="bad"), dict(unit="x"), dict(parent="Xx")):
            cases.append((mz, kw))
    for f in BASES[:6]:
        mz = mass(parse_formula(f)) + ADDUCT_SHIFT["[M+H]+"]
        obs = [(mz, 100.0), (mz + 1.00335, 7.0 * len(f) / 4), (mz + 2.0067, 3.5), (mz + 5.0, 2.0), (mz - 0.5, 1.5)]
        frags = [(mz - 18.0106, 50.0), (mz - 44.0, 100.0), (100.0, 20.0), (mz, 10.0), (60.0, 5.0), (mz + 1.0, 4.0)]
        for kw in (dict(obs=obs), dict(obs=obs, norm="mono"), dict(obs=obs, norm="max"), dict(obs=obs, norm="sum"), dict(obs=obs, r200=0.0), dict(obs=obs, r200=240000.0, sigma_ppm=1.0, sigma_int=0.2),
                   dict(obs=obs, iso_min_rel=0.1, iso_noise_rel=3.0), dict(frags=frags), dict(frags=frags, frag_ppm=20.0), dict(obs=obs, frags=frags, weights=(1.0, 1.0, 3.0)),
                   dict(obs=obs, frags=frags, max_results=3, pool=20), dict(obs=[], frags=frags), dict(obs=obs, ion="[M+Na]+"), dict(obs=obs, elements="C:0-30,H:0-50,13C:0-1,N:0-4,O:0-6")):
            cases.append((mz, dict(tol=5.0, **kw)))
    cases += [(-1.0, {}), (0.0, {}), (float("nan"), {})]
    for mz, kw in cases:
        kw = dict(kw)
        if isinstance(kw.get("elements"), str):
            kw["elements"] = parse_elements(kw["elements"])
        args = {k: (v if k != "elements" else {s: list(r) for s, r in v.items()}) for k, v in kw.items()}
        args = {k: ([list(x) for x in v] if k in ("obs", "frags") else v) for k, v in args.items()}
        if mz != mz:
            continue
        out.append({"kind": "compose", "mz": mz, "args": args, **run(lambda: compose(mz, **kw))})
    Path(sys.argv[1]).write_text(json.dumps(out), encoding="utf-8")
    print(len(out))


if __name__ == "__main__":
    main()
