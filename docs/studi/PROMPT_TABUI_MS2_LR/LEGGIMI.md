# Prompt Tabui MS2 LR — ordine dei pacchetti

Studio: `docs/studi/TABUI_MS2_LR.md`. Ogni file `M*.md` è un prompt pronto per una chat DEV: si copia la parte dopo «---» (le regole comuni sono in fondo a ciascuno).

| Pacchetto | Contenuto | Dipende da | In parallelo con |
|---|---|---|---|
| M1 | CE dei `.wiff` nell'indice e nell'mzML | — | M2 |
| M2 | Motore `ms2_links/1`, `relink`, worker, esempio `--ms2` | — | M1 |
| M3 | Bersagli MS2 e diagnosi degli ioni che il Dig non elenca | M2 | M4 |
| M4 | Scheda «MS2 matches», correzioni, memoria, Excel, e2e | M2 | M3 |
| M5 | Similarità riga ↔ genitore in LR, decisione D6 | M4 | — |

Ordine: M1 ∥ M2 → M3 ∥ M4 → M5. M1 non blocca nessuno: senza CE le MS2 si raggruppano per (polarità, precursore) e due CE dello stesso precursore si fondono (sui file del corso c'è una sola CE). Le decisioni D1-D6 (§9 dello studio) hanno un default (a) con cui i prompt sono scritti; se il Coordinatore sceglie diversamente, si corregge il prompt prima di lanciarlo.

Dati veri e verità dei test: repository privato `Freeesta/mzlab-dati`, `scratchpad/consegne/TABUI_MS2_LR_DATI.md` e `scratchpad/tabui_ms2_lr_truth.json`.
