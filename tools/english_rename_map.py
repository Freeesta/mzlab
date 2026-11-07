"""Proposed English names for the Italian file and directory names of mzLab (used by `tools/controlla_inglese.py --rinomine`).
Word by word from tools/italian_words.tsv, then three Italian word-order rules: adjective after its noun, `X di Y`, and a head noun
followed by its complement (`calibrazione_massa` -> `mass_calibration`). Topic-first names (`gruppi-vista`, `e2e_mappa_assi`) are
already English order and stay as they are.

    python3 tools/english_rename_map.py       # TSV: kind, old path, new path, note
"""
from __future__ import annotations

import re
import sys
from collections import Counter

import controlla_inglese as ci

# Italian adjectives follow their noun: `lettore_finto` -> `fake_reader`
ADJECTIVES = set("""sintetico sintetica sintetici sintetiche veloce veloci comune comuni pubblici pubblico pubbliche noti unito finto grezzi
grezza mirato diretta diretto nativo nativa nativi completa rapida rapido lenta lento lenti fresca stretto stretta nuovo nuova alta grandi
grande piatte piatto leggera fluida morti rotti parziali singoli misti esterni esterno estesi esteso potenziale computazionale immediato
variabili ostili danneggiati guidati fissi fisso fissa unica unico densa denso doppi doppio corti larghi enorme enormi incerto sicuri
predefinito predefinita precalcolata ritagliata puntata centrata condiviso dinamici finiti sperimentali benzoico""".split())
# head nouns that Italian puts before their complement: `regole-frammentazione` -> `fragmentation-rules`
HEADS = set("""calibrazione correzione diagnosi regole regola guardie guardia cursore tabella tabelle elenco barra finestra
nomi nome numero numeri struttura strutture indice indici""".split())
# never the complement of a head noun: quantifiers, and words that name the kind of file (`msn-regole-prove` = rule tests)
NOT_COMPLEMENT = set("molti molte tutti tutte due uno nessun prove prova vista viste tour".split())
ADJ_EN = {"variabili": "variable"}          # adjectives whose dictionary form is the noun
OF = {"di", "del", "della", "dei", "delle", "degli", "dello"}
VERBS = set("""abbina accorpa adatta affianca aggiorna aggiungi allinea allineo annota annulla apre apri attendi avanza avvia calcola carica cerca
chiude chiudi confronta conta controlla convalida converti copia corri crea deriva elimina esporta estrai fondi genera importa installa
integra interpreta leggi migra mostra naviga prepara ricarica ricollega ridimensiona rimescola riscalda riscrivi ritaglia salva scalda
scarica scrivi servi sottrai spezza spiega stima suggerisci togli torna trascina unisci valida valuta verifica visualizza sposta
ingrandisci trova registra applica filtra ordina raggruppa segna riempi rifai riapri pulisci prendi tieni ripeti esegui""".split())
SEPS = ("_", "-")
NO_RENAME = re.compile(r"(^|/)MSBNK-")          # MassBank accessions: third-party ids
# names chosen by hand where the word-by-word form collides with an existing file; a Rust module directory follows its file
OVERRIDES = {"crates/mzlab-core/src/flow/corsa.rs": "run_report.rs", "crates/mzlab-core/src/flow/corsa_tests.rs": "run_report_tests.rs",
             "crates/mzlab-core/src/msnalbero.rs": "msntree_build.rs", "mzlab/web/avanzamento.js": "job-progress.js"}
DIR_OVERRIDES = {k[:-3]: v[:-3] for k, v in OVERRIDES.items() if k.endswith(".rs")}


def _noun(w: str) -> bool:
    return w in ci.DICT and w not in VERBS and w not in ADJECTIVES and w not in OF and w not in ci.PROPER


