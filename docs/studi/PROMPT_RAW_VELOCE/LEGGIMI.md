# Prompt dei pacchetti dello studio «`.raw` veloci»

Un file per pacchetto; misure, obiettivi e tabella delle dipendenze in `docs/studi/RAW_VELOCE.md` (§3-4). Ogni prompt è autosufficiente: si copia la parte dopo «---». Ordine:
1. Ondata 1, insieme: RV-01 (bloccante: il DDA da 595 MB oggi non si apre), RV-02 (correttezza dei centroidi), RV-03 (primo del motore), RV-07, RV-09, RV-10.
2. Ondata 2: RV-04 dopo RV-03; RV-08 dopo RV-01; RV-11 (punti 1-3, 5, 6) dopo RV-02 e RV-03.
3. Catena del motore, in serie (ognuno dopo il merge del precedente, peso del `.wasm` prima → dopo in PR): RV-03 → RV-04 → RV-06 → RV-11 punto 4. RV-06 chiede anche RV-02.
4. Navigazione: i pacchetti P2-P6 di `docs/agenti/mappa-fluida.md` (non duplicati qui).

Ogni prompt ha «File toccati» (con **Motore**: sì/no) e «Dipendenze». File condivisi da chat parallele, con funzioni diverse: `rawlettore.rs` (RV-01, RV-03, RV-10), `sessione.rs` (RV-01, RV-03), `metodo_raw.rs`, `metodo-doc.js`, `lang/{it,en}.js` (RV-09, RV-10); chi unisce per secondo fa `git merge origin/main`.

Scelte di prodotto del Coordinatore (già nei prompt): lista di inclusione in sola lettura nella finestra Metodo (RV-09, punto 2.1); casella numpress spenta di default nell'mzML scaricato (RV-06, punto 4).

- [RV-01](RV-01.md): Memoria e apertura del `.raw`: trailer compatti, indice senza `serde_json::Value`
- [RV-02](RV-02.md): Centroidi del file segnati come profilo
- [RV-03](RV-03.md): Hash in una passata, fuori dal percorso critico
- [RV-04](RV-04.md): Tabella e `.mzidx` con una decodifica, senza il giro testo mzML
- [RV-06](RV-06.md): mzML e centroidi veloci
- [RV-07](RV-07.md): Desktop: profilo `opt-level = 3`
- [RV-08](RV-08.md): `.raw` che opentfraw non apre
- [RV-09](RV-09.md): Tutto il metodo del `.raw` in `metodo/1`
- [RV-10](RV-10.md): Tutto ciò che il `.raw` registra: status log e trailer nel tempo, error log, autocampionatore, eventi
- [RV-11](RV-11.md): Tabui: `.raw` scelti direttamente, indice durante il caricamento, riuso da Dati
