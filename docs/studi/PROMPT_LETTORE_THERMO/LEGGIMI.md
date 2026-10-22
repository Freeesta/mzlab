# Prompt dei pacchetti del lettore Thermo «migliore di quello del produttore»

Studio, lacune e metodo: `docs/studi/LETTORE_THERMO_PERFETTO.md`. Ogni prompt è autosufficiente: si copia la parte dopo «---». Già in main: T-0 (banco `scandump/1`, comparatore) e T-1/T-2 (catena delle reazioni, attivazione ed energia per reazione).

- [T-3](T-3.md): MSn per livello verificata contro RFR (`unknown_double`, energia per livello), attivazioni ETD/ECD/PQD/UVPD, filtro per passo, SPS/MSX/DIA/FAIMS, mzML
- [T-4](T-4.md): trailer, status log, error log, tune, riga di sequenza uguali a RFR; serie nel tempo (assorbe la parte Thermo di RV-10 e R-02 punto 4)
- [T-5](T-5.md): PXD006054 e gli altri `.raw` che non si aprono, file troncati, versioni non dichiarate, messaggi (assorbe RV-08)
- [T-6](T-6.md): velocità contro RFR sullo stesso Mac

Ordine: T-3, T-4, T-5 in parallelo (file diversi; in comune solo `MODIFICHE-MZLAB.md`, `scandump/main.rs` e `VERIFICATI`, una riga ciascuno: `git merge origin/main` prima di unire); T-6 dopo T-5 (i file che si aprono cambiano il corpus). Priorità se non tutti insieme: T-3, T-5, T-4, T-6. Decisione del Coordinatore: T-4 assorbe la parte Thermo di RV-10 e R-02 punto 4, T-5 assorbe RV-08 (segnati nei loro prompt). Tutti chiedono i dump RFR alla chat locale; senza dump lavorano su file costruiti e dichiarano «non verificato contro RFR».
