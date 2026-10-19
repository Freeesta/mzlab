# Prompt: Tabui, feature table dei TP con MS2, posizione e disegno

Studio: `docs/studi/TABUI_TP_TIMOLOLO.md`. Una chat per zona (regola 25), PR in fila dentro la chat:
- [LOCALE](LOCALE.md): chat locale sul Mac (`mz-loc`), dati e oro; si manda con `send_message`, in sola lettura salvo i file nuovi del privato in `coordinamento/consegne/`.
- [R](R.md): chat DEV Rust (Sonnet), pacchetti TT-R1, TT-R2, TT-R3 in quest'ordine.
- [U](U.md): chat DEV pagina (Sonnet), TT-U1 in due PR (scheda MS2 dopo TT-R1, scheda Structure dopo TT-R2).

Ordine: LOCALE e R partono insieme; U parte quando TT-R1 è in `main`; TT-R3 aspetta i file di LOCALE. Tetti dei `.wasm`: comune 512 000 B (`interp` nuovo), `chem` 569 344 B (non si tocca), motore 1 703 936 B, `sciex` 327 680 B. Numeri veri: solo nel privato (`coordinamento/consegne/TABUI_TP_TIMOLOLO_DATI.md`, `scratchpad/timolol_truth.json`, `scratchpad/timolol_msn_truth.json`).
