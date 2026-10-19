# Tabui, passo «1 · Files»: togliere i file e una riga per campione

- **Togliere** (`renderFiles`/`removeFiles` in `TP_Mine/js/20_tpmine.js`): ogni riga ha `.tp-rm` («Remove <nome>», ≥24 px, 40 al tocco) e accanto a «Choose files...» c'è `#tp-rmall`; togliere non chiede conferma e mostra «Removed … · Undo» (6 s) nel `#tolto-box` dell'app; Annulla rimette le righe al loro posto. I file restano nella scheda Dati.
- **Dopo «Remove all»** il ritorno alla scheda non ricarica da solo i file (`st.noAuto`); «Reload from Data» e «Choose files...» lo riattivano. Durante il Dig togliere è rifiutato.
- **Una riga per campione**: un campione letto da due sorgenti (Dati e scelta dei file, anche con `.mzML` ripetuto) è UNA riga (`fileKey`: nome base senza `.mzML`, minuscolo, in `pageFiles` e `addFiles`) e si mostra una sola volta senza estensione (`fileLabel`).
- **Nome intero**: sta nell'aiuto `TPHELP` del `.tp-fn` (a destra, mai sopra «Conditions of the files»), non in un `title`.
- **Prova**: `e2e_tpmine_file.py` (togli, annulla, togli tutti, niente doppioni, aiuto, axe).
