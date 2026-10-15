# Prompt dei pacchetti dello studio «`.raw` veloci»

Un file per pacchetto; misure, obiettivi e tabella delle dipendenze in `docs/studi/RAW_VELOCE.md` (§3-4). Ogni prompt è autosufficiente: si copia la parte dopo «---». Ordine:
1. Subito, in parallelo: RV-01 (bloccante: il DDA da 595 MB oggi non si apre), RV-02 (correttezza dei centroidi), RV-07, RV-08, RV-09.
2. Dopo RV-01: RV-03, poi RV-04; RV-05 in parallelo con RV-03/RV-04.
3. Dopo RV-02: RV-06.
4. Navigazione: dopo RV-05, i pacchetti P2-P6 di `docs/agenti/mappa-fluida.md` (non duplicati qui).

Domande di prodotto aperte al Coordinatore: lista di inclusione nell'interfaccia (RV-09, punto 2.1); numpress come scelta dell'mzML scaricato (RV-06, punto 4). Il resto dei due pacchetti non le aspetta.

- [RV-01](RV-01.md): Memoria e apertura del `.raw`: trailer compatti, indice senza `serde_json::Value`
- [RV-02](RV-02.md): Centroidi del file segnati come profilo
- [RV-03](RV-03.md): Hash in una passata, fuori dal percorso critico
- [RV-04](RV-04.md): Tabella e `.mzidx` con una decodifica, senza il giro testo mzML
- [RV-05](RV-05.md): Decodifica a blocchi e trasporto binario tra modulo e motore
- [RV-06](RV-06.md): mzML e centroidi veloci
- [RV-07](RV-07.md): Desktop: profilo `opt-level = 3`
- [RV-08](RV-08.md): `.raw` che opentfraw non apre
- [RV-09](RV-09.md): Tutto il metodo del `.raw` in `metodo/1`
