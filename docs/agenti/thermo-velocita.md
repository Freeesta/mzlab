# Lettore Thermo `.raw`: criteri di velocità e zero perdite, banco e numeri di partenza (T-7)

Studio: `docs/studi/LETTORE_THERMO_PERFETTO.md` §4, `docs/studi/RAW_VELOCE.md` §1-3. Prompt delle fasi: `docs/studi/PROMPT_LETTORE_THERMO/T-7.md`. Oracolo e formato del dump: `scandump.md`. Due livelli con criteri e numeri SEPARATI, mai una tabella che li mescola: **A** = lettore fedele e veloce contro RFR; **B** = ponte senza perdite verso Flow sui centroidi.

## Come si dichiara ogni numero
- Ogni numero porta lo SHA di 40 caratteri del commit di `main` su cui è stato misurato, la macchina, il bersaglio (`native` o `wasm`) e il profilo di compilazione (`cli` = `opt-level = 3`; `release` = `opt-level = "z"`, quello dei `.wasm`). Un numero senza questi quattro dati non entra nelle tabelle.
- Il confronto con RFR vale solo sullo stesso Mac e solo sui file che si aprono con ENTRAMBI i lettori; gli altri si elencano a parte con il motivo (mai mediati). ThermoRawFileParser, msconvert e mzmine si misurano sullo stesso Mac degli altri.
- Almeno cinque giri (15 per decidere un guadagno piccolo); si riportano la mediana, il giro più veloce (`best`) e il rumore (`noise`: intervallo e deviazione standard in % dei giri); un guadagno si dichiara solo se supera il rumore misurato sulla stessa macchina. Il primo giro (cache fredda) e i giri a cache calda sono colonne separate; per gli strumenti che passano da RFR si misura il costo totale che vede l'utente.
- Una riga di base si ripete dopo ogni PR che cambia il contenuto letto o il percorso di lettura (T-3 #1537 tocca gli eventi di scansione: ripetere la riga A prima di T-7c). Ogni numero va in tabella anche se perdiamo, con la causa.
- Regole del Coordinatore: solo Rust nel prodotto; **nativo** (CLI, desktop) senza tetti di `.wasm` (`mmap`, `rayon`, SIMD con `std::arch` dietro controllo della CPU all'avvio); **browser** con i tetti e il +10 % dichiarato (SharedArrayBuffer non c'è: più worker).
- `unsafe` solo in moduli piccoli e isolati con test, fuzzing e gemello sicuro confrontato al bit.
- Dipendenza nuova per la velocità solo se il guadagno misurato è ≥ 3 % sul suo bersaglio (regola `REGOLE.md` del 22/10: anche pochi punti percento contano) e la licenza è permissiva; nella PR righe di `Cargo.toml`/`Cargo.lock`, licenza e numeri prima/dopo.
- Nessuna regola di velocità supera A1-A4. Ogni intervento di velocità aggiunge una riga alla tabella cumulativa `misure-velocita.md` (data · PR · passo · prima · dopo · %).

## Criteri del livello A (lettore)
- **A1 Registro dei byte**: ogni byte appartiene a una sezione nominata (intestazione, `RawFileInfo`, `RunHeader`, metodo OLE2, indice, eventi, trailer, status, tune, error log, pacchetti, riempimento), interpretato o conservato grezzo con un nome. Byte fuori da ogni sezione = `unexplained_bytes`: test rosso sui file costruiti, obiettivo 0 sul corpus. Il registro sta in un esempio o in una feature solo nativa, mai nel modulo `thermo`.
- **A2 Andata e ritorno**: ricodificare ogni sezione ridà gli stessi byte (SHA-256); con A1 a 0 la somma ridà il file. Lo scrittore sta solo nei test e negli strumenti nativi.
- **A3 Profilo conservato**: il lettore non centroida mai di nascosto; dà il profilo e, per le scansioni FT che lo hanno, il flusso dei centroidi del produttore (m/z, intensità, rumore, linea di base, risoluzione, carica) senza perdere colonne.
- **A4 Impronte**: SHA-256 per scansione degli array in forma canonica (m/z e intensità in f64 little-endian, ordine del file) e per file (impronta delle impronte), campo `peaks_sha256` di `scandump/1`. Se differisce da RFR il comparatore dà la massima differenza in ulp e la causa; nessuna tolleranza nascosta.
- **A5 Velocità a regime**: MB/s e scansioni/s per apertura, lettura di tutte le scansioni, una scansione a caso (100 a caso, mediana e 95° percentile). Obiettivo: nativo ≥ RFR su tutte e tre; browser entro 1,7× il nativo.
- **A6 Tempo al primo risultato**, tre tempi dal clic: TIC/BPC (indice e intestazioni, senza decodificare picchi), prima scansione decodificata, tutto. Obiettivo: TIC/BPC e prima scansione sotto RFR.
- **A7 Confronto**: RFR, ThermoRawFileParser, msconvert e l'importazione `.raw` di mzmine (con il lettore che usa dentro), stesso Mac, stessi file, versione di ogni strumento.

## Criteri del livello B (Flow sui centroidi)
- **B1** La perdita profilo → centroidi esiste solo nel passo `spectra.centroid` (D6/L8, parametri in `prov/1`); lettore, colonne e matrice non perdono nulla.
- **B2 Colonne senza perdite**: m/z e intensità in colonne separate, delta + rimescolamento dei byte + zstd, MAI Numpress né arrotondamenti; decodificate ridanno le impronte A4. Blocco opzionale nuovo del `.mzidx` con versione dichiarata (`error.newerFormat`); i `.mzidx` esistenti restano uguali byte per byte. Numeri: dimensione contro `.raw` e mzML, MB/s di scrittura e lettura.
- **B3 Matrice scansione × bin (CSR)**: un XIC è una fetta di colonna, uguale al bit alla somma a forza bruta; la feature table dalla matrice è uguale byte per byte a quella della via mzML.
- **B4 Centroidatura fusa**: decodifica e centroidatura in un passo, centroidi uguali al bit a `lettori_centroidi.rs`; un algoritmo nuovo è un parametro del passo, spento di base, dichiarato con perdita e confrontato con `meta.compare`.
- **B5 Flow contro mzmine**: tempo e memoria di picco `.raw` → centroidi → feature table contro mzmine (`tools/bench/corri_mzmine.py`) e contro la nostra via `.raw` → mzML → feature, stessi file e parametri.

## Banco
- `cargo run --profile cli -p mzlab-lettori --features thermo --example thermo_banco -- FILE.raw --sha SHA40 --machine TESTO --profile cli [--reps 5] [--target native]`: una riga `banco/1` (apertura, TIC/BPC dall'indice, prima scansione, tutte le scansioni, 100 scansioni a posizioni fisse, MB/s, scansioni/s; `first_lap`, `median`, `best`, `noise`). Il file è letto in memoria prima del giro (`file_read_ms` a parte): non misura il disco. Le 100 posizioni sono xorshift64 da `0x9E3779B97F4A7C15`, `k = x % n`: l'oracolo RFR deve usare le stesse.
- `python3 tools/bench/thermo_velocita.py unisci BANCO.jsonl RFR.jsonl > UNITO.tsv` poi `tabella UNITO.tsv --sha SHA40 --maquina TESTO`: tabelle per famiglia solo sui file aperti da entrambi, conteggio dei restanti a parte, nessun nome di file. `RFR.jsonl` = righe `banco/1` con `reader: "rfr"` e le stesse fasi.
- Fasi del percorso verso la mappa e dell'mzML (livello B, nativo): `cargo run --profile cli -p mzlab-core --features thermo --example raw_tempi -- FILE.raw` (`RAW_VELOCE.md` §1). Browser: `python3 tools/bench_raw.py FILE.raw` (serve `wasm-bindgen`).
- Dump di `thermo_scandump` (`scandump.md`): il suo `read_all_ms` include trailer e righe del dump per scansione, il suo `open_ms` apre due volte e copia il file; NON è lo stesso lavoro di `thermo_banco` e non va confrontato con esso.

## Rumore di misura (cloud 4 vCPU, `thermo_banco`, profilo `cli`, `main` `ab4fe7d00606e58218a7ea8570f333a4026b4fc4`)
- Dentro una corsa da 15 giri la deviazione standard è 8-28 % della media e l'intervallo (massimo − minimo) 26-118 % della mediana, su LAB-A, PXD077492, PXD064022 e MTBLS5358: la macchina è condivisa e i giri singoli non separano differenze dell'1-2 %.
- Tra cinque corse da 15 giri sullo stesso file (PXD077492) la mediana di «tutto» varia 32,7-34,6 ms (5,5 %) e quella dell'apertura 63,6-73,5 ms (14 %); il giro più veloce varia 32,0-33,2 ms (3,9 %) e 61,7-62,9 ms (2 %). Quindi sul cloud si decide con `best` su ≥ 15 giri e con la stessa corsa a coppie prima/dopo (stesso binario, stessa sessione, a turno); i guadagni sotto il 4 % si confermano sul Mac con il banco su macchina ferma.

## Numeri di partenza, livello A
**A-1 · Mac arm64, nativo, `thermo_scandump` di `a9a7468ffce0921fa03b2a99ca04d64a4b0e0453`** (con #1508, prima di T-3 e T-5), RFR = RawFileReader ufficiale, 221 file aperti da entrambi su 238 (24,5 GB, 1 899 674 scansioni; il numero di scansioni coincide in ogni file). Lettura di tutte le scansioni = fase `read_all` del dump di entrambi (metadati, filtro, trailer, picchi per scansione); file ≥ 500 scansioni non separati.

| Famiglia | File | Scansioni | Apertura s (RFR / nostro) | Lettura s (RFR / nostro) | Scansioni/s | MB/s | Rapporto mediano per file |
|---|---|---|---|---|---|---|---|
| Q Exactive | 119 | 883 156 | 21,9 / 9,9 | 148,0 / 42,4 | 5 966 / 20 825 | 64 / 224 | 3,5× |
| Orbitrap Fusion, Eclipse | 68 | 744 150 | 13,5 / 8,0 | 120,3 / 22,9 | 6 186 / 32 470 | 94 / 493 | 5,0× |
| Orbitrap IQ-X | 16 | 198 790 | 3,3 / 2,9 | 29,2 / 7,4 | 6 802 / 26 878 | 122 / 480 | 4,4× |
| LTQ, Velos, Elite | 6 | 14 441 | 0,4 / 0,3 | 1,7 / 0,3 | 8 570 / 43 107 | 52 / 264 | 10,1× |
| TSQ | 6 | 59 129 | 0,5 / 3,5 | 1,2 / 0,4 | 49 233 / 140 449 | 66 / 189 | 3,4× |
| Tutti (inclusi 6 file di altri strumenti o senza nome) | 221 | 1 899 674 | 39,8 / 24,7 | 300,6 / 73,5 | 6 319 / 25 854 | 82 / 334 | 3,6× |

- **Dove perdiamo**: l'apertura. Nostro più lento di RFR in 18 file su 221 (8 IQ-X, 5 TSQ, 3 LTQ, 1 Q Exactive, 1 Fusion); sui TSQ a SRM l'apertura costa 0,17-1,35 s contro 0,05-0,09 s (somma TSQ 3,5 s contro 0,5 s). Nella lettura di tutto non perdiamo in nessun file. Causa non verificata: nel dump l'apertura è doppia e copia il file (vedi sopra), e i file SRM hanno decine di migliaia di eventi; A-2 la misura con il banco.
- **Aperti da uno solo, a parte**: RFR apre e noi no 7 file (formato 63/64: LTQ Velos, LTQ Orbitrap XL, 4 TSQ Vantage, 1 TSQ Quantum Ultra, `AllocationTooLarge`; il lavoro T-5 #1533 su `main` ≥ `8fc88a6350b190dba1587fa9f7793bd9968d4848` tollera i flussi illeggibili: la riga va rifatta, richiesta R-06); noi apriamo e RFR no 2 file; 8 non li apre nessuno (5 sono file AppleDouble da 4 KB).
- Mancano per A5-A7: scansione a caso, TIC/BPC e prima scansione di RFR con le definizioni del banco; ThermoRawFileParser, msconvert e mzmine (richieste R-07 e R-08 nella descrizione della PR T-7a).

**A-2 · Cloud 4 vCPU Intel Xeon 2,80 GHz, 16 GB, nativo, `thermo_banco`, profilo `cli`, 5 ripetizioni, mediana, `main` `ab4fe7d00606e58218a7ea8570f333a4026b4fc4`** (lettore invariato rispetto a quel commit). Solo il nostro lettore (qui non c'è RFR: tendenza, non confronto). 28 `.raw` ≤ 90 MB del corpus aperti; 7 file del corpus non si aprono o non hanno scansioni decodificabili (tre sono troncati, `err.file.rawTruncated`, per esempio 1,4 MB su 102 MB dichiarati). File in memoria; apertura = `apri` + `indice`.

| File | MB | Scansioni | Picchi | Apertura ms | TIC/BPC ms | 1ª scansione µs | Tutto ms | MB/s | Scansioni/s | A caso µs p50 / p95 |
|---|---|---|---|---|---|---|---|---|---|---|
| LAB-A | 86,8 | 7952 | 4 185 930 | 163,6 | 0,10 | 49 | 119,4 | 727 | 66 616 | 21 / 29 |
| LAB-B | 82,3 | 16 524 | 1 051 091 | 208,1 | 0,13 | 33 | 117,8 | 699 | 140 325 | 14 / 32 |
| MTBLS5358 | 34,1 | 7492 | 3 642 049 | 45,6 | 0,05 | 12 | 35,8 | 953 | 209 512 | 5 / 6 |
| PXD077492 | 31,8 | 6916 | 520 434 | 63,5 | 0,07 | 15 | 32,7 | 973 | 211 758 | 7 / 13 |
| PXD064999 | 28,8 | 4647 | 1 715 797 | 52,2 | 0,04 | 20 | 41,9 | 686 | 110 821 | 11 / 18 |
| PXD064022 (SRM) | 22,9 | 11 056 | 36 594 | 422,7 | 0,13 | 8 | 20,8 | 1100 | 531 046 | 3 / 4 |
| MTBLS9839 | 17,4 | 370 | 94 080 | 12,9 | 0,00 | 15 | 3,0 | 5883 | 125 027 | 8 / 16 |
| PXD059868 | 15,1 | 261 | 232 003 | 12,1 | 0,00 | 25 | 4,8 | 3140 | 54 194 | 19 / 51 |
| PXD006054 | 11,5 | 27 567 | 48 580 | 58,9 | 0,30 | 16 | 17,2 | 669 | 1 605 758 | 1 / 2 |
| MTBLS14724 | 10,8 | 543 | 401 233 | 12,0 | 0,00 | 18 | 8,2 | 1310 | 66 059 | 19 / 41 |
| MTBLS14496 (SRM) | 10,5 | 20 552 | 42 205 | 437,1 | 0,28 | 10 | 30,1 | 348 | 682 222 | 3 / 4 |
| MTBLS1752 | 7,5 | 17 859 | 35 607 | 44,3 | 0,18 | 15 | 10,8 | 695 | 1 649 187 | 1 / 2 |
| PXD057351 (SRM) | 7,3 | 12 526 | 100 208 | 214,4 | 0,14 | 12 | 42,7 | 170 | 293 254 | 4 / 5 |
| PXD067770 | 6,5 | 2365 | 324 499 | 39,1 | 0,02 | 16 | 15,0 | 434 | 157 558 | 9 / 17 |
| MTBLS1807 | 5,9 | 14 550 | 25 642 | 32,9 | 0,14 | 9 | 8,6 | 684 | 1 693 450 | 1 / 2 |
| altri 13 file ≤ 4,2 MB | 0,1-4,2 | 1-5396 | | 1,1-63,0 | ≤ 0,04 | 4-86 | 0,0-10,8 | | | 1-76 / 1-107 |

- L'apertura è dominata da qualcosa che non è la lettura dei picchi: sugli SRM (PXD064022, MTBLS14496, PXD057351) apre in 214-437 ms e legge tutto in 21-43 ms, cioè l'apertura è 10-20× la lettura; è dove RFR vince (A-1). Cause da cercare nel profilo prima di T-7c (indice degli eventi e dei trailer, `GenericRecordStream`).
- TIC/BPC e prima scansione sono già sotto il millisecondo e i 100 µs su tutti i file: per A6 il confronto con RFR è l'unico dato mancante.
- La lettura a 0,7-1,1 GB/s con il file in memoria misura la decodifica, non il disco: il tempo da disco o OPFS sta in `file_read_ms` del banco e nelle letture a pezzi del browser.

**A-2b · Riga di base ripetuta dopo T-3 (#1537, eventi di scansione): cloud 4 vCPU, nativo, `thermo_banco`, profilo `cli`, `main` `239fa02602a7cf97c68f25440e033cd6e8b13dc4` contro `ab4fe7d00606e58218a7ea8570f333a4026b4fc4` (A-2).** Due binari a turno sullo stesso file (stessa sessione), 15 giri e due corse per binario, giro più veloce (`best`) della corsa migliore; rumore tra le due corse dello stesso binario 0,3-4,5 % (fino a 19 % sui file piccoli). 20 file > 1 MB; differenza = nuovo − vecchio.

| Misura | Mediana delle differenze | Intervallo | Oltre il rumore |
|---|---|---|---|
| Apertura (`apri` + indice) | −3,1 % | −9,4 … +0,4 % | più veloce di 6-9 % su 6 file SRM o con pochi picchi (MTBLS14496, MTBLS7889, PXD057351, PXD059868, PXD064022, PXD019525) |
| Tutte le scansioni | +0,8 % | −2,4 … +15,3 % | **più lento** di 13-15 % su MTBLS1752, MTBLS1807, PXD019516 (SRM, 1-2 picchi per scansione, +77-87 ns per scansione), 8,9 % su PXD006054, 3,7-4,6 % su LAB-B, MTBLS775, MTBLS9839 |

- Nei file Orbitrap grandi (LAB-A, PXD077492, PXD064999, PXD067770) la lettura di tutto non cambia oltre il rumore (da −2,4 a +3,3 %). Il costo per scansione che T-3 aggiunge è fisso, quindi pesa solo dove la scansione è quasi vuota; la causa (costruzione del filtro per passo o lettura degli eventi) non è stata cercata: da profilare prima di T-7c.
- TIC/BPC e prima scansione restano sotto 1 ms e 100 µs. Il rapporto con RFR sul Mac aspetta R-15 (RFR contro `thermo_banco`).

**A-3 · Cloud 4 vCPU, bersaglio `wasm`, `tools/bench_raw.py` (modulo `thermo`, profilo `release` senza `wasm-opt`, Node 22, `wasm-bindgen` 0.2.129), `main` `ab4fe7d00606e58218a7ea8570f333a4026b4fc4`, UNA corsa** (non le 5 ripetizioni: provvisorio). Nativo = A-2 (profilo `cli`, `opt-level = 3`); il confronto mescola profilo e lavoro (la scansione del modulo costruisce anche la meta JSON e copia l'array), quindi è un limite superiore del rapporto.

| File | `apri` + indice ms (wasm / nativo) | Tutte le scansioni ms (wasm / nativo) | Rapporto apertura | Rapporto lettura | µs per scansione (wasm) |
|---|---|---|---|---|---|
| LAB-A | 255,8 / 163,6 | 748,4 / 119,4 | 1,6× | 6,3× | 75,2 |
| PXD077492 | 248,6 / 63,5 | 146,4 / 32,7 | 3,9× | 4,5× | 17,6 |
| PXD064022 (SRM) | 1483,6 / 422,7 | 68,2 / 20,8 | 3,5× | 3,3× | 4,4 |

- **Dove perdiamo**: A5 chiede browser ≤ 1,7× il nativo e oggi la lettura è 3,3-6,3× (rapporto di profilo `z` contro `3` incluso). Memoria del modulo dopo `apri`: 32-41 MB (il tetto di memoria iniziale del modulo è 32 MB).

## Numeri di partenza, livello B
**B-1 · Cloud 4 vCPU, nativo, `raw_tempi`, profilo `cli`, `main` `ab4fe7d00606e58218a7ea8570f333a4026b4fc4`, una corsa** (cache calda, letture a blocchi di 256 KB come nel modulo). Decodifica di tutto, via normale e via `lettori_centroidi`; il resto è comune.

| File | Decodifica (s) normale / centroidi | Tabella s | `.mzidx` s | Tabella + `.mzidx` + scratch s | mzML s | Picco memoria |
|---|---|---|---|---|---|---|
| LAB-A (87 MB, 7952 scansioni) | 0,130 / 0,137 | 0,604 | 0,563 | 0,947 | 2,282 (98,2 MB) | 297 MB |
| PXD077492 (32 MB, 6916 scansioni) | 0,071 / 0,041 | 0,541 | 0,156 | 0,500 | 1,290 (35,9 MB) | 234 MB |

- Non esistono ancora: colonne (B2), matrice (B3), centroidatura fusa (B4), feature table da `.raw` (B5). Le righe B2-B5 si riempiono nelle fasi T-7d e T-7e; `.raw` → centroidi → feature table contro mzmine richiede mzmine sul Mac (R-08). Il tempo del Flow `.raw` → mzML → feature table e i valori di mzmine su mzML (`mzfinder-benchmark.md`) non hanno qui uno SHA e non si riportano.
- Mappa dei costi (SHA-256 dell'impronta, SHA-1 della testata mzML, zlib) e obiettivi di RV-03/RV-04/RV-06: `RAW_VELOCE.md` §2.2 e §3 (misure precedenti con `opt-level = "z"`, senza SHA).

## Righe da ripetere
- Riga A sul Mac con il banco su entrambi i lettori (R-15, RFR contro `thermo_banco`): SHA nuovo, stessa tabella A-1 con le definizioni di `thermo_banco`; la ripetizione sul cloud dopo T-3 è A-2b.
- Bersaglio `wasm`: A-3 va ripetuto con 5 ripetizioni e, per il rapporto con il nativo, con lo stesso profilo (`release`) dei due bersagli; tetto del modulo `thermo` 546 304 B (`wasm-tetti.md`).
