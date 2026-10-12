# Metodo di acquisizione (`metodo/1`)

Stato attuale. Il metodo di UN campione (strumento, sorgente, esperimenti, LC, rivelatori, registro) come un solo JSON, qualunque sia il file di partenza. Schema: `mzlab/web/schemi/metodo-1.json`. Codice: `crates/mzlab-core/src/metodo.rs` (albero, adattatore `.dam`, SHA-1, `userParam`) e `metodo_wiff.rs` (adattatore `.wiff`). Vale il principio «niente risposte»: il documento riporta ciò che il file dice, senza verdetti.

## Regole del documento
- Un valore sconosciuto è ASSENTE (mai `null`, mai 0); le sezioni vuote sono omesse; le unità stanno nel nome del campo (`flusso_mL_min`, `ce_eV`, `iniezione_uL`).
- Sezioni: `schema` (= `metodo/1`), `origine` (formato, file con nome/SHA-1/byte, numero del campione, `file_creato` = ora locale del PC dello strumento, senza fuso; `lettore`), `campione`, `strumento`, `operatore`, `ms` (`sorgente`, `periodi[].esperimenti[]` con `parametri` e `transizioni`), `lc` (`gradiente`, `flusso_mL_min`, `forno_C`, `pressione_max`), `campionatore`, `rivelatori`, `registro`, `grezzo` (testo del Log ≤ 256 KB), `avvisi` (chiave i18n + `dove`: `meta.nonDecodificato`, `meta.mancante`).
- I parametri di `ms` sono ordinati per id (l'ordine nel file è un accidente dello scrittore): lo stesso metodo letto da `.dam` e da `.wiff` dà lo stesso `ms`. `tipo` di un esperimento è «MRM» solo se ci sono transizioni; il nome del file non è una prova.
- Opzione anonima (`anon=1`): tolti `operatore`, `strumento.serie`, utente, computer, percorsi e numero di serie anche dal testo grezzo del Log.

## Adattatori
- `.dam`: `from_dam` (metodo dell'esperimento; LC e rivelatori dall'XML del metodo LC).
- `.wiff`: `from_wiff(&Ole, campione, &Source)`. Da `SampleSubtree/SampleN/{Log, SampleDABE/DATA, SampleDABE/CFR_INFO}` e da `SummaryInformation` (proprietà 12 = creazione, FILETIME).
  Il tipo di sorgente (`ms.sorgente.tipo`, p. es. «Turbo Spray») è il «Source Type» del Log del `.wiff`; un `.dam` da solo non lo dice, quindi manca. Produttore e modello dello strumento si leggono dal Log («Config Table»), mai da un valore scritto nel codice: se il file non li dà, il campo è assente.
  Misurato sui file del 3200: modello e seriale (record «Config Table» del Log), utente e computer (record `driver:User=`), pompe e campionatore (voci del driver), inizio/fine corsa.
  DA CONFERMARE (si ricavano dalla posizione nei flussi `DATA`/`CFR_INFO`, senza etichetta nel file): `campione.nome`, `campione.vial`, la versione di Analyst. Il DAD (`DADRealTimeData`) non è decodificato: avviso `meta.nonDecodificato`.
- mzML: `userParam name="mzLab metodo/1"` nel `<run>` con il documento (XML escape); `from_userparam` lo rilegge. Senza di esso l'mzML non ha metodo.
- Un `.wiff` senza `.wiff.scan` non si apre come dati ma dà il suo metodo.

## Abbinare `.wiff` e mzML
Gli mzML di ProteoWizard hanno in `<sourceFile>` un cvParam SHA-1 (MS:1000569) uguale a quello del `.wiff` (il nome nell'mzML può differire dal file vero); il campione viene dal `nativeID` `sample=N`. `mzml_sources` legge il primo MB dell'mzML; `sha1_file` calcola l'impronta del `.wiff`. Un `.wiff` caricato da solo con mzML già aperti con lo stesso SHA-1 dà a ciascuno il suo documento (nota `load.note.wiffMethod`); senza abbinamento resta l'errore `err.file.wiffNoScan`.

## Rotta e motore
- GET `/api/metodo?name=<file>[&anon=1]` → `{"source":"mzml|wiff|dam|none","metodo":{…}}` (op `lr_metodo` di `rust-metodo.js`, chiamata da `rust-worker-lab.js`).
- Motore (`crates/mzlab-wasm/src/metodo.rs`): `sha1_file`, `mzml_sources`, `mzml_metodo`, `wiff_metodo` (legge il `.wiff` intero, rifiuta oltre 192 MB con `memory_budget`), `lab_metodo`.
- Test: `crates/mzlab-core/tests/metodo_parity.rs` (dati veri, `MZLAB_DATI`): `ms` del `.wiff` uguale al `.dam` nominato, documento anonimo senza identità, SHA-1 uguale a quello dell'mzML di riferimento; `tests_e2e/e2e_metodo.py` (rotta e flusso del `.wiff` solo).
