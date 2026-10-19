# Prompt residui: Tabui MS2 LR

Studio: `docs/studi/TABUI_MS2_LR.md`. Resta il pacchetto [M1](M1.md) (energia di collisione dei `.wiff` nell'indice `.mzidx` e `<activation>` nell'mzML scritto da mzLab; `crates/mzlab-core/src/wiff/`, `mzmlwrite.rs`). Il resto della serie (finestra attorno al tempo della riga, raggruppamento delle MS2, bersagli, interfaccia, confronto col genitore) è in `main`: `docs/agenti/mzfinder-lr.md` e `mzfinder-ms2.md`.

Il `.wasm` che cambia è il motore (tetto 1 703 936 B); gli altri moduli che lo includono non devono cambiare peso. Dati veri e verità dei test: repository privato `Freeesta/mzlab-dati`, `scratchpad/consegne/TABUI_MS2_LR_DATI.md`.
