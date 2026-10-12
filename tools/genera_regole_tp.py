"""Writes TP_Mine/data/regole_tp.json (format `regole_tp/1`) from the list of transformations that mzFinder already has.

    python3 tools/genera_regole_tp.py [--controlla]

The rules are `tpmine.hr.network.transformation_table` (the HR list plus the default transformations of `tpmine.chem`, one per net change).
`--controlla` only checks that the committed file is the one this script writes. The fine curation of the rules is the chemist's: the list can be
replaced without touching the code (the Rust reader is `crates/mzlab-feat/src/hypo`)."""
import json
import re
import sys
from pathlib import Path

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI.parent))
sys.path.insert(0, str(AQUI.parent / "TP_Mine" / "py"))
from tpmine import chem  # noqa: E402
from tpmine.hr import network as NW  # noqa: E402
from tpmine.hr import rules as RU  # noqa: E402

DEST = AQUI.parent / "TP_Mine" / "data" / "regole_tp.json"
ELS = ["C", "H", "N", "O", "S", "F", "Cl", "Br"]
POLARE, APOLARE = "piu_polare", "meno_polare"
# direction of the retention time of the product against the parent in reversed phase (provisional; the chemist curates it)
DIREZIONE = {
    "hydroxylation": POLARE, "dihydroxylation": POLARE, "trihydroxylation": POLARE, "hydration": POLARE, "carboxylation": POLARE,
    "demethylation": POLARE, "loss of C4H8": POLARE, "loss of C2H4": POLARE, "loss of propene (N-deisopropylation)": POLARE,
    "hydrolytic dechlorination": POLARE, "hydrolytic defluorination": POLARE, "decarboxylation": POLARE,
    "methylation": APOLARE, "dehydration": APOLARE, "reductive dechlorination": APOLARE, "reductive debromination": APOLARE,
}


def identificatore(nome: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", nome.lower()).strip("_")


def regole() -> dict:
    out = []
    for nome, vec in NW.transformation_table(ELS):
        delta = {e: int(n) for e, n in zip(ELS, vec) if n}
        r = {"id": identificatore(nome), "nome": nome, "delta_formula": chem.fmt_delta(delta), "passi_max": 2,
             "livello_previsione": 3 if nome in RU.CLASS_RULES else 4, "fonte": "transformation_table",
             "direzione_rt": DIREZIONE.get(nome, "libera")}
        if nome in RU.CLASS_RULES:
            need = RU.CLASS_RULES[nome]
            r["richiede"] = {"tutti": need.get("all", {}), "almeno_uno": need.get("any", [])}
        out.append(r)
    return {"format": "regole_tp", "version": 1, "regole": out}


def main() -> int:
    testo = json.dumps(regole(), indent=1, ensure_ascii=False) + "\n"
    if "--controlla" in sys.argv:
        if not DEST.exists() or DEST.read_text(encoding="utf-8") != testo:
            print("regole_tp.json non coincide con transformation_table: python3 tools/genera_regole_tp.py")
            return 1
        return 0
    DEST.write_text(testo, encoding="utf-8")
    print(f"{DEST}: {len(regole()['regole'])} regole")
    return 0


if __name__ == "__main__":
    sys.exit(main())
