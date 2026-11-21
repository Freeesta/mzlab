"""The command-line flags of the tools are English; the Italian ones must not come back (no aliases)."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = re.compile(r"(?<![\w-])--(uscita-esportazione|timsdata-versione|uscita-sorbenti|senza-controlli|ultimi-falliti|trfp-versione|altezza-min|sito-pronto|senza-disco|rt-genitore|elimina-raw|mz-genitore|senza-wasm|robustezza|controlla|sintetico|sintetici|senza-e2e|paralleli|pacchetto|mostra-ok|minimizza|etichetta|dettaglio|aggiorna|completo|cambiati|peggiori|varianti|rapporto|versione|tetto-gb|sospetti|solo-fmt|sorbenti|serie-lr|rinomine|registri|registra|librerie|genitore|composti|campioni|campione|campagna|aggiungi|profilo|scenari|tabelle|oracolo|massimo|elimina|cursore|analisi|maquina|scrivi|elenco|misura|uscita|verita|nativo|lavoro|giorni|veloce|ultimi|taglio|rapida|minimo|ignora|gruppo|gruppi|genera|evento|esteso|debole|bracci|atteso|prova|lista|fonte|vista|togli|tetto|stato|serie|scala|reale|passi|parte|nuovi|lotto|denso|ciclo|cicli|altri|aiuti|solo|seme|semi|giri|dati|tipo|sito|riga|nota|fumo|crea|oro)(?![\w-])")
# `msn rules --lista` is the refusal test of the old CLI flag; the `cancello` flags follow their own family
SKIP = {"tests/test_no_old_flag_names.py", "crates/mzlab-cli/src/msn_rules/tests.rs", "docs/agenti/english-remaining.md"}


def test_old_flags_are_gone():
    files = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    hits = []
    for f in files:
        if not f or f in SKIP or "/vendor/" in f or f.startswith("mzlab/web/esempi/") or not f.endswith((".py", ".rs", ".md", ".yml", ".json", ".sh", ".js", ".toml")):
            continue
        try:
            text = (ROOT / f).read_text(encoding="utf8")
        except (OSError, UnicodeDecodeError):
            continue
        for n, line in enumerate(text.splitlines(), 1):
            if OLD.search(line):
                hits.append(f"{f}:{n}")
    assert not hits, hits[:20]