def reorder(toks: list[str]) -> tuple[list[str], bool]:
    """Apply the Italian word-order rules to the tokens of one name; True when one fired."""
    low, fired, k = [t.lower() for t in toks], False, 2
    while k < len(toks):
        a, sep, b, hit = low[k - 2], toks[k - 1], low[k], True
        if sep not in SEPS:
            hit = False
        elif b in ADJECTIVES and a in ci.DICT and a not in VERBS:
            toks[k - 2], toks[k] = ADJ_EN.get(b, toks[k]), toks[k - 2]
        elif b in OF and k + 2 < len(toks) and toks[k + 1] == sep and _noun(a) and _noun(low[k + 2]):
            toks[k - 2:k + 3] = [toks[k + 2], sep, toks[k - 2]]                  # X di Y -> Y X
        elif a == "da" and re.search(r"(are|ere|ire)$", b):
            toks[k - 2] = "TO" if toks[k - 2].isupper() else "to"                 # da sistemare -> to fix
        elif a in HEADS and _noun(b) and b not in NOT_COMPLEMENT and not (k + 2 < len(toks) and low[k + 2] in ADJECTIVES):
            toks[k - 2], toks[k] = toks[k], toks[k - 2]
        else:
            hit = False
        low, fired = [t.lower() for t in toks], fired or hit
        k += 4 if hit else 1                                                     # a swapped pair is not swapped again
    return toks, fired


def translate_component(comp: str) -> tuple[str, bool]:
    """English form of one path component, and whether a word-order rule changed it."""
    toks, fired = reorder(re.findall(r"[A-Z]?[a-zà-ÿ]+|[A-Z]+(?![a-z])|[^A-Za-zÀ-ÿ]+", comp))
    out = []
    for tok in toks:
        w = tok.lower()
        en = ci.DICT.get(w, "") if w not in ci.PROPER else ""
        if en and tok[0].isalpha():
            en = en.replace(" ", "_")
            tok = en.upper() if tok.isupper() and len(tok) > 1 else en.capitalize() if tok[0].isupper() else en
        out.append(tok)
    return "".join(out), fired


def _new_dir(d: str) -> str:
    parts = d.split("/")
    return "/".join(DIR_OVERRIDES.get("/".join(parts[:k + 1]), translate_component(c)[0]) for k, c in enumerate(parts))


def rename_map(files: list[str]) -> list[tuple[str, str, str, str]]:
    """Directories first, then the files whose own name changes (new path after the directory renames). Notes:
    GITHUB (a .github/ file names the old path: renamed only with Federico), COLLISION (two sources, an existing path, or a module
    directory that lands next to another module's file get the same name), MERGE (the new directory exists), REORDERED (a
    word-order rule fired: read it)."""
    existing = set(files)
    existing_dirs = {"/".join(f.split("/")[:k]) for f in files for k in range(1, f.count("/") + 1)}
    rows, targets, new_of = [], Counter(), {}
    for rel in files:
        parts = rel.split("/")
        en, fired = translate_component(parts[-1])
        en = OVERRIDES.get(rel, en)
        new = (_new_dir("/".join(parts[:-1])) + "/" if len(parts) > 1 else "") + en
        new_of[rel] = new
        if en != parts[-1] and not NO_RENAME.search(rel):
            targets[new] += 1
            rows.append(("file", rel, new, "REORDERED" if fired else ""))
    new_files = set(new_of.values())
    dir_rows = []
    for d in sorted(existing_dirs):
        base = d.rsplit("/", 1)[-1]
        en, fired = translate_component(base)
        en = DIR_OVERRIDES.get(d, en)
        if en != base:
            new = _new_dir(d)
            clash = new + ".rs" in new_files and new_of.get(d + ".rs") != new + ".rs"
            dir_rows.append(("dir", d, new, "COLLISION" if clash else "MERGE" if new in existing_dirs else "REORDERED" if fired else ""))
    gh = "\n".join((ci.ROOT / f).read_text(encoding="utf-8", errors="replace") for f in files if f.startswith(".github/"))
    out = []
    for k, o, n, note in dir_rows + rows:
        if k == "file" and (targets[n] > 1 or (n in existing and n != o)):
            note = "COLLISION"
        crate = o.split("/")[1] if k == "dir" and o.count("/") == 1 and o.startswith("crates/") else ""
        names = [o] + ([crate, crate.replace("-", "_")] if crate else [])
        if any(re.search(r"(?<![\w-])" + re.escape(x) + r"(?![\w-])", gh) for x in names):
            note = (note + " GITHUB").strip()
        out.append((k, o, n, note))
    return out


def main() -> int:
    for row in rename_map(ci.tracked()):
        print("\t".join(row))
    return 0


if __name__ == "__main__":
    sys.exit(main())
