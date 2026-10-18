# Current State: Tabui English-Only & Unqualified Formula Audit

## 1. Rules and Guardrails

Tabui (also known as TP Mine / mzFinder) follows four strict rules:

1. **English only in Tabui**: All user interface text, tables, modals, tooltips, generated reports, and exported columns must be strictly in English. No Italian strings are permitted in user-facing components.
2. **Formula never bare**: The word «formula» never appears on its own in column headers, exported files, or user interface labels. It must always be qualified (e.g. `Neutral formula`, `[M+H]+ formula`, `Ion formula`, `neutral_formula`).
3. **No decorative Unicode**: Clean standard characters only; no decorative Unicode symbols in text or labels.
4. **No `lang/` keys for Tabui**: Tabui does not store keys in global `mzlab/web/lang/` catalogs; its explanatory tooltips live exclusively in `TP_Mine/js/40_tabui_testi.js` (`TABUI_T.en`).

The automated guard verifying these rules is run via:
```bash
pytest tests/test_tabui_inglese.py
```

---

## 2. Status of Previously Tracked Items

Most items originally identified during initial audits have been completed:
- **`TP_Mine/data/composti.csv`**: Renamed column `formula` to `neutral_formula` (generated via `tools/genera_composti.py`).
- **`crates/mzlab-feat/src/attributio/review/feature.rs`**: Candidate label updated from `formula` to `neutral formula`.
- **`TP_Mine/data/regole_tp_2.json`**: Transformation profiles translated to English (e.g. `TiO2/UV: ·OH radicals, h+ holes, O2·−`).
- **`TP_Mine/js/52_attributio_banco.js` & `50_attributio.js`**: Renamed `Banco 2b` UI labels and accessibility titles to `Bench 2b`.
- **`TP_Mine/js/40_tabui_testi.js`**: Italian dictionary `T.it` removed completely; only English `T.en` retained.
- **`TP_Mine/js/44_tp_riga.js`**: Help strings registered in English only (`TXT.en`).
- **`mzlab/web/lang/` catalogs**: `tpmine.*` keys removed from global translation dictionaries.
- **Exported verdicts (`TP_Mine/js/20_tpmine.js`)**: `LAB` mapping translates internal verdict tokens (`forte`, `possibile`, `debole`, `progenitore`) to `strong`, `possible`, `weak`, `parent` upon export.
- **Python engine (`TP_Mine/py/`)**: Legacy Python engine files removed from the repository (`ece63651`).

---

## 3. Open Items & Internal Identifiers

The following items represent open tasks or verified internal identifiers across the codebase:

| Location | Scope / Context | Current Text | Target / Status | Notes |
|---|---|---|---|---|
| `TP_Mine/js/34_pipeline_run.js:182` | UI / Table | `p.segno` ('contro', 'pro') | Needs alignment to 'against' / 'supports' | Partially handled with UI fallback formatting. |
| `TP_Mine/js/37_feature_table.js:34` | Internal select values | `value="nessuna"`, `value="somma"`, `value="mediana"` | Internal values for Rust normalisation | Option labels are in English; values are internal backend parameters. |
| `TP_Mine/data/regole_tp_2.json:3` | Rule profile key | `"predefinito": "fotocatalisi"` | Internal JSON configuration | Profile key referenced by `crates/mzlab-feat`. |
| `crates/mzlab-feat/src/export.rs:186` | Internal JSON keys | `{"hojas":[{"nombre":"Features","filas":[...]}]}` | Internal serialization for `xlsx.js` | Internal format expected by Excel export generator. |
| `TP_Mine/js/20_tpmine.js:792` | Export metadata keys | `t.Q3_osservato`, `t.RT_attesa_min`, etc. | Internal object keys | Column headers in exported Excel files are already English. |

---

## 4. Guard Verification Command

To test compliance across Tabui files, run:
```bash
python3 -m pytest tests/test_tabui_inglese.py
```
