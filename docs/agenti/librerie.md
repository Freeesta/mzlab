# Librerie spettrali: formato `.mzlib` v1, costruttore, lettore (`crates/mzlab-lib`)
Stato attuale. Il formato è versionato e specificato qui byte per byte; il codice è in `crates/mzlab-lib` (Rust puro, senza `unsafe`, nel grafo WASM solo `ruzstd` e `serde_json`). Le librerie non stanno mai nel repository.

## Struttura del crate
- `parse.rs`: lettori in streaming (`Parser::push(&[u8])` in qualunque spezzettatura, `end()`): MSP, MGF, record MassBank `.txt` (`ACCESSION` … `PK$PEAK:` … `//`), JSON (array MoNA/GNPS oppure oggetti uno dopo l'altro, cioè JSON-lines di mzMine; i picchi possono essere `[[mz,i],…]`, una stringa JSON o testo `mz:i mz:i`). `detect(nome, testa)` sceglie il formato dall'estensione, poi dal contenuto (`lib.err.unsupported` per `.lib/.mzvault/.sqlite/.db/.nist`, `lib.err.unknown` per il resto).
  Regole degli stessi campi di `LibParser` (`libreria-worker.js`): record senza precursore o senza picchi = scartati e contati (`Stats`: `read, dropped, no_prec, no_peaks, broken`), polarità da `Ion_mode`/`polarity`, poi dal segno dell'addotto `]n±`, poi dalla carica; testi tagliati a 2048 byte; righe oltre 16 MiB e oggetti JSON oltre 32 MiB saltati (contati come `broken`); al massimo 200 000 picchi letti per record.
- `clean.rs`: pulizia come Flash entropy: via i picchi ≥ precursore − 1,6 Da, via il rumore < 1% del picco più alto, picchi a meno di 0,05 Da fusi (il più intenso prende i vicini: m/z media pesata, intensità somma), al massimo 500 picchi; `weighted` = pesatura d'entropia di Li 2021 (somma 1; se l'entropia è < 3, esponente 0,25 + 0,25·S).
- `scores.rs`: punteggi spettro-spettro. `pair_scores` (accoppiamento uno a uno entro una tolleranza assoluta, prima i prodotti d'intensità più alti): **entropia** pesata di Li 2021 (`entropy`), **coseno pesato** (coseno delle radici delle intensità, `cosine`), `forward` e `reverse` (nomenclatura NIST: forward = tutti i picchi del campione e solo i picchi accoppiati della libreria, reverse = il contrario), picchi in comune (`shared`), frazione d'intensità del campione spiegata (`explained`); `common_losses` = perdite neutre (precursore − m/z) in comune;
  `modified_cosine` = «coseno modificato» delle analoghe (i 32 picchi più intensi, esclusi la regione del precursore ± 1,5 e il rumore < 1%; un picco si accoppia anche alla differenza dei precursori); `hybrid_pairs` = accoppiamento ibrido a forza bruta (riferimento della ricerca Flash); `gnps_cosine` = **coseno GNPS** (regole sotto). «coseno pesato» e «coseno GNPS» sono punteggi diversi e hanno nomi diversi nei risultati. `mzlab-core::entropy` resta il riferimento a forza bruta dei dati d'oro.
- `search.rs`: `Query::new(prec, pol, mz, it)` pulisce la richiesta come le voci della libreria. `identity`: gli spettri della polarità giusta (quelli senza polarità si tengono) con precursore nella finestra (`Tol::Ppm` o `Tol::Da`) sono un intervallo contiguo di indici; ogni candidato si legge e si valuta con `pair_scores` (migliore entropia prima; `dprec` in ppm o Da), perdite neutre comuni solo per i primi.
  `flash_scores(lib, q, Open|NeutralLoss|Hybrid, tol, intervalli)`: per ogni picco della richiesta si leggono dagli indici le sole voci nella finestra di m/z (e di perdita neutra), il contributo di una coppia è `pair_term(a, b)` (la somma sulle coppie è la similarità d'entropia), le coppie in conflitto si risolvono uno a uno per contributo decrescente; la tabella leggera (`Library::light`: `scale` e precursore arrotondato di ogni spettro, 8 byte per spettro) si costruisce alla prima ricerca.
  `analogs`: ricerca ibrida nella finestra ±`max_delta` (200 Da), esclusi |Δ| < 0,5 Da e meno di 3 coppie; i 300 migliori ricevono il coseno modificato (≥ 4 picchi accoppiati), ordinati per quello.
- `decoy.rs` (decoy): `make_decoy(spettro, DecoyKind::Fragments|Precursor, seme)` = frammenti con m/z pseudo-casuali nell'intervallo dello spettro (intensità conservate) oppure precursore spostato di 5-50 Da; seme = FNV-1a di accessione, nome, InChIKey e precursore con un seme globale (xorshift64* scritto a mano, mai l'ordine o l'orologio). `with_decoys` = bersagli + un decoy ciascuno, marcati dal prefisso `DECOY:` nell'accessione e indicizzati dal `Builder` come voci normali; `best_scores` = miglior entropia bersaglio e decoy di una richiesta (lo stesso `identity`).
- `decoy.rs` (q-value): `lib_fdr`/`qvalues`: FDR(s) = decoy ≥ s / bersagli ≥ s, q = minimo cumulativo, pari merito con lo stesso valore (il risultato non dipende dall'ordine); `LibFdr { q_value, n_target_ge, n_decoy_ge, decoy_kind }`; `search_fdr` = lotto di richieste con q; `fdr_note()` = chiave `lib.fdr.note`. Prove: `tests/fdr.rs` (+ `tests/golden/lib/qvalues.json`), `tests/test_librerie_fdr_wasm.py` (stessi byte nativo e `.wasm`).
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
Directory: `u32 n_blocchi`, `u32` 0, poi `n_blocchi` voci da 40 byte; i dati dei blocchi seguono (allineati a 8). Voce: `u64` posizione dal inizio della sezione, `f64` precursore del primo spettro (solo tabella spettri), `u32` byte memorizzati, `u32` byte decompressi, `u32` indice del primo elemento, `u32` CRC32C dei byte memorizzati, `u8` metodo (0 grezzo, 1 zstd), `u8` chiave di polarità del primo spettro (solo tabella), 2 byte a 0, `u32` riservato.
Il lettore controlla: numero di blocchi atteso, primo elemento = indice × elementi per blocco, offset dentro la sezione, byte decompressi sotto il massimo del tipo, CRC prima di decomprimere, lunghezza esatta dopo.

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
Blocchi da 512 spettri, zstd. Contenuto decompresso di un blocco con P picchi: quattro piani di byte delle differenze di m/z (u32, 1e-5 Da; il primo picco di ogni spettro è assoluto, gli altri sono differenze strettamente positive), poi due piani di byte delle intensità (u16 = intensità pulita / massimo × 65535, almeno 1). Totale 6·P byte. m/z massimo 40 000 Da; errore di quantizzazione ≤ 5·10⁻⁶ Da (0,005 ppm a m/z 1000).

### Metadati (tipo 3)
Blocchi da 128 spettri, zstd; per ogni spettro, nell'ordine della tabella, 10 testi: ciascuno `varint` lunghezza + byte UTF-8: nome, addotto, energia di collisione, strumento, formula, InChIKey, SMILES, accessione, autori, licenza (testo). Si leggono solo per i candidati mostrati.

### Indici Flash (tipi 4 frammenti e 5 perdite neutre)
Voci da 10 byte: m/z (u32, 1e-5 Da; per le perdite neutre: precursore − m/z), indice dello spettro (u32), intensità pesata d'entropia (u16 = p / `scale` × 65535, almeno 1), ordinate per m/z e poi per spettro. Una voce di perdita neutra con valore < 1e-5 Da non si scrive.
Directory (allineata a 8): `u64 n_voci`, `u32 n_pagine`, `u32 voci_per_pagina` (6553), poi per pagina `u32` primo m/z, `u32` numero di voci, `u32` CRC32C dei byte della pagina. Pagine in slot da 65536 byte, ciascuna con `count` m/z (u32), `count` spettri (u32), `count` intensità (u16), poi zeri; solo l'ultima può avere meno di 6553 voci e il file finisce dopo i suoi byte usati. Una finestra di m/z [lo, hi] legge le sole pagine tra `partition_point(primo m/z < lo) − 1` e l'ultima con primo m/z ≤ hi.

## Costruttore (memoria limitata)
`Builder::add(&Spectrum)`: pulisce, quantizza (picchi sulla stessa unità di m/z fusi) e accoda il record in un flusso di lavoro; in memoria resta solo una chiave per spettro (≈ 32 byte). `finish(&mut Sink)`: ordina le chiavi, rilegge i record in ordine e li scrive in blocchi (picchi direttamente nel file, metadati e tabella in flussi di lavoro poi copiati), mentre le voci degli indici vanno in 128 secchi di m/z da 16 Da (l'ultimo prende il resto) dello stesso file di lavoro; ogni secchio si legge, si ordina e riempie le pagine una alla volta.
Il file di lavoro (`Scratch`) è un `Vec<u8>` nei test, un file sul disco negli esempi, un handle OPFS nel browser. Ordine dei byte del file: testata riservata (256) | picchi | metadati | spettri | indice frammenti | indice perdite neutre | testata e tabella (scritte per ultime).

## Lettore
`Library::open(source, byte_di_cache)` legge testata, tabella e directory (poche centinaia di KB anche per 10⁶ spettri); blocchi e pagine si decodificano a richiesta in una cache LRU con tetto in byte. Il tetto sta in un solo punto (`read.rs`: `CACHE_BYTES` = 64 MB, `CACHE_BYTES_SMALL` = 32 MB per iPad e telefono, `cache_budget(small)`; test in `tests/format.rs`).
API: `rec(i)`, `peaks(i)`/`peaks_q(i)`, `meta(i)`, `ranges(pol, lo, hi)` (intervalli di indici; gli spettri senza polarità si tengono sempre), `for_each_entry(nl, lo, hi, f)` (voci di un indice in una finestra di m/z), `verify()` (ogni CRC, ordine, conteggi: `Ok` solo per un file sano).

## Nel browser (`crates/mzlab-lib-wasm`, `libreria-worker.js`)
Descritto in `docs/agenti/alta-risoluzione.md` (voce Librerie) e `docs/agenti/rust.md`. Risultati dell'identità per voce: `pos`, `name`, `type`, `ce`, `inst`, `formula`, `ik`, `smiles`, `accession`, `authors`, `license` (testo), `lic` (classe), `prec`, `polarity`, `ent`, `cos` (coseno pesato), `forward`, `reverse`, `shared`, `explained`, `nlShared`, `dprec` (ppm o Da), `pairs` (indici nella richiesta pulita `query` e nella libreria), `mz`, `it`;
analoghi: `mcos`, `matched`, `shifted`, `delta`, `coinc`, `hybrid`, `pairs` (con il terzo valore «spostata»), `qa`/`la` (i 32 picchi usati). I testi delle librerie sono dati non fidati: la pagina li mostra solo con `EHt`. `searchAll` (una riga per richiesta: il migliore con `alt`) riporta anche `accession`, `authors`, `license`, `smiles`, `ce`, `inst`, `type` della voce migliore, che le esportazioni di mzFinder citano. Misure e bersagli: `docs/agenti/misure.md`, `python3 tools/bench.py --librerie [--gb]`.

## FDR della ricerca in libreria (`decoy.rs`, `Lib.search_with_fdr`)
- Solo HR e mzFinder, mai in bassa risoluzione, mai sul punteggio composito dell'identità: solo sull'entropia della ricerca in libreria (arrotondata a 1e-4: `quantize`). Una libreria con decoy si costruisce con `with_decoys`; `search_fdr` / `search_with_fdr(precs, pols, starts, mz, it, tol, ppm, frag, kind)` cerca un lotto di richieste e dà per ciascuna `{pos, score, decoy, q, nt, nd, kind, note}` (`LibFdr`: `q_value`, `n_target_ge`, `n_decoy_ge`, `decoy_kind`), con q calcolato sul lotto.
- **Avvertenza**: è un limite inferiore. Gli errori veri sono isomeri e analoghi, non casuali, e i decoy non li imitano: su una libreria sintetica con isomeri, q ≤ 0,001 corrisponde a un errore osservato del 49 %. Il testo si mostra sempre come «FDR stimato della ricerca in libreria (limite inferiore)» (`lib.fdr.note`), mai come probabilità che l'identità sia giusta.

## Coseno GNPS (regole di Attributio, riscritte)
Campione contro riferimento, tolleranza ASSOLUTA in Da (0,01 o 0,02): via i picchi ≥ precursore − 0,5 e quelli < 1% del picco base; intensità in radice quadrata; accoppiamento avido per prodotto decrescente, ogni picco una volta (a parità: prima lo stesso m/z, poi gli indici più alti). `cosine` = somma / (norma del campione × norma del riferimento); `reverse` = somma / (norma dei soli picchi del campione accoppiati × norma del riferimento).
Coseno modificato (`modified = true`): un picco del riferimento a m/z `m` si accoppia anche a un picco del campione a `m + Δ`, `Δ = precursore(campione) − precursore(riferimento)`; `n_shifted` conta le coppie spostate. Il codice di Attributio non è copiato (licenza «MIT draft»).

## Prove
`crates/mzlab-lib/tests/parse.rs` (le stesse attese di `tests/test_libreria.py` più MassBank, MoNA, GNPS, JSON-lines, spazzatura casuale), `tests/format.rs` (andata e ritorno, blocchi e pagine multipli, finestre di precursore, cache dentro il tetto, file troncati a ogni punto, 1500 bit cambiati a caso: ogni modifica è scoperta o innocua, conteggi dichiarati enormi, versione futura, sezioni fuori dal file).
`tests/parita.rs` + `tools/parita_librerie.py` (dipendenze di sviluppo `ms_entropy` e `matchms`, gruppo `parita` di `pyproject.toml`; non servono ai test, solo per rigenerare `tests/golden/librerie/`): `pares.json` e `flash.json` da spettri SINTETICI; senza quantizzazione l'entropia, il coseno pesato, i picchi in comune e il coseno modificato coincidono entro 1e-6 con `ms_entropy`/`matchms` (e con `mzlab-core::entropy` entro 1e-9), attraverso il `.mzlib` (quantizzato, indici Flash) entro 1e-4 con gli stessi primi 10 candidati;
`parita_attributio_ms2.json` (60 coppie sintetiche, dalla ricerca su Attributio) per il coseno GNPS: coseno, reverse, picchi accoppiati e coppie entro 1e-9 (1e-4 quantizzato). Nota: `ms_entropy` 1.5.3 manca nella ricerca ibrida una coppia spostata che coinvolge il primo picco di entrambi gli spettri: il generatore dà a ogni spettro un primo picco proprio. `tests/search.rs` (polarità, finestre, ibrida con coppie in competizione, analoghe).
Validazione privata con `MZLAB_DATI`: `tests/test_librerie_rust.py` (la libreria MassBank del repository dei dati contro le MS2 della serie, come `tests/test_hr_identificazione.py`: il genitore è identificato con forza, i prodotti no, le analoghe ritrovano il genitore con i Δm e i coseni attesi in meno di 100 ms) tramite `cargo run --release -p mzlab-lib --example valida`.
Esempio da riga di comando: `cargo run --release -p mzlab-lib --example build -- IN.msp OUT.mzlib` (tempo, peso, memoria massima).

## Pacchetti pubblici del sito (`tools/librerie_pacchetti.py`)
Costruiti dalla CI di Pages (`pages.yml`, dopo i `.wasm`; l'uscita sta in `site/static/librerie/`, tenuta nella cache di Actions finché non cambiano lo script, `crates/mzlab-lib/src` e i record UniTO), mai nel repository né nel `manifest.json` dell'app desktop. Il `sw.js` non li mette in cache (stanno nella cartella dell'utente).
- Ingresso: la release MassBank-data fissata nel codice (`VERSION`, `ZIP_URL`, `ZIP_SHA256`; un'impronta diversa ferma la costruzione; senza rete lo script avvisa ed esce con 0, il sito esce senza pacchetti). Si tengono solo i record `MS_TYPE MS2`, ionizzazione `ESI*`, `ION_MODE` positivo o negativo, con picchi e licenza `CC0`, `CC BY` o `CC BY-SA` (mai NC, mai `dl-de/by-2-0`; GNPS, MoNA, HMDB e NIST non stanno nel sito, li importa l'utente).
- Uscita: `massbank-pos`, `massbank-neg` (i record concatenati passano dall'esempio `build` di `mzlab-lib`) e `unito-standard` (i 61 record `tests/golden/librerie/massbank_unito/`, CC BY, autori UniTO, polarità mista: la libreria di esempio HR), ciascuno diviso in pezzi da 4 MiB `<id>/NNN.bin` con SHA-256, più `catalogo.json`.
- `catalogo.json` (`formato: "librerie/1"`, `pezzo`): `librerie[]` con `id, nome, versione, sorgente, url_sorgente, licenza` (testo), `licenze` (conteggio per classe), `contributori` (conteggio per cartella MassBank), `polarita` (1, -1, 0), `spettri, picchi, dimensione, sha256` (del file intero), `lettore_min` (dalla testata del `.mzlib`), `attribuzione`, `pezzi[]` (`file, dimensione, sha256`); `esterne[]` con `id, nome, sorgente, licenza, url, formato, dimensione, sha256` (null dove la fonte cambia: GNPS) per il download dell'app desktop.
- Prove: `tests/test_librerie_pacchetti.py` (filtro delle licenze, pezzi, catalogo, impronta sbagliata, rete assente; il costruttore è sostituito da un finto). Attribuzioni in `LICENZE-TERZI.md`.

## La cartella delle librerie (`libreria-cartella.js`, `libreria-worker.js`, `libreria.js`; app: `crates/mzlab-desktop/src/librerie.rs`)
Una sola cartella per utente. App per il computer: cartella nativa (predefinita `~/Documents/mzLab/Librerie`, `%USERPROFILE%\Documents\mzLab\Librerie`, scelta con «Cambia cartella…», percorso e interruttore delle fonti esterne in `config.json` della cartella dati dell'app), letta con `memmap2` dalle rotte `mzlab://localhost/lib/{dir,list,stat,read,put,remove,rename}` (il worker legge a fette con XHR sincrono e scrive a pezzi con POST; `librerie.rs` accetta solo `nome` o `.mzlab/nome`, mai `..`, percorsi assoluti o `\`).
Chrome/Edge: `showDirectoryPicker` una volta, handle in IndexedDB `mzlab-libdir` (chiave `dir`), scelta ricordata in `qqq.lib.cartella`; il permesso `readwrite` si richiede al primo clic o tasto della visita (`grantOnce`) e, finché manca, vale l'archivio del browser. Safari, Firefox e in generale: OPFS `librerie/` (importazione di file o cartella con `webkitdirectory`: si indicizza dal `File` senza copiare l'originale; `navigator.storage.persist()`).
Il worker riceve la cartella con `{cmd: "folder", mode: "opfs"|"fs"|"native", handle|base}` (i messaggi `folder` si eseguono in ordine e gli altri li aspettano); l'adattatore (`handleDir` per gli handle, `nativeDir` per l'app) ha lo stesso interfaccia, così il codice è uno solo.
- Layout: `<nome>.msp|.mgf|.json|.jsonl|.ndjson` (sorgenti, mai modificate), `<nome>.mzlib` (pacchetti o librerie esportate; sidecar `.mzlab/<nome>.pkg.json` `{id, versione, sha256, size, mtime, info}` se scaricati), `.mzlab/<nome>.mzlib` (indice di una sorgente, scritto una volta e letto da tutti i browser e dall'app) con `.mzlab/<nome>.idx.json` `{v, origine: {nome, size, mtime, sha256}, info, bytes}` scritto per ultimo (è il segno che l'indice è completo). L'indice si scrive prima in `.tmp` e poi sostituisce il vecchio in un passo (`replace`).
- Inventario (`CARTELLA.scan`, a ogni apertura della finestra e a ogni `list`): nome, dimensione e data di ogni file; lo SHA-256 (JS in streaming, `Sha256`) solo se dimensione o data sono cambiate.
  Stati: **pronta** (indice intero, origine invariata o ritoccata con lo stesso contenuto; pacchetto che si apre e coincide col sidecar), **da_indicizzare** (sorgente senza indice o cambiata), **aggiornamento** (un pacchetto con sidecar e il catalogo in cache più recente), **danneggiata** (indice mancante o di dimensione diversa, pacchetto troncato o con SHA-256 diverso, `.mzlib` che non si apre): mai un blocco, si reindicizza o si riscarica.
  Gli indici di file non presenti nella cartella (importati con il pulsante o trascinati) sono «importata», pronta finché l'indice è intero. `.txt` solo con importazione esplicita.
- Comandi del worker: `folder`, `list` (`libs`, `voci`, `folder: {mode, name}`), `add` (indicizza un `File` nella cartella; un `.mzlib` si copia), `index` (`name`: una sorgente della cartella), `remove` (di una sorgente toglie solo indice e meta; di un pacchetto il file e il sidecar). Le librerie di `libs` portano `id: "f:<nome>"`; le vecchie librerie dell'archivio OPFS radice o di IndexedDB restano elencate («pronta», «Togli») e cercabili come prima.
- Prove: `tests/test_libreria_cartella.py` (SHA-256, stati, inventario con una cartella in memoria, percorsi), `crates/mzlab-desktop/tests/librerie.rs` (rotte `lib/*`, percorsi, configurazione), `e2e_libreria_cartella` (cartella OPFS e cartella con handle: stati, file ritoccato o cambiato, indice troncato, pacchetto, file importato, finestra) e i passi della cartella nativa di `crates/mzlab-desktop/tests/e2e_shim.py`.

## Download dei pacchetti (`libreria-scarica.js`, `crates/mzlab-desktop/src/scarica.rs`)
Finestra «Librerie» → riquadro «Librerie disponibili». Nessuna richiesta di rete se l'utente non clicca: «Mostra le librerie disponibili» legge il catalogo (`static/librerie/catalogo.json` dal sito stesso, CSP invariata; nell'app lo legge il lato nativo, `lib_catalog`) e lo conserva in `.mzlab/catalogo.json`, così la visita dopo gli stati «Aggiornamento disponibile» si conoscono senza rete.
Elenco con casella, nome, versione, spettri, dimensione, licenza e stato (Non scaricata, Aggiornamento disponibile, Scaricata, Danneggiata) più «Attribuzioni e fonti» (testo e fonte di ogni pacchetto); si scarica solo ciò che l'utente spunta.
- Pezzi (4 MiB nel catalogo, qualunque dimensione per il codice): ognuno si verifica (dimensione e SHA-256) e, se sbagliato, si riscarica (due tentativi in tutto, poi `lib.dl.err.hash`); i pezzi verificati restano in `.mzlab/<id>.NNN.piece`, così un'interruzione (rete, «Annulla», chiusura) riparte dal primo pezzo mancante o sbagliato.
  Il pacchetto si assembla in `.mzlab/<id>.new`, si verifica per intero (dimensione, SHA-256, testata `MZLB` con versione minima del lettore ≤ 1) e solo allora sostituisce `<id>.mzlib` in un passo (`replace`/`rename`): il vecchio resta finché il nuovo non è verificato; poi si scrive il sidecar `.mzlab/<id>.mzlib.pkg.json` e si tolgono i pezzi. Un pacchetto con totale sbagliato non lascia nulla e non tocca il vecchio.
- Worker: `catalog` (`cached`: senza rete, `cat`: dato dall'app), `download` (`ids`, avanzamento `{id, piece, pieces, bytes, total}`, risposta `{done, failed: [{id, key, params}], avail}`), `cancel` (interrompe il `fetch` in corso). Errori: chiavi `lib.dl.err.{http,hash,offline,cancelled,version,catalog,nocatalog,io,host,externalOff}`.
- App: comandi `lib_catalog`, `lib_download(ids)` (eventi `lib-progress`), `lib_download_external(id)`, `lib_cancel`, `lib_get_config`, `lib_set_external`.
  Fonti esterne (lista `esterne` del catalogo: GNPS CC0, MassBank MSP) SOLO con l'interruttore «Scarica librerie pubbliche da fonti esterne» (Impostazioni, `Config::esterne`, spento di base): una GET semplice senza dati dell'utente, solo `https` e solo dai siti di `ALLOWED_HOSTS` (anche a ogni salto dei reindirizzamenti, seguiti a mano), SHA-256 e dimensione controllati se il catalogo li dà (GNPS cambia: allora nessuna impronta fissa); il file va in `<id>.<formato>` nella cartella e la pagina lo indicizza.
- Prove: `tests/test_libreria_scarica.py` (node: pezzi, riscarico, ripresa, totale sbagliato, annulla, catalogo), `crates/mzlab-desktop/tests/scarica.rs` (stesse prove in Rust contro un server locale, più la lista dei siti consentiti), `e2e_libreria_scarica` (catalogo e pacchetti sintetici su `site/static/librerie/`: niente rete prima del clic, pezzo corrotto riscaricato, interruzione e ripresa, aggiornamento con totale sbagliato e poi giusto, stato noto alla visita dopo, annulla).
