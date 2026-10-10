# Librerie spettrali: formato `.mzlib` v1, costruttore, lettore (`crates/mzlab-lib`)
Stato attuale. Il formato è versionato e specificato qui byte per byte; il codice è in `crates/mzlab-lib` (Rust puro, senza `unsafe`, nel grafo WASM solo `ruzstd` e `serde_json`). Le librerie non stanno mai nel repository.

## Struttura del crate
- `parse.rs`: lettori in streaming (`Parser::push(&[u8])` in qualunque spezzettatura, `end()`): MSP, MGF, record MassBank `.txt` (`ACCESSION` … `PK$PEAK:` … `//`), JSON (array MoNA/GNPS oppure oggetti uno dopo l'altro, cioè JSON-lines di mzMine; i picchi possono essere `[[mz,i],…]`, una stringa JSON o testo `mz:i mz:i`). `detect(nome, testa)` sceglie il formato dall'estensione, poi dal contenuto (`lib.err.unsupported` per `.lib/.mzvault/.sqlite/.db/.nist`, `lib.err.unknown` per il resto). Regole degli stessi campi di `LibParser` (`libreria-worker.js`): record senza precursore o senza picchi = scartati e contati (`Stats`: `read, dropped, no_prec, no_peaks, broken`), polarità da `Ion_mode`/`polarity`, poi dal segno dell'addotto `]n±`, poi dalla carica; testi tagliati a 2048 byte; righe oltre 16 MiB e oggetti JSON oltre 32 MiB saltati (contati come `broken`); al massimo 200 000 picchi letti per record.
- `clean.rs`: pulizia come Flash entropy: via i picchi ≥ precursore − 1,6 Da, via il rumore < 1% del picco più alto, picchi a meno di 0,05 Da fusi (il più intenso prende i vicini: m/z media pesata, intensità somma), al massimo 500 picchi; `weighted` = pesatura d'entropia di Li 2021 (somma 1; se l'entropia è < 3, esponente 0,25 + 0,25·S).
- `build.rs`: `Builder` a memoria limitata (sotto). `read.rs`: `Library<S: Source>`. `io.rs`: `Source` (lettura a fette), `Sink` (scrittura a offset), `Scratch` (entrambe), `ExtentStore` (flussi accodati in un solo file di lavoro), `ReadSeekSource`. `codec.rs`: blocchi zstd (`ruzstd`; il compressore è `Fastest`, un blocco che non si riduce resta grezzo). `format.rs`: costanti e record a dimensione fissa. `error.rs`: ogni errore è una chiave i18n con parametri, mai un testo.
- Errori (chiavi): `lib.err.corrupt` (`what`: parte danneggiata o file troncato → reindicizzare), `lib.err.version` (`found`, `need`), `lib.err.archive` (non è un `.mzlib`), `lib.err.tooLarge` (`what`), `lib.err.noSpectrum` (`dropped`), `lib.err.io`, `lib.err.unsupported`, `lib.err.unknown`, `error.memoryBudget`. Mai un panico su dati dichiarati falsi: ogni lunghezza e ogni offset letti dal file si confrontano con la dimensione del file prima di allocare, le allocazioni grandi usano `try_reserve`.

## Il file `.mzlib` v1
Tutto little-endian; ogni sezione comincia a un multiplo di 8 byte. Mai nel repository.

### Testata (36 byte, offset 0)
| Offset | Tipo | Campo |
|---|---|---|
| 0 | 4 byte | magic `MZLB` |
| 4 | u16 | versione del formato (1) |
| 6 | u16 | versione minima del lettore (1): un lettore più vecchio rifiuta con `lib.err.version` |
| 8 | u32 | numero di spettri (1 … 2³¹) |
| 12 | u32 | numero di sezioni (1 … 64) |
| 16 | u64 | numero totale di picchi |
| 24 | u32 | bandierine (0) |
| 28 | u32 | riservato (0) |
| 32 | u32 | CRC32C dei byte 0..32 |

Segue la tabella delle sezioni: per ogni sezione 32 byte, poi un u32 con il CRC32C di testata + tabella. Le prime sezioni cominciano a 256 (`HEAD_RESERVED`). Una sezione di tipo sconosciuto si ignora; mancare di una delle cinque note è `corrupt`.

| Offset | Tipo | Campo della voce |
|---|---|---|
| 0 | u32 | tipo: 1 spettri, 2 picchi, 3 metadati, 4 indice dei frammenti, 5 indice delle perdite neutre |
| 4 | u32 | riservato (0) |
| 8 | u64 | posizione della sezione nel file |
| 16 | u64 | lunghezza della sezione |
| 24 | u32 | lunghezza della directory in testa alla sezione |
| 28 | u32 | CRC32C della directory (che contiene i CRC32C di ogni blocco o pagina: così tutto il file è coperto, salvo i byte di allineamento) |

### Sezioni a blocchi (spettri, picchi, metadati)
Directory: `u32 n_blocchi`, `u32` 0, poi `n_blocchi` voci da 40 byte; i dati dei blocchi seguono (allineati a 8). Voce: `u64` posizione dal inizio della sezione, `f64` precursore del primo spettro (solo tabella spettri), `u32` byte memorizzati, `u32` byte decompressi, `u32` indice del primo elemento, `u32` CRC32C dei byte memorizzati, `u8` metodo (0 grezzo, 1 zstd), `u8` chiave di polarità del primo spettro (solo tabella), 2 byte a 0, `u32` riservato. Il lettore controlla: numero di blocchi atteso, primo elemento = indice × elementi per blocco, offset dentro la sezione, byte decompressi sotto il massimo del tipo, CRC prima di decomprimere, lunghezza esatta dopo.

