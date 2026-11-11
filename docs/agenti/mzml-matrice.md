# Matrice di conformità mzML

Fixture: `tools/mzml_varianti.py OUT_DIR` (20 file sintetici, numeri inventati); prova: `crates/mzlab-cli/src/mzml_variants_tests.rs` (`canali`, `features`, `validate` su ogni variante; salta senza python3 + numpy). Le fixture Numpress si scrivono con `MzmlWriter` (`Opzioni::numpress`), non da Python. Lettura = `mzlab-core` (tabella `scantable`) per l'mzML semplice, lettore `mzml-numpress` (`aperti`) per Numpress: `lettori-aperti.md`.

| Variante | Fixture | Esito (CLI) |
|---|---|---|
| mzML semplice, indicizzato (`indexedmzML`), offset assurdi | `plain_*`, `indexed`, `bigoffsets` | si apre; la tabella delle scansioni legge le teste agli offset dell'indice (un indice che non torna col file si ignora e il file si percorre), il motore (`mzml::read`) legge in sequenza |
| float 32/64 bit × senza compressione/zlib | `plain_32_*`, `plain_64_*` | si apre |
| CRLF, BOM UTF-8 | `crlf`, `bom` | si apre |
| MS2, MS3, MS4 | `ms3_4`, `ms3_only_idx` | si apre; il tipo proposto da `canali` è `lc-ms2-mirato` |
| profilo / centroidi | `profile` | si apre |
| mobilità ionica (array `MS:1002893`) | `mobility` | si apre; l'array extra non è usato |
| `referenceableParamGroup`, `scanSettings` | `rpg`, `scansettings` | si apre; la tabella delle scansioni rilegge i parametri del gruppo dove sta il riferimento (`spechead`) |
| cromatogrammi TIC, BPC, SRM, SIM, assorbimento | `chroms_all` | si apre; tipo proposto `misto` |
| MS-Numpress linear/pic/slof, con e senza zlib | `Opzioni::numpress`, `open_numpress*.rs` | si apre dal lettore `aperti` (sito, app, CLI) |
| file vuoto, non XML | `empty`, `notxml` | `err.file.notMzml` |
| base64 rotta | `badb64` | `err.file.base64` |
| gzip (`.mzML.gz`) | `gz.mzML.gz`, `open_gz.rs` | si apre dal lettore `mzml-gz` (`open/mzml_gz.rs`): decomprime in memoria fino a `Limiti::byte` (`error.memoryBudget` oltre), poi lo legge `mzml-numpress`; gzip troncato dopo qualche spettro si apre con quelli che ha (`mancanti` = `declared` meno le righe), gzip rotto o troncato prima del primo byte `err.file.zlib`; sul sito si gonfia a flusso su OPFS (`gz-flusso.js`, `lettori-aperti.md`) |
| file troncato | `truncated` | si apre con gli spettri che ha; `Table::declared` (conteggio di `<spectrumList>`) meno le righe lette dà `stats.missing` e la riga del caricamento dice «N scansioni mancanti» (`conv.row.missing`); la CLI scrive `warn.file.truncated` su stderr (anche per i file letti da `aperti`: `.mzML.gz`, Numpress; sul sito il modulo restituisce il conteggio con la conversione e `lr.js` lo somma a quello dell'indice); `validate` lo segnala INVALID |
| mzMLb, imzML | – | non letti |

Non provato qui: il motore `.wasm` del sito e l'app desktop sulle stesse fixture (stesso codice del core, ma senza e2e per variante).

## Misure (file grande sintetico, 700 scansioni × 100 000 punti, profilo)
`tools/mzml_grande.py` scrive il file zlib (893 MB); `mzlab-cli convert x.mzML.gz --compression smallest|standard` ne fa i gemelli; `mzlab-cli canali FILE` costruisce l'indice (CLI nativa, release, 4 CPU). Tempi e picco di memoria: Standard 1003 MB, 20,5 s, 109 MB; Smallest (zlib + Numpress) 180 MB, 26,9 s, 67 MB; scrittura Standard 43 s / 940 MB, Smallest 35 s / 916 MB (il `.gz` si gonfia in memoria). Numpress è con perdita; il dato è sintetico (rumore e picchi inventati), il rapporto reale dipende dal file.

Testa di uno spettro o cromatogramma (`spechead`): un solo lettore, oggi usato dalla tabella delle scansioni, dà i termini in ordine di file (spettro, `scanList`, scansioni), i precursori in ordine di file (finestra, primo ione selezionato, attivazione), i prodotti (mai un precursore) e i termini degli array; un `referenceableParamGroupRef` ripete i parametri del gruppo dove sta, il primo termine di un'accession vince, nomi senza prefisso di namespace, CDATA come testo, il tempo in ogni unità (`UO`) diventa minuti.
Limiti di `spechead`: una lista tiene al più 4096 termini e un elemento 64 stadi (un file ostile non riempie la memoria). Prove: `spechead/tests.rs`, `scantable/tests_variants.rs`.

Scrittura (`mzmlwrite/text.rs`): ogni testo letto da un file passa da `esc` (`& < > "` e i caratteri fuori dalla produzione `Char` di XML 1.0 diventano U+FFFD), anche i nomi dei termini CV.
Un termine generico con un testo del file (`GENERIC`: `MS:1000031` e i modelli dei produttori, `MS:1000531` software, `MS:1000008`/`MS:1000443`/`MS:1000026` tipo di sorgente, analizzatore, rivelatore) tiene il nome CV; il testo va in un `userParam` (`instrument model` per i modelli, altrimenti il nome CV), riletto da `mzmlhead/params.rs`, `scantable/analyzer.rs` e `open/mzml_header.rs`.
`tests/test_mzml_secondo_parser.py` legge con lxml (libxml2, senza recupero) l'mzML di `convert` di ogni famiglia sintetica, in Standard e Smallest.

Validatore (`mzml_validate`): prima un passaggio XML 1.0 rigoroso (`wellformed.rs`: caratteri fuori da `Char`, attributi malformati o ripetuti, `<` in un valore, entità sconosciute, contenuto fuori dalla radice; si ferma al primo, codice `xml`). Ogni mzML scritto da una conversione passa da `check_written`: un errore è `err.file.mzmlInvalid` e il file non resta (`convert.mzml` della CLI, `nat2mzml` dell'app, «Scarica .mzML» del sito con il modulo `valida`: `CONV.valid`).
