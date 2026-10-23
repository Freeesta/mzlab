# Conversione `.raw` → mzML: banco, roofline, cause misurate

Banco di misura del convertitore (`crates/mzlab-core/examples/raw_mzml_banco/`, lanciatore `tools/bench/raw_mzml.py`) e numeri di partenza dei criteri V1-V9 (`docs/studi/raw-mzml-immediato/CRITERI.md`). Il banco non cambia nessun byte di uscita: legge con `Lettore` e scrive con `Conversione`/`MzmlWriter` veri. Tabella cumulativa dei guadagni: `misure-velocita.md`; lettore Thermo: `thermo-velocita.md`.

## Come si lancia
- `python3 tools/bench/raw_mzml.py --file X.raw` (file vero) o `--denso 2` (caso denso sintetico da 2 GB, generato in `--tmp` e cancellato); `--zlib-rs` rifà tutto con la feature `banco-zlib-rs`; le opzioni dopo `--` vanno al banco. Scrive `.verifica/raw-mzml/<etichetta>.<backend>.json` (schema di `MISURE.md` §4: macchina, git, rustc, kernel, profilo `cli`, banda del disco misurata con `dd`).
- Diretto: `cargo build --profile cli -p mzlab-core --example raw_mzml_banco`, poi `target/cli/examples/raw_mzml_banco --file X.raw --giri 5 --json out.json`. `--solo fasi,scala,cli,micro,d8` sceglie le sezioni (micro e d8 solo se chieste), `--thread 1,2,3,4`, `--peaks tutto|file`, `--senza-impronte` (niente SHA-256 dell'uscita nelle serie di scala), `--senza-conta` (l'allocatore contatore non conta: per i tempi a più thread, vedi D8), `--forma lab-a|lab-b` (con `--denso`: spettri piccoli delle due forme dei file LAB), `--gb`, `--cancella`.
- Ogni misura è ripetuta `--giri` volte (5 di norma): il JSON ha i secondi di ogni giro, mediana, scarto interquartile (`iqr_s`), `migliore_s`, `primo_giro_s`, `mediana_senza_primo_s`, CPU, RSS di picco (azzerato con `clear_refs`), picco dell'heap, numero e byte delle allocazioni (allocatore contatore, solo nel banco), page fault. Le serie di scala fanno `scrivi_con` a livelli zlib 0/1/6 e `indicizzato` (livello 1) con 1…N thread.
- Rumore: dentro un lancio lo scarto interquartile è ≤ 3 % sulle sorgenti piccole e sul denso; tra due lanci le mediane stanno entro 5 %; il primo giro su un file non in cache costa fino a +9 % (su PRIDE da 1,7 GB +6 s: [16,0 · 10,0 · 10,1] s a 1 thread), quindi per un confronto si usa `mediana_senza_primo_s` o `migliore_s` su ≥ 15 giri. Un cambio sotto 1-2 % si vede solo con molti giri sullo stesso binario; non lanciare cargo, clippy o il pre-push mentre il banco misura.
- Il generatore denso è deterministico (SplitMix64 intero, solo operazioni IEEE): stesso SHA-256 a ogni giro (`denso_test.rs`); il `Lettore` finto serve i pacchetti dal file attraverso i decoder veri (`opentfraw::pacchetto::decodifica`), rilettura al bit su 3 scansioni; la variante da 8 GB si genera e si cancella in un test `#[ignore]` (`cargo test --profile cli -p mzlab-core --example raw_mzml_banco -- --ignored`).

## Macchina e T_min
- Macchina di questi numeri: Xeon 2,8 GHz, 4 core, 16 GB, Linux 6.18 (VM), profilo `cli`, zlib `miniz_oxide`, niente SHA-NI (SHA-1 ≈ 440 MB/s, SHA-256 ≈ 204 MB/s). Disco: scrittura sequenziale 216 MB/s (`dd conv=fdatasync`), lettura 3,0 GB/s (page cache).
- `T_min = max(B_in/BW_lettura, B_out/BW_scrittura, W_cpu/N, W_seriale)`; `W_seriale` = SHA-1 dell'ingresso nella testata del mzML (un solo flusso), che su file grandi è già un pavimento. Micro-banchi (`--solo micro`): base64 1,87 GB/s; conversione freq→m/z 4,1 ns/punto; zeramento 63 GB/s; malloc+free 65 ns; XML 80-86 µs/spettro; decodifica FT pacchetto intero 8,4 µs (lab-a, 11 KB), 10,6 µs (lab-b, 4 KB), 4,6 ms (denso, 1,5 MB), di cui lista dei picchi 4,2 µs / 0,83 µs / 154 µs; deflate livello 0/1/6 sul denso:
  214 / 160 / 22 MB/s (rapporto compresso/originale 1,00 / 0,80 / 0,80).