### Spettri (tipo 1)
Blocchi da 4096 record da 32 byte, grezzi. Ordine: chiave di polarità (0 negativa, 1 non nota, 2 positiva), poi precursore, poi posizione nel file d'origine. Un intervallo di precursori in una polarità è quindi un intervallo contiguo di indici (ricerca binaria).
| Offset | Tipo | Campo |
|---|---|---|
| 0 | f64 | m/z del precursore |
| 8 | u32 | indice del primo picco nel flusso globale dei picchi |
| 12 | u16 | numero di picchi (≤ 500) |
| 14 | i8 | polarità (+1, −1, 0) |
| 15 | u8 | licenza: 0 non nota, 1 CC0, 2 CC BY, 3 CC BY-SA, 4 CC BY-NC, 5 CC BY-NC-SA, 6 altra (il testo esatto sta nei metadati) |
| 16 | u16 | bandierine: bit 0 = meno di 3 picchi |
| 18 | u16 | riservato |
| 20 | u32 | posizione dello spettro nel file d'origine |
| 24 | f32 | `scale`: il massimo delle intensità pesate d'entropia (le intensità degli indici sono u16 frazioni di questo) |
| 28 | u32 | riservato |

### Picchi (tipo 2)
Blocchi da 4096 spettri, zstd. Contenuto decompresso di un blocco con P picchi: quattro piani di byte delle differenze di m/z (u32, 1e-5 Da; il primo picco di ogni spettro è assoluto, gli altri sono differenze strettamente positive), poi due piani di byte delle intensità (u16 = intensità pulita / massimo × 65535, almeno 1). Totale 6·P byte. m/z massimo 40 000 Da; errore di quantizzazione ≤ 5·10⁻⁶ Da (0,005 ppm a m/z 1000).

### Metadati (tipo 3)
Blocchi da 1024 spettri, zstd; per ogni spettro, nell'ordine della tabella, 10 testi: ciascuno `varint` lunghezza + byte UTF-8: nome, addotto, energia di collisione, strumento, formula, InChIKey, SMILES, accessione, autori, licenza (testo). Si leggono solo per i candidati mostrati.

### Indici Flash (tipi 4 frammenti e 5 perdite neutre)
Voci da 10 byte: m/z (u32, 1e-5 Da; per le perdite neutre: precursore − m/z), indice dello spettro (u32), intensità pesata d'entropia (u16 = p / `scale` × 65535, almeno 1), ordinate per m/z e poi per spettro. Una voce di perdita neutra con valore < 1e-5 Da non si scrive.
Directory (allineata a 8): `u64 n_voci`, `u32 n_pagine`, `u32 voci_per_pagina` (6553), poi per pagina `u32` primo m/z, `u32` numero di voci, `u32` CRC32C dei byte della pagina. Pagine in slot da 65536 byte, ciascuna con `count` m/z (u32), `count` spettri (u32), `count` intensità (u16), poi zeri; solo l'ultima può avere meno di 6553 voci e il file finisce dopo i suoi byte usati. Una finestra di m/z [lo, hi] legge le sole pagine tra `partition_point(primo m/z < lo) − 1` e l'ultima con primo m/z ≤ hi.

## Costruttore (memoria limitata)
`Builder::add(&Spectrum)`: pulisce, quantizza (picchi sulla stessa unità di m/z fusi) e accoda il record in un flusso di lavoro; in memoria resta solo una chiave per spettro (≈ 32 byte). `finish(&mut Sink)`: ordina le chiavi, rilegge i record in ordine e li scrive in blocchi (picchi direttamente nel file, metadati e tabella in flussi di lavoro poi copiati), mentre le voci degli indici vanno in 128 secchi di m/z da 16 Da (l'ultimo prende il resto) dello stesso file di lavoro; ogni secchio si legge, si ordina e riempie le pagine una alla volta. Il file di lavoro (`Scratch`) è un `Vec<u8>` nei test, un file sul disco negli esempi, un handle OPFS nel browser. Ordine dei byte del file: testata riservata (256) | picchi | metadati | spettri | indice frammenti | indice perdite neutre | testata e tabella (scritte per ultime).

## Lettore
`Library::open(source, byte_di_cache)` legge testata, tabella e directory (poche centinaia di KB anche per 10⁶ spettri); blocchi e pagine si decodificano a richiesta in una cache LRU con tetto in byte. Il tetto sta in un solo punto (`read.rs`: `CACHE_BYTES` = 64 MB, `CACHE_BYTES_SMALL` = 32 MB per iPad e telefono, `cache_budget(small)`; test in `tests/format.rs`). API: `rec(i)`, `peaks(i)`/`peaks_q(i)`, `meta(i)`, `ranges(pol, lo, hi)` (intervalli di indici; gli spettri senza polarità si tengono sempre), `for_each_entry(nl, lo, hi, f)` (voci di un indice in una finestra di m/z), `verify()` (ogni CRC, ordine, conteggi: `Ok` solo per un file sano).

## Prove
`crates/mzlab-lib/tests/parse.rs` (le stesse attese di `tests/test_libreria.py` più MassBank, MoNA, GNPS, JSON-lines, spazzatura casuale), `tests/format.rs` (andata e ritorno, blocchi e pagine multipli, finestre di precursore, cache dentro il tetto, file troncati a ogni punto, 1500 bit cambiati a caso: ogni modifica è scoperta o innocua, conteggi dichiarati enormi, versione futura, sezioni fuori dal file). Esempio da riga di comando: `cargo run --release -p mzlab-lib --example build -- IN.msp OUT.mzlib` (tempo, peso, memoria massima).
