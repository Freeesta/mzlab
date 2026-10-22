# Regole comuni dei prompt RM (valgono per tutta la chat DEV)

Ogni prompt RM-n dice di leggere questo file per primo. Insieme formano il prompt completo.

## Ruolo e consegna
- Sei una chat DEV di mzLab (repository `Freeesta/mzlab`) sul convertitore Thermo `.raw` → mzML. Lo studio è in `docs/studi/raw-mzml-immediato/`: leggi `CRITERI.md` (i numeri che devi raggiungere) e le righe di `MISURE.md` e `DUBBI.md` che il tuo prompt nomina. Non rileggere `RISPOSTE.md` intero: solo i paragrafi citati.
- Coordinatore = mz COO 11 (`session_01SqVGjScVGPMmoMETWKcX2F`). A fine lavoro mandagli UN messaggio con `send_message`: ≤ 5 righe con PR, SHA di 40 caratteri a CI verde e cosa resta.
- Se sei bloccato o hai una scelta: PR aperta con la spiegazione e domanda al Coordinatore con una raccomandazione. Mai unire dalla chat.
- Ramo `claude/raw-mzml-<argomento>` da `origin/main`; `tools/fondi_main.py` solo se in conflitto; `tools/pre_pr.py` completo prima di spingere.
- Commit e PR in inglese. Eventi della PR: skill `steward`.
- Contesto oltre 200k token a fine PR: blocco di consegna al Coordinatore e fermati.
- AGENTS.md è nel tuo contesto. Per il codice: `python3 tools/indice.py`, `grep -n`, subagente `cercatore`.

## Zero perdite prima della velocità
- Nessuna regola di velocità supera F1-F7 di `CRITERI.md` né A1-A4 di `docs/studi/PROMPT_LETTORE_THERMO/T-7.md`.
- Ogni ottimizzazione lascia invariate le impronte: SHA-256 dell'mzML per le opzioni che la PR non cambia (F10), `raw_equivalenza.rs`, `raw_parity.rs`, `msntree_parity.rs`.
- Dove i byte cambiano: prima/dopo su un file nella PR e versione `conv/1` aumentata.
- **Determinismo (F9)**: stessi byte con 1/N thread, su ogni macchina e in `.wasm`. Niente FMA implicita né funzioni libm nella formula m/z; niente orologio nei byte.

## Due bersagli
- **Nativo** (CLI, app desktop): senza tetti di `.wasm`, con `mmap` (`memmap2`), `rayon`, SIMD `std::arch` dietro controllo della CPU all'avvio e la via scalare uguale al bit.
- **Browser** (`.wasm`): con i tetti. Modulo `thermo` 546 304 B (`tools/build_lettori_wasm.sh`, `tests/test_lettori_wasm.py`, `docs/agenti/wasm-tetti.md`): fino a +10 % è ammesso se dichiarato nella PR con il prima/dopo e aggiornato nei tre file. Il `.wasm` del motore non cresce (`cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm`, prima e dopo). SharedArrayBuffer non c'è (GitHub Pages senza COOP/COEP): più worker con intervalli di scansioni.
- Ogni numero dice a quale bersaglio appartiene.

## Dati molto densi (regole del Coordinatore)
- **Memoria a flusso**: mai l'intero file né tutti i picchi in RAM. Il budget è dichiarato e controllato (V6). Se uno stadio deve tenere più del budget, usa lo scratch esistente (`Spill` in `crates/mzlab-core/src/mzidx_spill.rs`), non un meccanismo nuovo.
- **Buffer riusati**: nessuna allocazione per scansione a regime (V7, allocatore contatore nel banco).
- **Layout piatto e allineato** per SIMD: array di m/z e intensità contigui, non vettori di struct.
- **Back-pressure** tra gli stadi, in byte, non in numero di spettri: canali limitati, mai code illimitate.
- **Nucleo scalabile** da 1 a N thread con lo stesso codice; 1 thread è il caso del browser.
- **Misure**: oltre ai file veri, sul caso denso sintetico di RM-0 (≥ 2 GB, ≥ 200 000 punti per scansione). Picco di memoria dichiarato per ogni fase.

## Codice e dipendenze
- Solo Rust nel prodotto. Python solo in `tools/` e nei test di riferimento; JS solo interfaccia.
- **`unsafe`** solo in moduli isolati e piccoli, ognuno con:
  - un test;
  - il fuzzing (`cargo fuzz` o equivalente nel CI notturno; un workflow nuovo solo passando dal Coordinatore);
  - una versione sicura accanto, che un test confronta al bit sulle stesse entrate.
- **Ogni guadagno conta**, anche di pochi punti percento: è ammesso se misurato oltre il rumore (5 giri, mediana, scarto interquartile dichiarato, stesso risultato ripetendo) e il codice resta piccolo e leggibile. In ogni PR: il guadagno in % sul passo e sul totale, il rumore di misura e una riga nella tabella cumulativa `docs/agenti/misure-velocita.md` (chi la trova assente la crea, con la riga in `docs/agenti/INDICE.md`).
- **Dipendenza nuova** solo con un guadagno misurato ≥ 3 % sul suo bersaglio (sotto il 3 % si scarta o si riscrive senza dipendenza) e una licenza permissiva (MIT, Apache-2.0, BSD, Zlib). Nella PR: le righe aggiunte di `Cargo.toml`/`Cargo.lock`, la licenza e i numeri prima/dopo. Già nel `Cargo.lock`: `memmap2`, `rayon`, `crossbeam-channel`, `itoa`, `sha1`, `base64`, `flate2` con `miniz_oxide` e `zlib-rs`.
- **Contratto `Lettore`**: ogni numero letto dal file passa da `Limiti` prima di allocare; gli errori sono chiavi `err.file.*`, mai panico; ciò che il file non ha è assente.
- Ogni modifica a `vendor/opentfraw` è una riga in `vendor/opentfraw/MODIFICHE-MZLAB.md` (Apache-2.0).
- Ogni opzione nuova del passo `convert.mzml` ha il suo `--set conv.NOME=VALORE` (regola CLI, `docs/agenti/cli.md`).

## Oracoli e dati
- **RFR** (RawFileReader) e msconvert: solo sul Mac nella chat locale, fuori da ogni repository. Si confrontano solo le uscite (dump `scandump/1` in `mzlab-dati/scratchpad/rfr/`, mzML di msconvert). Nel repository nessuna DLL, codice o testo di RFR; mai decompilare.
- Se un dump manca: lavora su file costruiti, scrivi «non verificato contro RFR» e chiedi al Coordinatore la richiesta R-x di `RICHIESTE_LOCALE.md`.
- **Dati**: solo nel privato `Freeesta/mzlab-dati`, con il risolutore unico (`tools/dati_percorsi.py`/`.rs`); i test con file veri saltano senza `MZLAB_DATI`.
- Nel pubblico solo file costruiti e aggregati: nessun nome di file privato (usa LAB-A, LAB-B, BAF o l'accession) né cognome di `nomi_vietati.txt`.

## Documenti
- Il file d'area è `docs/agenti/raw-mzml.md`: lo crea RM-0, con una riga in `docs/agenti/INDICE.md`. Stato attuale, righe ≤ 600 caratteri, file ≤ 25 600 B.
- Ogni PR aggiorna lì le righe dei criteri che tocca (misura, macchina, profilo di compilazione) e la riga di `CRITERI.md` «Oggi», se cambia.
- Niente «ora…», numeri di PR o date nei documenti d'area.

## Subagenti
- Haiku solo per lavoro meccanico. Mai per decisioni, codice nuovo, asserzioni dei test, commit, push.