## Punti di partenza V1-V9
Misure in secondi, mediana senza primo giro. Lab-b e lab-a: `--senza-conta`, 7 giri; le altre colonne con l'allocatore contatore acceso. LAB-A, LAB-B e BAF non sono raggiungibili da questa macchina: non verificato su file veri; valgono i numeri di `MISURE.md` (LAB-B `convert` 5,97 s, scrittore 2,87 s; LAB-A 3,96 s) e le sorgenti sintetiche di forma uguale (`--forma lab-b`, 16 731 scansioni di 4 KB, 69 MB; `--forma lab-a`, 6 336 scansioni di 11 KB, 70 MB), che danno scrittore a 1 thread 2,1 s (lab-b) e 0,8 s (lab-a), ordine di grandezza di quelli veri.

| | PRIDE (stand-in, 1,71 GB, 46 730 scansioni) | denso 0,4 GB (271 scansioni) | denso 2 GB | denso 8 GB | lab-b sintetico | lab-a sintetico |
|---|---|---|---|---|---|---|
| T_min (s) | 5,7 (SHA-1 ingresso); scrittura 3,3; CPU/4 2,4 | 3,3 (scrittura 711 MB); CPU/4 1,8 | ≈ 16 (3,55 GB/216 MB/s) | ≈ 66 (14,2 GB) | ≈ 1 | ≈ 0,5 |
| V1/V2/V3 scrittore, liv. 1, 1 thread | 11,8 | 7,2 | indicizzato 42,4 | indicizzato 168 | 2,1 | 0,8 |
| V4 liv. 0, 4 thread | 13,7 | 2,46 | 11,6 | 46,3 | 0,61 | 0,21 |
| V4 liv. 1, 4 thread | 13,3 | 2,46 | 11,95 | 48,6 | 0,62 | 0,24 |
| V4 liv. 6, 4 thread | 16,9 | 8,5 | 41,8 | 168,8 | 0,81 | 0,35 |
| V5 liv. 1, 1→2→4 thread | 11,8 → 12,6 → 13,3 (più lento) | 7,2 → 2,65 → 2,46 (2,9×) | — | 168 → 72 (indicizzato 1→4, 2,3×) | 2,14 → 0,86 → 0,62 (3,5×) | 0,79 → 0,36 → 0,24 (3,3×) |
| V6 RSS di picco, 1 thread (4 thread) MB | 339 | 26 (111) | 31 indicizzato (122) | 48 indicizzato (158) | 13 | 7 |
| V7 allocazioni per scansione | ≈ 670 | ≈ 59 000 | ≈ 59 000 | ≈ 59 000 | ≈ 1 720 | ≈ 1 590 |
| V9 apertura (s) | 0,95 | 0,00005 | — | — | — | — |

Letture: V5 è rispettato sui sintetici lab-b e lab-a (4 thread: 2,7-3,5× a livello 0-1, 3,6× a livello 6) e sul denso (2,9× con 4 thread a livello 1, 3,9× a livello 6 su 4 core: 33→8,5 s); PRIDE non è rimisurato dopo la correzione del pool (D8): la sua riga è quella del pool vecchio. V6: l'RSS non è costante con la dimensione del file (4 thread: 111 → 122 → 158 MB per 0,4 → 2 → 8 GB;
1 thread indicizzato 70 → 31 → 48 MB) e PRIDE sta a 339 MB: causa non identificata (nel denso il picco dell'heap è 12 MB: la parte oltre l'heap è memoria di sistema/mappata). V7: lo scarto dall'obiettivo (≤ 1 ogni 1000 scansioni) è di 6-8 ordini di grandezza; a 1 thread il contatore stima il 10-20 % del tempo in allocazione, contro circa il 10 % di callgrind (stesso ordine di grandezza). V8 (browser) non è misurato qui (RM-5). V10: vedi sotto.

Suddivisione di una scrittura PRIDE senza zlib (fasi, secondi sul totale 4,08 xml-solo → 1,66 scrittura sink): decodifica 1,5, extra per scansione 1,5, XML con array vuoti 1,66, base64 3,5, SHA-1 uscita e offset 1,2, scrittura su file 0,77; zlib livello 1 aggiunge 1,25 s, livello 6 25 s (miniz_oxide).

## zlib: miniz_oxide contro zlib-rs
Stesso banco, build con `--features banco-zlib-rs`, 1 thread, secondi totali (miniz → zlib-rs; byte di uscita): PRIDE livello 1 8,76 → 7,26 s (−17 %; 722 → 837 MB, +16 %), livello 6 32,8 → 17,6 s (−46 %; 718 → 717 MB); denso 0,4 GB livello 1 6,99 → 10,8 s (+54 %; 711 → 921 MB, +30 %), livello 6 32,5 → 19,1 s (−41 %; 712 → 796 MB).
Il livello 0 oscilla di più (PRIDE 7,5 → 4,2 s, denso 6,3 → 3,1 s: il primo giro paga la cache fredda), da non leggere come guadagno. zlib-rs cambia i byte compressi: richiede una nuova versione `conv/1` con prima/dopo; guadagna dove l'ingresso è fatto di spettri a centroidi e perde sui profili densi a livello 1. Scelta rimandata a RM-3 (V10).

