# RM-0 — Banco, roofline e caso denso sintetico (nessuna ottimizzazione)

Prompt per una chat DEV: copia tutto ciò che segue la riga «---». Primo pacchetto: gli altri partono dai suoi numeri.

---

Pacchetto RM-0 del convertitore Thermo `.raw` → mzML di mzLab. Leggi prima `docs/studi/raw-mzml-immediato/prompt/COMUNI.md` (regole), poi `docs/studi/raw-mzml-immediato/CRITERI.md`, `MISURE.md` e `RISPOSTE.md` §1.3 e §3.1. Questo pacchetto non cambia nessun byte dell'uscita: costruisce il banco con cui si misurano tutti gli altri.

**Leggi**:
- `crates/mzlab-core/examples/raw_tempi.rs`;
- `crates/mzlab-core/src/lettori_mzml.rs` (`scrivi`, `scrivi_con`);
- `crates/mzlab-core/src/mzmlwrite.rs` e `mzmlwrite/{codifica,pool,thread,testa}.rs`;
- `crates/mzlab-core/src/rawlettore.rs` (`scansione_dati`, `extra_scansione`);
- `crates/mzlab-cli/src/convert.rs`;
- `vendor/opentfraw/src/scan_data.rs` (struttura del pacchetto FT e test che costruiscono pacchetti).

**Tocca**:
- un esempio nuovo `crates/mzlab-core/examples/raw_mzml_banco.rs`;
- un generatore del caso denso in `crates/mzlab-core/examples/` o in un modulo di test;
- `tools/bench/raw_mzml.py` (lancia e raccoglie, nessun calcolo);
- `docs/agenti/raw-mzml.md` (nuovo) e la riga in `docs/agenti/INDICE.md`.

**Non tocca**: lo scrittore, il lettore e le uscite. Le impronte dell'mzML restano uguali.

## Passi
1. **Banco per fase**: apertura, decodifica, `extra_scansione`, XML, base64, compressione, SHA-1 della testata, SHA-1 dell'uscita, scrittura.
   - Per ogni fase: tempo, CPU, RSS di picco (`getrusage` o `/proc/self/status` nel nativo) e numero di allocazioni (allocatore contatore nell'esempio).
   - Con 1/2/4/N thread e livelli 0/1/6.
   - Uscita JSON come in `MISURE.md` §4.
2. **Micro-banchi da soli** (roofline): decodifica di un pacchetto FT, conversione m/z, formattazione dell'XML di uno spettro, base64, miniz L1 con reset e zlib-rs con flusso riusato (feature `zlib-rs` di flate2, solo nell'esempio), SHA-1, dd in lettura e scrittura. Da questi calcola T_min con la formula di `RISPOSTE.md` §1.3 e scrivilo per LAB-A, LAB-B, BAF e il caso denso.
3. **D11**: scomponi `mzlab-cli convert` (5,97 s su LAB-B contro 2,87 s dello scrittore nel banco): SHA-256 per la cache, indice, copia/collegamento, chiave `conv/1`. Tabella con le cause.
4. **D8**: spiega perché 3 thread sono più lenti di 1 (callgrind per thread, `perf stat` con cache-misses e banda se c'è). Ipotesi da confermare o scartare: memset del `reset` per array e produttore seriale.
5. **Caso denso sintetico**: un generatore deterministico (seme fisso) di pacchetti FT veri nel formato di `scan_data.rs`.
   - Profilo ≥ 200 000 punti per scansione, con pezzi, fudge, lista e descrittore coerenti; totale ≥ 2 GB.
   - Lo esegue un `Lettore` finto che serve le scansioni dal file generato a offset, con la stessa decodifica del vero.
   - Il contenitore `.raw` non serve: il suo costo si misura sui file veri.
   - Una variante da 8 GB per V6, generata e cancellata nel test.
6. **BAF**: misura V3 (URL e SHA-256 in `dati/formati/_grandi_thermo_aperti.tsv` del privato) se il file è raggiungibile, altrimenti chiedilo al Coordinatore.

## Accettazione
- `docs/agenti/raw-mzml.md` ha la tabella di partenza di V1-V9 per LAB-A, LAB-B, BAF e il denso, con T_min, macchina e profilo, e le cause di D8 e D11 scritte con i numeri.
- Il banco è ripetibile: due giri danno mediane entro il 5 %.
- Il generatore dà lo stesso SHA-256 a ogni giro.
- `cargo test -p mzlab-core` e `tools/pre_pr.py` verdi; impronte dell'mzML invariate.

## Prove
- Il generatore produce pacchetti che il decoder di `scan_data.rs` rilegge con i valori generati: test al bit su 3 scansioni.
- Il conto delle allocazioni di `raw_tempi` riproduce il ~10 % di malloc/free misurato da callgrind (`MISURE.md` §2), con un ordine di grandezza coerente.
