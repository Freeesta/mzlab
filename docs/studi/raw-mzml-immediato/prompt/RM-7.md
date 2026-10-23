# RM-7 — Prove esterne e validatori (con la chat locale)

Prompt per una chat DEV: copia tutto ciò che segue la riga «---». Dopo RM-2, RM-3 e RM-4; si ripete dopo ogni PR che cambia i byte.

---

Pacchetto RM-7 del convertitore Thermo `.raw` → mzML di mzLab. Leggi prima `docs/studi/raw-mzml-immediato/prompt/COMUNI.md`, poi in `docs/studi/raw-mzml-immediato/` la sezione E, F8 e F9 di `CRITERI.md` e `RICHIESTE_LOCALE.md`.

**Tocca**:
- `tools/` (script che preparano il pacchetto per la locale e confrontano i risultati: Python ammesso, nessun calcolo nel prodotto);
- `docs/agenti/raw-mzml.md` con i risultati aggregati.

**Non tocca**: il codice del prodotto. Ogni difetto trovato diventa una domanda al Coordinatore, con il file e il numero.

## Passi
1. Prepara per la locale, per ogni file e opzione, il comando `mzlab-cli convert` con il commit e l'SHA-256 atteso dell'uscita. La locale lo ripete e confronta l'impronta (F9 su un'altra macchina).
2. Dopo R-c e R-d raccogli i risultati: validatori (F8), lettori di terzi con array non standard (E6), mzmine e SIRIUS (E7), tempi a cache fredda (E8).
3. Tabella finale V1-V10 e F1-F10 con: macchina, strumento e versione, mediana di 5 giri e impronte.

## Accettazione
Ogni criterio di `CRITERI.md` ha un numero e un esito (passa / fallisce / non misurato con la causa). Nessun numero è omesso.
