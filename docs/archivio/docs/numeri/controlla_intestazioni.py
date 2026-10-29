#!/usr/bin/env python3
r"""Verifica delle intestazioni e del formato dei dati nell'uscita CLI di mzLab (Track T6).

Controlla tre regole di qualità e conformità per i file CSV prodotti dalla CLI:
- Regola (a): Nessuna intestazione deve chiamarsi esattamente "formula" (case-insensitive).
              Ogni colonna di formula molecolare deve specificare "Neutral formula"
              oppure l'addotto tra parentesi quadre (es. "[M+H]+ formula").
              Se non è presente alcuna colonna formula, lo stato è "NON ANCORA".
- Regola (b): Le aree dei picchi (colonne matching '.*:area') devono essere in notazione
              scientifica d.dde+N (r"^[+-]?\d+(\.\d+)?[eE][+-]?\d+$") e non devono
              terminare con suffissi come M/k/G. Se la PR che introduce la notazione
              scientifica nella CLI non è ancora confluita in main, lo stato è "NON ANCORA".
- Regola (c): Nessun carattere '_' nei nomi leggibili di eventuali colonne di modifica
              (es. etichette di modifiche o prodotti di trasformazione TP). Se non è
              presente alcuna colonna di modifiche, lo stato è "NON ANCORA".

Utilizza esclusivamente la libreria standard di Python (csv, re, sys, pathlib).
"""

import csv
from pathlib import Path
import re
import sys

# Pattern per notazione scientifica esponenziale (es. 2.293075e+06, 1.5e-04, 3e5)
SCI_NOTATION_PATTERN = re.compile(r"^[+-]?\d+(\.\d+)?[eE][+-]?\d+$")

# Pattern per numero decimale/intero standard senza esponente (es. 2293075.4)
STD_FLOAT_PATTERN = re.compile(r"^[+-]?\d+(\.\d+)?$")

# Pattern per suffissi alfabetici non ammessi (es. M, k, G)
FORBIDDEN_SUFFIX_PATTERN = re.compile(r"[a-df-zA-DF-Z]$")


def verifica_regola_a(headers: list[str]) -> tuple[str, str]:
    """Verifica la Regola (a): divieto di 'formula' nuda; formula neutra o con addotto."""
    bare_formula_found = False
    formula_cols = []

    for h in headers:
        norm = h.strip().lower()
        if norm == "formula":
            bare_formula_found = True
            formula_cols.append((h, False))
        elif "formula" in norm:
            # Deve contenere "neutral formula" o "[addotto] formula" (es. [M+H]+ formula)
            is_valid = bool(
                "neutral formula" in norm
                or "formula neutra" in norm
                or re.search(r"\[.+\][0-9]*[+-]?\s*formula", norm)
                or re.search(r"formula\s*\[.+\][0-9]*[+-]?", norm)
            )
            formula_cols.append((h, is_valid))

    if bare_formula_found:
        return (
            "VIOLATA",
            "Trovata colonna con nome vietato 'formula' senza specificatore di stato o addotto.",
        )

    if formula_cols:
        invalid = [h for h, valid in formula_cols if not valid]
        if invalid:
            return (
                "VIOLATA",
                f"Trovate colonne formula non conformi: {', '.join(invalid)}.",
            )
        valid_cols = [h for h, valid in formula_cols if valid]
        return (
            "RISPETTATA",
            f"Tutte le colonne formula sono conformi: {', '.join(valid_cols)}.",
        )

    return (
        "NON ANCORA",
        "Nessuna colonna relativa a formule molecolari presente nell'output CLI attuale.",
    )


def verifica_regola_b(headers: list[str], rows: list[dict[str, str]]) -> tuple[str, str]:
    """Verifica la Regola (b): notazione scientifica d.dde+N per aree e assenza di suffissi M/k/G."""
    area_cols = [
        h for h in headers
        if re.search(r"(:|^)area$", h.strip(), re.IGNORECASE)
        or h.strip().lower().endswith(":area")
        or h.strip().lower() == "area"
    ]

    if not area_cols:
        return (
            "NON ANCORA",
            "Nessuna colonna di area dei picchi (es. '.*:area') presente nell'output.",
        )

    values = []
    for col in area_cols:
        for r in rows:
            raw_v = r.get(col, "").strip()
            if raw_v:
                values.append((col, raw_v))

    if not values:
        return (
            "NON ANCORA",
            f"Colonne di area presenti ({', '.join(area_cols)}), ma nessun dato registrato.",
        )

    has_forbidden_suffix = False
    all_scientific = True
    all_standard_float = True

    for col, v in values:
        if FORBIDDEN_SUFFIX_PATTERN.search(v):
            has_forbidden_suffix = True
        if not SCI_NOTATION_PATTERN.match(v):
            all_scientific = False
        if not STD_FLOAT_PATTERN.match(v):
            all_standard_float = False

    if has_forbidden_suffix:
        return (
            "VIOLATA",
            f"Rilevati suffissi non numerici vietati (es. M/k/G) nelle aree ({', '.join(area_cols)}).",
        )

    if all_scientific:
        samples = [v for _, v in values[:3]]
        return (
            "RISPETTATA",
            f"Aree in {', '.join(area_cols)} in notazione scientifica d.dde+N (es. {', '.join(samples)}).",
        )

    if all_standard_float:
        samples = [v for _, v in values[:3]]
        return (
            "NON ANCORA",
            f"Aree attualmente in formato float standard (es. {', '.join(samples)}) senza suffissi M/k/G. "
            "La PR per la notazione scientifica d.dde+N in mzlab-cli non è ancora confluita in main.",
        )

    return (
        "VIOLATA",
        f"Valori delle aree non conformi in {', '.join(area_cols)}.",
    )