## D11: le impronte di `mzlab-cli convert`
Su PRIDE (RM-0, misura `--solo cli`, senza SHA-NI) il comando impiegava ≈ 35 s contro 20 s del solo scrittore indicizzato: non era conversione ma impronte calcolate più volte sugli stessi byte (SHA-256 dell'ingresso 8,6 s, SHA-1 dell'ingresso 5,7 s dentro la testata, SHA-256 dell'uscita 3,7 s in `keep` e di nuovo 3,7 s nel rapporto, ingresso letto 3×, uscita riletta 2×).
Ora ogni byte è hashato una volta per scopo: `flow_set::hash_file` (`hashio::hash_read`) legge l'ingresso una volta e ne prende SHA-256 e SHA-1, e `file_set` passa lo SHA-1 al lettore (`Membro.sha1`), che non rilegge il file per la testata; lo scrittore passa per `hashio::HashWrite` e `write_mzml_with` restituisce byte e SHA-256 dell'uscita, che `FileRunner::keep_known` e il rapporto di `convert.rs` usano senza rileggerla (`flow_set::remember_sha`). Un file preso dalla cache viene hashato come prima.
Misura sintetica (mzXML 320 MB, uscita 323 MB senza zlib, macchina con SHA-NI, 3 giri alternati): 2,93 → 2,73 s; byte di uscita identici (`cmp`). Il guadagno è proporzionale al costo degli hash: su PRIDE non rimisurato.

## D8: più thread di codifica non rallentano
Il pool scrittore (`mzmlwrite/pool.rs`) passa gli spettri agli operai a blocchi (≤ 32 spettri o ≈ 100 000 punti) e se li fa restituire con il testo: li libera il thread che li ha allocati.
Sintetico lab-b (`--forma lab-b --solo scala --senza-conta --thread 1,2,3,4`, 7 giri, secondi): livello 0 1,65 · 0,65 · 0,61 · 0,61, livello 1 2,14 · 0,86 · 0,58 · 0,62, livello 6 2,94 · 1,23 · 0,84 · 0,81; lab-a livello 0 0,63 · 0,28 · 0,20 · 0,21. Con lo stesso banco il pool a un messaggio per spettro dava a livello 0 1,57 · 3,28 · 3,39 · 3,40 s (lab-b) e 0,61 · 1,00 · 0,94 · 0,86 s (lab-a). I byte di uscita sono identici (SHA-256 uguali a 1 e 4 thread, livelli 0/1/6 e indicizzato; test `tests_pool`).
- Causa: i vettori di ogni spettro sono allocati dal thread che legge; se li libera un operaio, i blocchi tornano nell'arena di glibc del lettore e contendono il suo blocco (`futex`: 516 000 chiamate in cinque passate di lab-b a 4 thread con un messaggio per spettro, 8 800 con blocchi e restituzione). La sola messa a blocchi non basta (a 4 thread lab-b livello 0: 2,7 s contro 1,8 a 1 thread); la restituzione dei vettori al thread lettore porta lab-b a 0,8 s e il blocco da 32 a 0,6 s (livello 6: 1,23 → 0,93 s).
- Il banco con l'allocatore contatore (`conta.rs`: quattro atomici condivisi per allocazione) rallenta da solo i run a più thread (lab-b livello 0 a 4 thread 5,1 s con il contatore, 2,95 senza prima della correzione): per i tempi a più thread si usa `--senza-conta`.
- `--solo d8` misura i meccanismi isolati: canale con 1 compito per messaggio 12-24 µs di sovraccosto per compito, con 32 compiti per messaggio 0,3-1 µs; blocchi da 8 KB liberati da un altro thread 1,4 µs a 1 thread, 5,2 a 2, 10,6 a 4, contro 0,08-0,17 µs se liberati dallo stesso thread. Escluso: il numero di arene di glibc (`MALLOC_ARENA_MAX=1` e `=4`) e la compressione (presente anche a livello 0).

## Denso sintetico
`Parametri::denso`: 20 500 picchi centroidi con ≥ 200 000 punti di profilo per scansione (8-12 bin per picco), 271 scansioni per 0,4 GB, ≈ 1,4 MB per pacchetto; decodifica 4,6 ms/scansione (lista dei picchi 154 µs), 57 468 allocazioni per scansione nel decoder (una per picco e per bin: V7). Uscita non compressa 892 MB (0,4 GB), 4,46 GB (2 GB), 17,8 GB (8 GB): a livello 0 con 4 thread il limite diventa il disco di scrittura (216 MB/s → 4,1 s per 892 MB contro 2,46 s misurati: la cache di sistema assorbe la differenza).
Il caso da 8 GB occupa 8 GB di disco più l'uscita: si cancella da solo con `--cancella`.
