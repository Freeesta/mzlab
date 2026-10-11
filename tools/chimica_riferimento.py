"""Reference answers of mzlab/chem (elements, subformulas) for crates/mzlab-core/tests/chimica_parity.rs.

    python3 tools/chimica_riferimento.py OUT.json

Generic formulas only (no compound names): [{kind, ...}].
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.chem.elements import formula_mz, normalize_formula  # noqa: E402
from mzlab.chem.subformulas import submasses, subformulas  # noqa: E402
from mzlab.i18n import UserError  # noqa: E402

FORMULAS = ["C6H12O6", "c9h10cl2n2o", "CH4", "C(OH)2", "C2H5(OH)", "H2O", "CO", "co", "Co", "C15H12N2O", "C12H10ClNO2S", "Br2", "NaCl", "C2H3Cl3", "(CH3)3N", "C6H5NO2", "",
            " c 6 h 6 ", "Xx2", "C(", "C)", "C2(H", "1C", "C6H12O6)2", "((CH3)2)3", "c1h4", "SiH4", "SeO2", "KI", "C100H200", "C0H4", "H0", "Cl", "cl2", "CuSO4", "Fe2O3", "Ca(OH)2"]
ADDUCTS = [None, "[M+H]+", "[M+Na]+", "[M-H]-", "[M+HCOO]-", "[M+NH4]+", "[M+K]+", "[M+Cl]-", "[M+X]+"]


def run(fn):
    try:
        return {"ok": json.loads(json.dumps(fn()))}
    except UserError as e:
        return {"err": e.key if hasattr(e, "key") else str(e), "params": {k: str(v) for k, v in (getattr(e, "params", None) or {}).items()}}
    except Exception as e:  # noqa: BLE001
        return {"err": "python:" + type(e).__name__}


def main() -> None:
    out = []
    for f in FORMULAS:
        out.append({"kind": "norm", "f": f, "norm": normalize_formula(f)})
        for a in ADDUCTS:
            out.append({"kind": "formula", "f": f, "adduct": a, **run(lambda: formula_mz(f, a))})
    rng = np.random.default_rng(5)
    for f in ["C6H12O6", "C9H10Cl2N2O", "C12H10ClNO2S", "CH4", "C2H5NO2", "C8H9NO", "Br2H", "C3H8N3O2PS", "C20H40", "C30H60O10"]:
        for z in (1, 2, -1, 0):
            for mn in (0.0, 50.0, 120.5):
                out.append({"kind": "sub", "f": f, "z": z, "min": mn, **run(lambda: [[n, m] for n, m in subformulas(f, z, mn)[1]])})
                out.append({"kind": "submasses", "f": f, "z": z, "min": mn, **run(lambda: submasses(f, z, mn).tolist())})
    Path(sys.argv[1]).write_text(json.dumps(out), encoding="utf-8")
    print(len(out))


if __name__ == "__main__":
    main()
