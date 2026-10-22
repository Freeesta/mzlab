# RM-8 — Fuzzing e casi limite di ingresso e uscita

Prompt per una chat DEV: copia tutto ciò che segue la riga «---». Dopo RM-4.

---

Pacchetto RM-8 del convertitore Thermo `.raw` → mzML di mzLab. Leggi prima `docs/studi/raw-mzml-immediato/prompt/COMUNI.md`, poi in `docs/studi/raw-mzml-immediato/` `RISPOSTE.md` §3.3 (casi limite) e F7-F9 di `CRITERI.md`.

**Leggi**:
- i moduli `unsafe` di RM-3 e RM-4;
- `vendor/opentfraw/src/scan_data.rs` (`Limiti`);
- i fuzzer esistenti (`grep -rln fuzz crates vendor`).

**Tocca**: bersagli di fuzzing (decodifica del pacchetto, escape XML, scrittore ordinato, `mmap` contro `read`), test dei casi limite e `docs/agenti/raw-mzml.md`. Per un job notturno nuovo passa dal Coordinatore.

**Non tocca**: il comportamento, salvo i difetti trovati (ognuno con il suo test).

## Passi
1. **Pacchetti costruiti** per ogni caso di §3.3 in ingresso:
   - scansione vuota, senza profilo, senza lista;
   - m/z f64, senza fudge, nparam 0/4/5/7;
   - analizzatori misti;
   - indice fuori dal file, troncato, numeri enormi.
   
   Esito atteso: uscita valida oppure `err.file.*`, mai panico.
2. **Uscita**:
   - offset oltre 4 GB (scrittore con un `Write` finto che conta senza scrivere);
   - `defaultArrayLength` 0;
   - array di lunghezze diverse;
   - caratteri di controllo e non ASCII nel filtro e nel trailer;
   - NaN e Inf.
   
   Ogni caso passa il validatore.
3. **Fuzzing** di ogni modulo `unsafe` contro la sua versione sicura, al bit.

## Accettazione
- Nessun panico su 10⁶ ingressi di fuzzing per bersaglio.
- Ogni caso limite ha il suo test.
- Le uscite valide con il validatore nostro e, in un campione, con quelli di R-d.