def verifica_regola_c(headers: list[str], rows: list[dict[str, str]]) -> tuple[str, str]:
    """Verifica la Regola (c): nessun carattere '_' nei nomi leggibili di colonne di modifica."""
    mod_cols = []
    for h in headers:
        norm = h.strip().lower()
        if any(term in norm for term in ["modification", "modifica", "transformation", "prodotto_trasformazione", "tp_label", "tp_name"]):
            mod_cols.append(h)

    if not mod_cols:
        return (
            "NON ANCORA",
            "Nessuna colonna di modifica o annotazione TP presente nell'output CLI attuale.",
        )

    bad_headers = [h for h in mod_cols if "_" in h]
    bad_values = []
    for col in mod_cols:
        for r in rows:
            val = r.get(col, "").strip()
            if "_" in val and val not in bad_values:
                bad_values.append(val)

    if bad_headers or bad_values:
        details = []
        if bad_headers:
            details.append(f"intestazioni con '_': {', '.join(bad_headers)}")
        if bad_values:
            details.append(f"valori con '_': {', '.join(bad_values[:3])}")
        return (
            "VIOLATA",
            f"Rilevato carattere '_' in nomi leggibili di modifica ({'; '.join(details)}).",
        )

    return (
        "RISPETTATA",
        f"Colonne di modifica presenti ({', '.join(mod_cols)}) e prive di caratteri '_'.",
    )


def analizza_file(csv_path: Path):
    if not csv_path.exists():
        print(f"ERRORE: File non trovato: {csv_path}", file=sys.stderr)
        sys.exit(1)

    with csv_path.open(encoding="utf-8") as f:
        reader = csv.reader(f)
        try:
            headers = next(reader)
        except StopIteration:
            print(f"ERRORE: File CSV vuoto: {csv_path}", file=sys.stderr)
            sys.exit(1)

    with csv_path.open(encoding="utf-8") as f:
        dict_reader = csv.DictReader(f)
        rows = list(dict_reader)

    res_a = verifica_regola_a(headers)
    res_b = verifica_regola_b(headers, rows)
    res_c = verifica_regola_c(headers, rows)

    controlli = [
        (
            "Regola (a)",
            "Intestazione formula (divieto 'formula' nuda; ammessa neutra o [Addotto])",
            res_a[0],
            res_a[1],
        ),
        (
            "Regola (b)",
            "Formato aree picchi (.*:area in scientifico d.dde+N; no suffissi M/k/G)",
            res_b[0],
            res_b[1],
        ),
        (
            "Regola (c)",
            "Nomi modifiche/TP (nessun carattere '_' nei nomi leggibili)",
            res_c[0],
            res_c[1],
        ),
    ]

    sep_len = 120
    print("=" * sep_len)
    print(f"VERIFICA FORMATTAZIONE E INTESTAZIONI CLI: {csv_path.name}")
    print(f"Percorso file: {csv_path}")
    print("=" * sep_len)
    header_line = (
        f"{'Regola':<12} | {'Descrizione':<40} | {'Stato':<14} | {'Dettaglio / Motivazione'}"
    )
    print(header_line)
    print("-" * sep_len)
    for reg, desc, stato, dett in controlli:
        print(f"{reg:<12} | {desc:<40} | {stato:<14} | {dett}")
    print("-" * sep_len)

    n_rispettata = sum(1 for _, _, s, _ in controlli if s == "RISPETTATA")
    n_non_ancora = sum(1 for _, _, s, _ in controlli if s == "NON ANCORA")
    n_violata = sum(1 for _, _, s, _ in controlli if s == "VIOLATA")

    print(
        f"Riepilogo: {len(controlli)} verifiche eseguite "
        f"({n_rispettata} RISPETTATA, {n_non_ancora} NON ANCORA, {n_violata} VIOLATA)."
    )

    if n_violata > 0:
        sys.exit(1)
    sys.exit(0)


def main():
    if len(sys.argv) > 1:
        target = Path(sys.argv[1]).resolve()
    else:
        target = Path(__file__).resolve().parent / "uscita_cli" / "features.csv"

    analizza_file(target)


if __name__ == "__main__":
    main()
