"""Writes docs/azioni.md, the encyclopedia of the actions that change state, from COMANDI_TAB in mzlab/web/comandi.js (never by hand).

    python3 tools/doc_comandi.py          # rewrite the page
    python3 tools/doc_comandi.py --check  # exit 1 if the page is not the one the table gives (tests/test_comandi.py)
"""
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
JS = ROOT / "mzlab" / "web" / "comandi.js"
OUT = ROOT / "docs" / "azioni.md"
UNDO = {
    "istantanea": "undo with a snapshot (storia.js)", "comando": "command: do / undo handler (comandi.js)", "tolto": "removal with its own undo / redo (STORIA.tolto)",
    "layout": "arrangement of the panels (affianca.js)", "flow": "own stack of the Flow (S.undo)", "draw": "own stack of the editor (Ketcher)",
    "da-portare": "NOT undoable yet", "nessuna": "not undone",
}


def tabella() -> list[dict]:
    """The rows of COMANDI_TAB: the text between TAB-BEGIN and TAB-END is a plain literal, evaluated by node."""
    src = JS.read_text(encoding="utf-8")
    corpo = src.split("// TAB-BEGIN", 1)[1].split("// TAB-END", 1)[0]
    r = subprocess.run(["node", "-e", corpo + "\nconsole.log(JSON.stringify(COMANDI_TAB))"], capture_output=True, text=True, encoding="utf-8", check=True)
    return json.loads(r.stdout)


def etichette(lang: str) -> dict[str, str]:
    return dict(re.findall(r'^\s*"([^"]+)":\s*"((?:[^"\\]|\\.)*)"', (ROOT / "mzlab" / "web" / "lang" / f"{lang}.js").read_text(encoding="utf-8"), re.M))


def pagina() -> str:
    righe, it = tabella(), etichette("it")
    o = ["# Azioni che cambiano stato (generato)", "",
         "Generato da `python3 tools/doc_comandi.py` dalla tabella `COMANDI_TAB` di `mzlab/web/comandi.js`: non si modifica a mano. "
         "Ogni azione che cambia stato nella vista Dati sta nella tabella; `tests/test_comandi.py` lo controlla.", "",
         "| Id | Nome | Modulo | Cosa sceglie l'utente | Stato che cambia | Tasto | Passi | Annullamento |", "|---|---|---|---|---|---|---|---|"]
    for r in righe:
        und = UNDO[r["annulla"]] + (f": {r['motivo']}" if r.get("motivo") else "")
        o.append(f"| `{r['id']}` | {it.get(r['nome'], r['nome'])} (`{r['nome']}`) | `{r['modulo']}` | {r['parametri']} | {', '.join(r['campi']) or '-'} | {r['tasto'] or '-'} | {r['unisci'] or '-'} | {und} |")
    n = sum(r["annulla"] == "da-portare" for r in righe)
    o += ["", f"Da portare nello stack: {n} azioni su {len(righe)}.", ""]
    return "\n".join(o)


def main() -> int:
    if not shutil.which("node"):
        print("node not available")
        return 0 if "--check" in sys.argv else 1
    testo = pagina()
    if "--check" in sys.argv:
        return 0 if OUT.exists() and OUT.read_text(encoding="utf-8") == testo else 1
    OUT.write_text(testo, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
