# Ottimizzazione di `mzlab-core` e del ponte `mzlab-wasm`

Studio di peso, velocità, memoria e qualità del motore, senza cambiare il comportamento. Ogni numero viene da una misura su questo codice (`origin/main` a `7325171`), salvo dove è scritto «stima». Complementare a `docs/studi/FORMATI_NATIVI_MS.md` (i lettori nuovi in un modulo a parte): qui si libera spazio dentro il motore, là si evita di occuparlo.

## 0. Sintesi
- Il motore pesa 1 703 705 B in locale (1 703 607 B nella CI: la differenza di 98 B sono i percorsi del registro di Cargo scritti nei messaggi dei panic, diversi tra `/root/.cargo` e `/home/runner/.cargo`). Tetto 1 703 936 B.
- **Il singolo intervento che conta: gli ordinamenti della libreria standard pesano ~197 KB (11,5 % del `.wasm`)**. Sostituendo quelli «freddi» con un ordinamento compatto non generico, il `.wasm` scende a **1 563 674 B (−140 031 B, misurato)**; tutti i 225 test di `mzlab-core` passano. Sostituendoli tutti si arriva a 1 518 538 B (−185 167 B), ma la costruzione dell'indice rallenta: i punti caldi restano con l'ordinamento standard.
- Le leve di compilazione sono già al massimo: `opt-level="z"`, `lto`, `codegen-units=1`, `panic="abort"` (`Cargo.toml`). `opt-level="s"` dà 2 014 234 B (+310 KB). Un `wasm-opt` recente (binaryen 132 contro 112) dà solo −4…−5 KB.
- Velocità: il lettore mzML spreca un quarto del tempo in `decode_array` (copia carattere per carattere del base64, azzeramento del buffer zlib, decompressore nuovo per ogni array). Un prototipo di 30 righe porta la lettura di un mzML di 101 MB da 1,58 s a 1,18 s (**−25 %**), con `.mzidx` identico byte per byte.
- I moduli «a richiesta» dentro il motore (statistica ~85 KB, «Da dove viene questo ione?» ~112 KB, conversione in centroidi ~20 KB) non servono finché il punto 1 non è esaurito: ogni modulo nuovo ricopia 150-250 KB di libreria standard, JSON e formattazione.

## 1. Come si misura (riproducibile nel container cloud)
Strumenti: `cargo install wasm-bindgen-cli --version <versione di Cargo.lock> --locked`, `cargo install twiggy --locked`, `npm install --prefix $D/wo wasm-opt@1.4.0` (lo stesso di `tools/wasm_opt.sh`). Tutto si installa in ~3 minuti.
```
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm
wasm-bindgen --target web --no-typescript --out-dir $D/w target/wasm32-unknown-unknown/release/mzlab_wasm.wasm
F="--enable-bulk-memory --enable-sign-ext --enable-mutable-globals --enable-nontrapping-float-to-int"
$D/wo/node_modules/.bin/wasm-opt -Oz $F -o $D/o.wasm $D/w/mzlab_wasm_bg.wasm && stat -c %s $D/o.wasm      # il numero del tetto
$D/wo/node_modules/.bin/wasm-opt -Oz $F --debuginfo -o $D/n.wasm $D/w/mzlab_wasm_bg.wasm                    # con i nomi, solo per twiggy
twiggy top -n 60 $D/n.wasm; twiggy top -n 100000 -f csv $D/n.wasm > $D/all.csv; twiggy dominators $D/n.wasm > $D/dom.txt
```
I valori di `twiggy` sul file con i nomi sono in byte di codice reali (la sezione dei nomi, 567 KB, è una riga a parte e sparisce nel file pubblicato).
Velocità nativa: `cargo build --release --example carico --example mzidx -p mzlab-core`, poi `target/release/examples/carico FILE.mzML` e `target/release/examples/mzidx FILE.mzML OUT.mzidx`. File grande: `python3 tools/dati_sintetici.py DIR` e si replicano 25 volte gli spettri di `HR_DDA-Exploris-t30.mzML` (19 800 spettri, 101 MB). Profilo: `CARGO_PROFILE_RELEASE_DEBUG=1` e `valgrind --tool=callgrind` (installato nel container) sul file da 4 MB.

