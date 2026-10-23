# Prompt dei pacchetti del lettore Thermo «migliore di quello del produttore»

Studio, lacune e metodo: `docs/studi/LETTORE_THERMO_PERFETTO.md`. Ogni prompt è autosufficiente: si copia la parte dopo «---». Già in main: T-0 (banco `scandump/1`, comparatore) e T-1/T-2 (catena delle reazioni, attivazione ed energia per reazione).

- [T-3](T-3.md): MSn per livello verificata contro RFR (`unknown_double`, energia per livello), attivazioni ETD/ECD/PQD/UVPD, filtro per passo, SPS/MSX/DIA/FAIMS, mzML
- [T-4](T-4.md): trailer, status log, error log, tune, riga di sequenza uguali a RFR; serie nel tempo (assorbe la parte Thermo di RV-10 e R-02 punto 4)
- [T-5](T-5.md): PXD006054 e gli altri `.raw` che non si aprono, file troncati, versioni non dichiarate, messaggi (assorbe RV-08)
- [T-5b](T-5b.md): l'ultimo file del corpus letto male dopo T-5 (R-06): LTQ Orbitrap XL v63, 4 scansioni su 1141, eventi non letti. Chiude quando ogni file che RFR apre si apre anche da noi con le stesse scansioni, oppure ha una causa scritta.
- [T-6](T-6.md): velocità contro RFR sullo stesso Mac (assorbito da T-7)
- [T-7](T-7.md): velocità e zero perdite, a fasi T-7a…e: lettore fedelissimo (registro dei byte, andata e ritorno, impronte; MB/s e scansioni/s contro RFR, ThermoRawFileParser, msconvert, mzmine) e ponte senza perdite verso Flow sui centroidi (colonne, matrice scansione × bin; `.raw` → feature table contro mzmine)

Ordine: T-3, T-4, T-5 in parallelo (file diversi; in comune solo `MODIFICHE-MZLAB.md`, `scandump/main.rs` e `VERIFICATI`, una riga ciascuno: `git merge origin/main` prima di unire); T-7 dopo T-3 e T-5 (i file che si aprono cambiano il corpus), una fase per volta, T-7a prima di ogni codice. Priorità se non tutti insieme: T-3, T-5, T-5b, T-4, T-7. Chat DEV nuove: Sonnet.
Decisione del Coordinatore: T-4 assorbe la parte Thermo di RV-10 e R-02 punto 4, T-5 assorbe RV-08, T-7 assorbe T-6 (segnati nei loro prompt); un tetto di `.wasm` può salire fino a +10 % se dichiarato. Tutti chiedono i dump RFR alla chat locale; senza dump lavorano su file costruiti e dichiarano «non verificato contro RFR».
