# Prompt residui dello studio «`.raw` veloci»

Misure, obiettivi e dipendenze in `docs/studi/RAW_VELOCE.md` (§3-4). Ogni prompt è autosufficiente: si copia la parte dopo «---». Restano:

- [RV-08](RV-08.md): `.raw` che opentfraw non apre (Exploris, Lumos, trappole lineari, TSQ vecchi); oggi `vendor/opentfraw/src/reader.rs:469` legge e scarta l'ASInfo; obiettivo ≥ 38/41 aperti sui file del privato.
- [RV-09](RV-09.md): tutto il metodo del `.raw` in `metodo/1`; manca fra l'altro la lista di inclusione in sola lettura nella finestra Metodo (`mzlab/web/metodo-doc.js`; la lista è già estratta da `metodo_raw/inclusione.rs`).
- [RV-10](RV-10.md): status log e trailer come serie nel tempo, error log, autocampionatore, Tune; le serie esistono a richiesta (`metodo_raw/serie.rs`, `registro-serie.js`), Tune e autocampionatore non sono letti.

File condivisi da chat parallele: `rawlettore.rs`, `metodo_raw.rs`, `metodo-doc.js`, `lang/{it,en}.js`, `grezzi-non-estratti.md`; chi unisce per secondo fa `git merge origin/main`. Navigazione: pacchetti P2-P6 di `docs/agenti/mappa-fluida.md`.