## 2. Dove va il peso (1,70 MB dopo `wasm-opt -Oz`)
| Componente | Byte | Nota |
|---|---:|---|
| `core::slice::sort` (stable 144,5 K, shared 39,4 K, unstable 12,9 K) | 196 800 | ~80 istanze, una per tipo **e** per chiusura; 55 K solo su `usize` (ordinamenti di indici con chiusure diverse) |
| corpi delle esportazioni inlineati da `wasm-opt` («shim») | 251 527 | non è colla: sono le funzioni del ponte inlineate; nessuna perdita |
| `mzlab_core` (nomi propri) | 658 939 | rotte 63 K, `rawlettore` 58 K, `agilent` 58 K, `statroute` 42 K, `composition` 29 K, `mzidx` 26 K, `wiff` 21 K, `raw` 20 K… |
| dati statici | 137 540 | ~54 K di testo: chiavi i18n, chiavi JSON, termini CV, nomi; ~10 K di percorsi dei panic; il resto tabelle (zlib, SHA, Unicode, masse) |
| `alloc` (Vec, String, BTree) | 68 653 | |
| iteratori e `Option`/`Result` monomorfizzati | ~60 000 | `filter_map` 17 K, `map` 15 K, `filter` 13 K: normali per Rust idiomatico |
| `quick_xml` 27 K, `miniz`+`flate2` 29 K, `sha1`+`sha2` 14 K, `base64` 8 K, `dlmalloc` 7 K | 85 000 | necessari |
| `hashbrown` 18 K + `RandomState`/SipHash 17 K | 35 000 | `HashMap` std in 14 file, compreso `vendor/opentfraw` |
| formattazione (`core::fmt` 21 K, float→testo 17 K) | 38 000 | servono: i JSON devono scrivere i float come Python |

Peso trattenuto in esclusiva (ciò che sparirebbe togliendo la sola funzione; limite inferiore, da `twiggy dominators`):
| Famiglia | Byte (esclusivi) | Dettaglio |
|---|---:|---|
| Thermo `.raw` | ~125 000 | `rawlettore::apri` 53,7 K, `raw::metodo_sha1` 32,0 K, `intestazione` 14,1 K, `RunHeader::read` 6,6 K, `testata_mzml` 5,9 K, `scansione` 4,1 K, `scan_filter` 3,9 K |
| «Da dove viene questo ione?» + formule | ~112 000 | esportazione `engine_origin` (comprende `composition::compose` 23,8 K e `isoscore`) |
| Agilent `.D` | ~90 000 | `apri` 27,3 K, `engine_agilent_m_metodo` 19,5 K, `metodo` 10,7 K, `cromatogramma` 10,6 K, `points` 9,0 K |
| Statistica (`api/stat/*`) | ~85 000 | `statroute::pca` 38,8 K, `reference` 32,0 K, `profile` 6,5 K, filtri e matrice ~5 K |
| SCIEX `.wiff` | ~25 000 | `apri` 9,1 K, cromatogrammi, scansione, testata, metodo |
| Conversione in centroidi | ~20 000 | `engine_mzml_to_centroids` |
| Mappa | ~43 000 | `engine_map_route`: resta (funzione centrale) |

## 3. Interventi
Ordine consigliato: P1 e P2 subito e in parallelo (file diversi); P6 dopo P1 (stessi file); gli altri solo se servono.
| # | Intervento | Guadagno | Rischio | Dipende da | Prova di equivalenza |
|---|---|---|---|---|---|
| **P1** | Ordinamenti compatti nei punti freddi di `mzlab-core` (`sortc`), std solo nei punti caldi; guardia di clippy contro i nuovi `sort*` | **−140 031 B misurati** (−8,2 %); anche i `.wasm` di mzFinder, che linkano `mzlab-core`, scendono | basso: stesso ordine (ordinamenti stabili); 6 punti con `partial_cmp` da controllare per i NaN | — | `cargo test --release -p mzlab-core` con `MZLAB_GOLDEN_STRICT=1`; `sito-rust.yml`; il `.wasm` sotto 1 564 000 B |
| **P2** | Lettore mzML: `decode_array` senza copie inutili, zlib senza azzeramento, una sola larghezza, nomi come byte | **−25 % misurato** sulla lettura (1,58 → 1,18 s, 101 MB); stima −30…−35 % con le altre due parti; anche `upload::prepare` e il costruttore dell'indice, che usano `mzml::parse` | medio: stessi errori (`err.file.zlib`, `error.memoryBudget`) sui file rotti e sulle bombe zlib | — | `golden.rs`, `mzidx_parity.rs`, `robustezza_mzml.rs`; `.mzidx` di 5 file sintetici identico con `cmp` prima/dopo; TIC di `carico` identico |
| P3 | Hasher deterministico (`FxHasher` scritto in 15 righe) al posto di `RandomState` in `mzlab-core`, `mzlab-wasm` e `vendor/opentfraw` | stima −5…−12 KB; se `opentfraw` resta con `RandomState` il guadagno è piccolo | basso: nessuna uscita dipende dall'ordine di una `HashMap` (da controllare per ogni iterazione) | — | test interi + `cmp` dei `.mzidx`; abbandonare se il guadagno misurato è < 5 KB |
| P4 | Indice `.mzidx`, caso «tutti i picchi in memoria» (`mzidx.rs:334`): ordinamento per secchi di 1 Da (già contati) e poi dentro ogni secchio | stima −10 % sulla costruzione (l'ordinamento è il 16 % delle istruzioni); costo: +50 % di memoria transitoria della sezione | medio (memoria su iPad) | — | `mzidx_parity.rs` + `cmp`; misura di `examples/mzidx` (tempo e `peak memory`) |
| P5 | `wasm-opt` di binaryen 132 con `--converge` in `tools/wasm_opt.sh` | −5 084 B misurati sul motore (gli altri `.wasm` in proporzione); +~50 s per modulo in CI | medio: ottimizzatore nuovo su tutti i `.wasm` | — | `sito-rust.yml`, `e2e_rust`, `e2e_lr` e gli e2e di mzFinder |
| P6 | Qualità: un aiuto unico per ordinare i `campi` per `(origine, chiave)` (3 copie), `mzlab-lib/sortutil.rs` sostituito da `mzlab_core::sortc` (stesso codice, `mzlab-lib` dipende già da `mzlab-core`) | −2…−4 KB (stima) e 90 righe in meno | basso | P1 | test di `mzlab-core` e `mzlab-lib`; il `.wasm` delle librerie non cresce |
| P7 | Memoria: `Engine::open` (`mzlab-wasm/src/lib.rs:208`) tiene per ogni spettro anche `wide` (intensità `f64`, +8 B a picco) quando il file le scrive a 64 bit | fino a −40 % della memoria di un mzML aperto per intero | medio: `intensity_f64` serve ad alcune rotte | — | solo dopo aver elencato chi chiama `intensity_f64` sui `Run` del motore; rotte identiche |
| P8 | Moduli a richiesta (statistica, origine, centroidi) o Thermo fuori dal motore | −20…−125 KB ciascuno (esclusivi, tabella §2) | alto: nuovo `.wasm`, worker, supervisore; ogni modulo ricopia 150-250 KB di runtime | P1 esaurito | — da studiare solo se il motore torna sopra 1 650 000 B |

Cose valutate e scartate (misurate): `opt-level="s"` (+310 KB); `--converge` con il `wasm-opt` attuale (−859 B); togliere i messaggi dei panic (`panic_immediate_abort`, `-Zlocation-detail=none`: solo nightly, la toolchain è fissata a 1.97 stabile; varrebbe ~10 K di dati più il codice di formattazione); `remap-path-prefix` (~2 KB, ma `.cargo/config.toml` non espande la home di Cargo); `strip` (i nomi li toglie già `wasm-bindgen`).

## 4. Dettagli verificati
### 4.1 Ordinamenti (P1)
- `sortc.rs` esiste già (`by`, `by_key`, `ord`, `any_by`, `any_by_key`) e serve 45 chiamate, ma ne restano 79 alla libreria standard in `mzlab-core` (più 2 in `vendor/opentfraw`). Un'istanza di `sortc::by` costa 500-900 B, una di `slice::sort_by` ~2,5 K (driftsort + quicksort + smallsort per ogni coppia tipo/chiusura).
- La variante più piccola: **un solo** ordinamento non generico su indici `u32` con `&mut dyn FnMut(usize, usize) -> Ordering`, poi la permutazione degli elementi (ciclo di `swap`). Una sola istanza nel `.wasm`; per ogni punto di chiamata resta solo la chiusura.
- Velocità a `opt-level=z`, 2,5 M elementi `(f32, f32, u32)` disordinati: std 249 ms, `sortc::by` 542 ms, variante `dyn` 1 627 ms. Su 10 000 elementi la variante `dyn` costa ~3 ms: irrilevante per le rotte, non per l'indice. Con la variante `dyn` ovunque la costruzione del `.mzidx` di 101 MB passa da ~1,9 s a ~2,2 s.
- **Punti caldi (restano `slice::sort*`)**: `mzidx.rs` (tutti), `mapgrid.rs:118,143`, `mapcells.rs:144`, `maptiles.rs:76,161`, `specpeaks.rs:131,165`, `specdiff.rs:140,174`, `rawlettore.rs:262`. Con questi esclusi la misura è 1 563 674 B.
- **Punti con `partial_cmp(..).unwrap_or(Equal)`** (ordine non totale se arriva un NaN: due algoritmi stabili diversi possono dare ordini diversi): `statref.rs:55,172,251`, `statmat.rs:109,145`, `pca.rs:110`. Se dal chiamante può arrivare un NaN, il punto resta std.
- `sort_unstable*` → stabile: tutti i punti ordinano interi o stringhe, dove elementi uguali sono identici, quindi l'ordine non cambia.
- Bonus: dal Rust 1.81 `slice::sort*` può andare in panic se il confronto non è un ordine totale; con `panic="abort"` il worker muore. `sortc` non va in panic.
- Ne restano ~11 K nella libreria standard (`BTreeMap::from_iter`, i 2 di `opentfraw`, un `i64`): non vale la pena.

### 4.2 Lettore mzML (P2), profilo callgrind su 4 MB
| Voce | Istruzioni | Causa | Codice |
|---|---:|---|---|
| copia in `memset` | 7,9 % | `read_to_end` di `ZlibDecoder` azzera ogni blocco della capacità riservata | `mzml.rs:158-165` |
| `String::push` + `chars` + `next_code_point` | 10,7 % | `a.text.chars().filter(!whitespace).collect::<String>()` anche quando il base64 non ha spazi (quasi sempre) | `mzml.rs:145` |
| `init_tree` di miniz | 6,8 % | un `ZlibDecoder::new` (stato da ~40 KB) per ogni array, due per spettro | `mzml.rs:161` |
| ciclo `f64`+`f32` | 10,3 % | ogni array produce **due** vettori; per l'm/z il `f32` si butta (`finish_spectrum`, `mzml.rs:219`) | `mzml.rs:174-184` |
| `attrs` + `name().to_string()` | ~5 % | un `Vec<(String, String)>` e una `String` per ogni elemento, anche per i `cvParam` ignorati | `mzml.rs:122-134`, `324`, `455` |
- Costruzione dell'indice (callgrind, stesso file): `decode_array` 34 %, `scantable::Source::read` 26 % (di cui SHA-256 14 %: serve, resta), ordinamento dei picchi 16 % (P4), `attrs` 4,7 %.
- Il prototipo misurato (solo le prime due righe della tabella) è in §5, prompt DEV-P2.

### 4.3 Qualità (P6 e regole)
- Funzioni oltre 100 righe: 16. Le più lunghe: `composition::compose` 452, `mzidx::build` 332, `msntree::msn_tree` 253, `iteminfo::item_info` 244, `mzml::parse` 186, `methodcheck::write_rows` 153, `scantable::read` 149, `lr::file_entry` 141. Sono porte riga per riga di Python con prove di parità: dividerle non cambia nulla per l'utente e può far crescere il `.wasm` (meno inlining). Regola: si dividono solo quando un pacchetto le tocca per altro, con misura del `.wasm` prima e dopo.
- Copie: ordinamento dei `campi` per `(origine, chiave)` in `metodo_raw/campi.rs:334`, `agilent/doc.rs:226`, `agilent/quant.rs:239`; `mzlab-lib/src/sortutil.rs` (21 usi) ripete `mzlab-core/src/sortc.rs`.
- `json.rs` (lettore e scrittore), `jsonb.rs` (rapporti) e `methodjson.rs` hanno compiti distinti: nessuna fusione utile.

## 5. Prompt per le chat DEV
Regole comuni: leggi `AGENTS.md` (già nel contesto) e `docs/agenti/rust.md`; ramo `claude/<argomento>` da `origin/main`; una PR, unione con merge commit dopo la CI verde; commit in italiano senza nomi di modelli; non toccare `.github/workflows/`, i tetti di dimensione, `mzlab/web/teoria/`; documenti = solo stato attuale (riscrivi la riga di `docs/agenti/rust.md` che descrive ciò che cambi). Misura del `.wasm` del motore: comandi di §1 di `docs/studi/OTTIMIZZAZIONE_CORE.md`. A fine lavoro un messaggio di max 5 righe al Coordinatore (PR, CI, pages, byte prima/dopo, cosa resta).

### DEV-P1 — Ordinamenti compatti nel motore
```
Pacchetto P1 di docs/studi/OTTIMIZZAZIONE_CORE.md (§4.1). Obiettivo: il .wasm del motore scende di ~140 KB senza cambiare nessuna uscita.
1. In crates/mzlab-core/src/sortc.rs aggiungi UNA funzione non generica `fn order(n: usize, cmp: &mut dyn FnMut(usize, usize) -> Ordering) -> Vec<u32>` (usa `by` sugli indici u32) e le funzioni pubbliche generiche sottili `slice_by<T>(v: &mut [T], cmp)`, `slice_by_key`, `slice_ord` che chiamano `order` e poi permutano `v` in place (ciclo di swap, nessun Clone/Copy richiesto). Commenti in inglese; test in sortc::tests: stesso risultato di `sort_by` std su 1000 vettori casuali con molti pareggi (confronta un campo `pos`).
2. Sostituisci con queste funzioni le chiamate a .sort() .sort_by() .sort_by_key() .sort_unstable() .sort_unstable_by() di crates/mzlab-core/src (esclusi i test), TRANNE i punti caldi, che restano come sono: mzidx.rs (tutti), mapgrid.rs:118,143, mapcells.rs:144, maptiles.rs:76,161, specpeaks.rs:131,165, specdiff.rs:140,174, rawlettore.rs:262. Elenco completo: grep -rnE "\.(sort|sort_by|sort_by_key|sort_unstable|sort_unstable_by)\(" crates/mzlab-core/src.
3. Punti con partial_cmp(..).unwrap_or(Equal): statref.rs:55,172,251, statmat.rs:109,145, pca.rs:110. Per ognuno leggi il chiamante: se un NaN può arrivare lascia lo std e scrivi perché in un commento di una riga; altrimenti converti.
4. Guardia: crates/mzlab-core/clippy.toml con disallowed-methods per slice::sort, sort_by, sort_by_key, sort_unstable, sort_unstable_by, sort_by_cached_key (motivo: "costs ~2.5 KB of .wasm per call site: use sortc"), e #[allow(clippy::disallowed_methods)] con un commento sui punti caldi. python3 tools/controlla_rust.py deve restare verde (clippy_base.json può solo scendere).
5. Prove: cargo test --release -p mzlab-core (MZLAB_GOLDEN_STRICT=1; python3 e numpy ci sono nel container), cargo test -p mzlab-lib -p mzlab-feat; misura del .wasm (atteso <= 1 564 000 B; riporta il numero) e dei .wasm di tools/build_feat_wasm.sh prima/dopo; lancia sito-rust.yml sulla PR (workflow_dispatch) se il container non basta.
6. docs/agenti/rust.md: riscrivi la frase sugli slice::sort* (oggi solo per le librerie) perché valga anche per mzlab-core, con i punti caldi.
```

### DEV-P2 — Lettore mzML più veloce
```
Pacchetto P2 di docs/studi/OTTIMIZZAZIONE_CORE.md (§4.2). Obiettivo: lettura mzML e costruzione del .mzidx più veloci, uscite identiche byte per byte. File: crates/mzlab-core/src/mzml.rs.
1. decode_array (riga ~144): se a.text non contiene spazi ASCII decodifica il base64 direttamente da a.text.as_bytes() (niente String intermedia); altrimenti tieni il filtro attuale.
2. zlib (righe ~152-168): al posto di ZlibDecoder::read_to_end usa flate2::Decompress::new(true) + decompress_vec(.., FlushDecompress::Finish) in un ciclo che fa crescere la capacità con try_reserve fino a limit+1 (stesso limite di oggi), così la memoria non viene azzerata. Gli errori devono restare IDENTICI: stream corrotto o troncato -> err.file.zlib con detail; oltre il limite -> error.memoryBudget. Riusa lo stesso Decompress per tutti gli array (reset(true)), tenendolo nello stato di parse.
3. Una sola larghezza: decode_array riceve cosa serve (m/z: solo f64; intensità a 32 bit: solo f32; intensità a 64 bit: f64 e f32 come oggi) e non alloca il vettore che nessuno usa. finish_spectrum e finish_chromatogram restano uguali nel risultato.
4. parse (riga ~306): confronta e.name().as_ref() come &[u8] (b"spectrum", ...) invece di to_string(); chiama attrs() solo negli elementi che leggono attributi.
Prototipo già misurato (solo 1 e 2): 101 MB da 1,58 s a 1,18 s, .mzidx identico. Misura e riporta con target/release/examples/carico e examples/mzidx su un mzML grande (python3 tools/dati_sintetici.py DIR, poi replica 25 volte gli spettri di HR_DDA-Exploris-t30.mzML: vedi §1 del documento).
Prove: cargo test --release -p mzlab-core con MZLAB_GOLDEN_STRICT=1 (golden.rs, mzidx_parity.rs, robustezza_mzml.rs, upload_parity.rs devono passare senza toccarli); .mzidx dei file di dati_sintetici costruiti con examples/mzidx prima e dopo: cmp identico; TIC di carico identico; il .wasm del motore non deve crescere oltre il tetto (riporta i byte). Aggiungi la riga di misura in docs/agenti/misure.md (sezione del costruttore del .mzidx) e aggiorna la riga di mzml.rs in docs/agenti/rust.md se cambia ciò che descrive.
```

### DEV-P3 — Hasher deterministico (facoltativo)
```
Pacchetto P3 di docs/studi/OTTIMIZZAZIONE_CORE.md. Prima MISURA: se il guadagno sul .wasm del motore è < 5 000 B chiudi la PR senza unire e scrivilo al Coordinatore.
1. crates/mzlab-core/src/hash.rs: un FxHasher (moltiplica-ruota a 64 bit, ~15 righe, commento in inglese), `pub type Mappa<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>` e `Insieme<T>`.
2. Sostituisci HashMap/HashSet in crates/mzlab-core/src (maproute, infusione, scantable/rows, msnalbero, maptiles, profspec, scantable, routes, originroute, ole) e crates/mzlab-wasm/src (lib.rs, nativi.rs): HashMap::new() -> Mappa::default(). In vendor/opentfraw/src (reader.rs, mzml.rs) la stessa cosa con un hasher locale, annotando la modifica in vendor/opentfraw/MODIFICHE-MZLAB.md.
3. Per ogni iterazione su una mappa (for, iter, keys, values, drain) verifica che l'uscita non dipenda dall'ordine (oggi è già casuale: se dipendesse, sarebbe un bug da segnalare, non da cambiare qui).
Prove: test di mzlab-core e mzlab-wasm, cmp dei .mzidx, misura del .wasm prima/dopo.
```

### DEV-P4 — Ordinamento dei picchi per secchi nell'indice (facoltativo)
```
Pacchetto P4 di docs/studi/OTTIMIZZAZIONE_CORE.md. File crates/mzlab-core/src/mzidx.rs, Sec::flush ramo `retained` (riga ~334: all.sort_by(by_mz)). I conteggi per secchio di 1 Da (self.counts, bucket()) ci sono già.
Distribuisci `kept` in un vettore nuovo per secchio (stabile, come begin_round/fill) e ordina ogni secchio con by_mz come fa già il ramo a giri (riga ~343). bucket() è monotona e by_mz mette i NaN in coda nell'ultimo secchio, quindi l'ordine finale è identico. Rispetta il budget: la copia transitoria va contata (reserve con errore error.memoryBudget, mai panico); se non sta, ricadi sull'ordinamento di oggi.
Misura con target/release/examples/mzidx su un mzML grande (tempo «build» e «peak memory»), prima/dopo, 3 giri. Unisci solo se il tempo scende di almeno il 5 % e la memoria massima non sale più del 10 %; altrimenti PR chiusa con i numeri al Coordinatore.
Prove: mzidx_parity.rs (budget grande e piccolo), cmp dei .mzidx prima/dopo.
```

### DEV-P5 — wasm-opt aggiornato (facoltativo, chiedi prima al Coordinatore)
```
Pacchetto P5 di docs/studi/OTTIMIZZAZIONE_CORE.md. tools/wasm_opt.sh installa wasm-opt@1.4.0 (binaryen 112). Prova il pacchetto npm binaryen@132.0.0 (bin/wasm-opt) con -Oz e --converge sugli stessi flag. Misurato sul motore: 1 699 673 B (-Oz) e 1 698 621 B (--converge) contro 1 703 705 B; tempo ~50 s a modulo. Misura tutti i .wasm del sito (motore, librerie, mzFinder) e i tempi. Non toccare i workflow né i tetti. Prove: sito-rust.yml, e2e_rust, e2e_lr, e2e dei moduli mzFinder. Unisci solo se tutti gli e2e sono verdi.
```

### DEV-P6 — Copie e aiuti (dopo P1)
```
Pacchetto P6 di docs/studi/OTTIMIZZAZIONE_CORE.md (§4.3), dopo l'unione di P1.
1. Un aiuto unico per ordinare i campi di un metodo per (origine, chiave), in crates/mzlab-core/src/metodo.rs accanto al tipo Campo, usato da metodo_raw/campi.rs:334, agilent/doc.rs:226, agilent/quant.rs:239.
2. crates/mzlab-lib/src/sortutil.rs: sostituisci i 21 usi con mzlab_core::sortc (stessa semantica: verifica riga per riga che le due by siano identiche) e togli il file; aggiorna la frase di docs/agenti/rust.md che lo nomina.
Prove: test di mzlab-core e mzlab-lib; misura dei .wasm del motore e delle librerie (non devono crescere).
```

### DEV-P7 — Memoria di `Engine::open` (studio + correzione piccola)
```
Pacchetto P7 di docs/studi/OTTIMIZZAZIONE_CORE.md. Engine::open (crates/mzlab-wasm/src/lib.rs:208) legge l'mzML intero con mzml::read e tiene per ogni spettro `wide: Option<Vec<f64>>` quando il file scrive le intensità a 64 bit (8 B in più a picco).
1) Elenca con file:riga chi legge `wide` o `intensity_f64` su un Run conservato in Engine::runs. 2) Se nessuna rotta del motore ne ha bisogno, scarta `wide` dopo la lettura in Engine::open (non in mzml::read, che altri usano). 3) Se qualcuna ne ha bisogno, non cambiare nulla e riporta l'elenco al Coordinatore. Prove: test delle rotte (routes_parity.rs, lr_parity.rs), e2e_rust; misura la memoria con un mzML a 64 bit (examples o e2e) prima/dopo.
```
