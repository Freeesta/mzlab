# Pacchetti .wasm: quanti, quali, con che tetto

Studio per il Coordinatore e per Federico. Solo numeri e proposte: nessun codice cambiato. Misure fatte sul sito pubblicato il 19/10/2026 (~17:00 UTC) e su costruzioni locali con la stessa catena della CI.

## 0. Sintesi

- I moduli .wasm sono 25: il motore, le librerie, 15 in `wasm-feat/` e 8 lettori. Pesano 11,3 MB grezzi e 5,06 MB gzip. GitHub Pages serve solo gzip (niente brotli).
- **Gli studenti ricevono al massimo 5 gruppi di moduli**: motore (662 KB gzip), librerie, grafo (Disegno) e il lettore della loro famiglia. **I 14 moduli `wasm-feat` restanti, tranne grafo, sono solo per l'amministratore** (mzFinder/Tabui è nascosto). Il tetto stretto serve davvero solo per il motore.
- Ogni modulo separato costa circa 3 richieste in fila (manifest, glue JS, wasm): con RTT 130 ms sono ~0,4 s prima ancora dei byte. Fondere conviene quando il modulo è piccolo e quasi sempre usato insieme all'altro.
- Fondere il gruppo HR (detect, proofs, mrm, finder, finder-edge, finder-msn) dà 469 KB gzip contro 913 KB: **-48,6 %**, perché il codice condiviso si paga una volta sola. Una corsa HR tipica passa da 586 KB a 469 KB e da 9 a 3 richieste.
- I tetti stretti dei moduli admin (chem 0,9 %, export 1,0 %, finder-lr 3,2 %, attributio-rev 3,1 %) e di alcuni lettori (sciex 246 byte, andi 2 KB) causano cicli di CI inutili: nessuno studente ci guadagna.
- Proposta: tetti in **un solo file**, regola «misura x 1,10» alzabile nella stessa PR con una riga di nota; non alzabile solo il motore.

## 1. Inventario

| modulo | grezzo (B) | gzip (B) | tetto (B) | margine | pubblico | quando si carica |
|---|---:|---:|---:|---:|---|---|
| motore (`wasm/`) | 1 544 800 | 661 540 | 1 703 936 | 159 136 (9,3 %) | tutti | all'avvio della vista Dati (compilato una volta, condiviso dai worker); non su telefono |
| librerie (`wasm-lib/`) | 392 372 | 175 602 | 614 400 | 222 028 (36,1 %) | tutti, a richiesta | alla prima operazione sulle librerie spettrali |
| grafo (`grafo`) | 199 739 | 95 557 | 512 000 | 312 261 (61,0 %) | tutti, a richiesta (Disegno) + admin | al primo taglio di legame in Disegno; in Tabui quando c'è uno SMILES |
| detect (`feat`) | 418 535 | 189 169 | 512 000 | 93 465 (18,3 %) | admin | a ogni corsa HR della Pipeline (un worker per file) |
| proofs | 450 840 | 199 796 | 512 000 | 61 160 (11,9 %) | admin | a ogni corsa HR con un genitore (dopo detect) |
| mrm | 226 910 | 105 236 | 512 000 | 285 090 (55,7 %) | admin | solo per serie MRM |
| finder | 436 302 | 197 466 | 512 000 | 75 698 (14,8 %) | admin | dal worker di Tabui alla prima rotta del Dig HR |
| finder-edge | 174 578 | 85 009 | 512 000 | 337 422 (65,9 %) | admin | alla prima di `identity_graph`, `polarity_pairs`, `origin_evidence` |
| finder-msn | 296 595 | 136 031 | 512 000 | 215 405 (42,1 %) | admin | solo se un file della serie è MSn |
| untargeted | 453 580 | 185 371 | 512 000 | 58 420 (11,4 %) | admin | solo con il motore «alla mzmine» scelto nella Pipeline |
| table | 309 967 | 143 846 | 512 000 | 202 033 (39,5 %) | admin | quando si mostra o si esporta una tabella |
| import | 178 755 | 85 869 | 512 000 | 333 245 (65,1 %) | admin | quando si apre una tabella di mzMine, FBMN/GNPS2, MS-DIAL |
| attributio | 432 382 | 192 461 | 512 000 | 79 618 (15,6 %) | admin | quando si sceglie una cartella di campagna |
| attributio-rev | 913 150 | 395 775 | 942 080 | 28 930 (3,1 %) | admin | alla prima revisione nel Banco 2b |
| attributio-export | 653 126 | 283 846 | 659 456 | 6 330 (1,0 %) | admin | quando si usa il menu Esporta |
| finder-lr | 717 979 | 325 241 | 741 376 | 23 397 (3,2 %) | admin | solo per serie full scan a risoluzione unitaria |
| chem | 564 322 | 254 901 | 569 344 | 5 022 (0,9 %) | admin | alla prima richiesta di reattività del genitore |
| lettori `aperti` | 405 594 | 189 504 | 413 696 | 8 102 (2,0 %) | tutti, a richiesta | solo se arriva un file: file mzML con Numpress, mzXML |
| lettori `andi` | 259 980 | 124 240 | 262 144 | 2 164 (0,8 %) | tutti, a richiesta | solo se arriva un file: `.cdf` netCDF/ANDI |
| lettori `thermo` | 523 943 | 234 509 | 540 672 | 16 729 (3,1 %) | tutti, a richiesta | solo se arriva un file: `.raw` Thermo |
| lettori `agilent` | 425 254 | 194 347 | 438 272 | 13 018 (3,0 %) | tutti, a richiesta | solo se arriva un file: `.D` Agilent LC/MS e varianti |
| lettori `sciex` | 327 434 | 156 153 | 327 680 | 246 (0,1 %) | tutti, a richiesta | solo se arriva un file: `.wiff` di altri QTRAP, varianti |
| lettori `waters` | 293 390 | 137 950 | 303 104 | 9 714 (3,2 %) | tutti, a richiesta | solo se arriva un file: cartelle `.raw` Waters |
| lettori `shimadzu` | 281 836 | 134 454 | 290 816 | 8 980 (3,1 %) | tutti, a richiesta | solo se arriva un file: `.qgd`, `.lcd` |
| lettori `bruker` | 421 690 | 177 154 | 434 176 | 12 486 (2,9 %) | tutti, a richiesta | solo se arriva un file: `.d` timsTOF/BAF |
| **totale 25 moduli** | 11 303 053 | 5 061 027 | | | | |

Margine = tetto meno grezzo. Gli stessi numeri sono in `tools/build_feat_wasm.sh` (feat), `tools/build_lettori_wasm.sh` (lettori) e nei tre workflow per motore e librerie (1 703 936 e 614 400, copiati in `pages.yml`, `sito-rust.yml`, `release.yml`).

Percorsi tipici (gzip, solo moduli):

| scenario | byte |
|---|---:|
| studente con un mzML | 661 540 |
| + librerie | 837 142 |
| + lettore Thermo | 896 049 |
| admin, Tabui HR (motore, detect, proofs, finder, table) | 1 391 817 |
| admin, Tabui HR + grafo | 1 487 374 |
| admin, Attributio (motore, attributio, rev) | 1 249 776 |
| admin, Attributio + export | 1 533 622 |

## 2. Costi reali della divisione

- **Richieste e latenza**: manifest, glue, wasm in fila. Misura dal runner: RTT ~0,13 s (0,13-0,44); il motore da 671 KB è costato solo 0,06 s oltre l'RTT. Il costo fisso per modulo è quindi ~3 x RTT.
- **Cache**: `sw.js` manda tutto network-first con `cache: "no-cache"` (GET condizionale a ogni uso); Pages dà `max-age=600`. Un modulo in più = una GET condizionale in più per sessione.
- **Compilazione**: 0,9-6,4 ms in Node per modulo (motore 6,4, rev 3,4, gli altri 1-2). Non è un argomento né pro né contro la divisione.
- **Codice condiviso duplicato**: sovrapposizione xz tra fratelli: feat+proofs 21 %, finder+edge 17 %, finder+msn 21 %, attributio+rev 22 %, rev+export 19 %, sciex+waters 27 %, andi+aperti 27 %, grafo+chem 12 %, motore+librerie 6,7 %.
- **Complessità**: 14 file JS leggono ognuno `manifest.json`; ~41 punti verificano SHA-256; `carica-modulo.js` esiste solo per i lettori; i tetti sono in almeno 6 posti e alcune doc sono fuori allineamento (shimadzu.md e waters.md 327 680, bruker.md 524 288, lettori-aperti.md 393 216, misure.md thermo 614 400 e agilent 458 752: lo script dice 290 816, 303 104, 434 176, 413 696, 540 672, 438 272).
- **CI**: il passo «mzFinder WASM engines» di `pages.yml` dura 3,5-5 min per 23 moduli; wasm-opt scala con i byte (detect 20 s, rev 52 s, gruppo HR fuso 59 s), quindi fondere risparmia solo i costi fissi. Il deploy ha atteso 24 min il 19/10 per la `concurrency`, non per i moduli.
- **Tetti come costo**: negli storici (14 commit su «tetto wasm», clone superficiale quindi numero minimo): detect oltre 460 800 -> mrm a parte; tetto 460 800 -> 512 000; chem: #1022, #1033, #1038 per stare sotto 569 344; #1067 attributio-rev alzato a 973 056 e poi riportato a 942 080 spostando la tabella delle classi (56 KB) in un file `classi-1.json`. Spesso la divisione è stata dettata dal tetto, non dal pubblico.

## 3. Guadagno della divisione

Costruzioni locali con wasm-bindgen 0.2.129 e la stessa catena (calibrazione: ±0,1 % sul pubblicato).

| insieme | separati (grezzo / gzip) | fusi (grezzo / gzip) |
|---|---|---|
| gruppo HR (detect, proofs, mrm, finder, edge, msn) | 2 002 848 / 912 589 | 1 070 889 / 469 309 |
| gruppo HR + untargeted | 2 456 236 / 1 097 973 | 1 320 337 / 560 678 |
| 8 lettori | 2 936 217 / 1 348 033 | 1 267 247 / 539 216 |
| attributio + rev + export | 1 998 658 / 872 082 | <= 1 133 531 / ~493 000 (stima, non compila: vedi §6) |

Costi marginali dentro un modulo HR (gzip): detect+proofs+finder 384 075; + edge + msn 445 035 (+61 KB); + mrm +24 KB; + untargeted +92 KB.

Punto di pareggio: un modulo X separato costa 3 x RTT + gzip/larghezza di banda; il pareggio è a ~750 KB gzip (fibra 20 ms, 100 Mbit/s), ~375 KB (wifi di campus 50 ms, 20 Mbit/s), ~300 KB (4G). Profili ipotizzati, calcolo e non misura sul campo. Regola pratica: **fondere X nell'ospite se gzip(X) < quota di sessioni dell'ospite che usano anche X x ~300 KB**.

I lettori fusi sarebbero 539 KB gzip per tutti, contro 125-235 KB della sola famiglia usata: non si fondono.

Resta vero a favore della divisione: (a) il pubblico (lo studente non scarica codice admin e il tetto del motore lo protegge); (b) la granularità della cache: i moduli admin cambiano ogni giorno, il motore molto meno; (c) le tabelle di dati vanno in file accanto al wasm, non dentro.

## 4. Verdetto per modulo

| modulo | verdetto | motivo |
|---|---|---|
| motore | resta; 1 703 936 non alzabile | tutti gli studenti all'avvio |
| librerie | resta separato; tetto troppo largo (+36 %) | a richiesta, 176 KB gzip |
| grafo | resta | pubblico Disegno; non fondere con moduli admin |
| detect, proofs, finder, finder-edge, finder-msn, mrm | **fondere in `mzfinder`** | -48,6 % gzip, 3 richieste invece di 9, il tetto per modulo scompare |
| table, import | fondere nel gruppo HR se si tollerano +230 KB grezzi; altrimenti restano | sono piccoli e usati con Tabui; da misurare prima |
| untargeted | resta | marginale 92 KB gzip, uso raro |
| finder-lr | resta | modalità diversa, 325 KB gzip |
| chem | resta, ma senza tetto proprio | solo admin, margine 5 KB: il tetto non protegge nessuno |
| attributio, rev, export | **fondere in uno** dopo aver tolto l'esclusività `cfg` | -43 % gzip stimato; gli ultimi 2 hanno margini di 29 KB e 6 KB |
| 8 lettori | restano per famiglia | un utente usa 1 famiglia; ma i tetti vanno allargati (sciex 246 byte, andi 2 KB) |

Tetti troppo larghi (512 000 contro 175-227 KB gzip): librerie, finder-edge, import, grafo, mrm. Tetti troppo stretti: chem, export, sciex, andi, aperti.

## 5. Regola proposta

1. **Tre gruppi per pubblico.** (a) Motore: tutti, all'avvio, tetto rigido e non alzabile. (b) A richiesta per tutti (librerie, grafo, lettori): tetto per modulo. (c) Admin (`wasm-feat` senza grafo): nessun tetto per modulo, un tetto per gruppo.
2. **Quando dividere.** Si divide solo se gzip(X) > quota d'uso insieme all'ospite x ~300 KB. Sotto ~150 KB gzip non si divide mai. Le tabelle di dati sono file, non pacchetti.
3. **Dove stanno i tetti.** In un solo file (es. `tools/wasm_tetti.txt`) letto da script, workflow e test; oggi sono in 6+ posti.
4. **Come si alza.** Tetto = misura x 1,10 arrotondato a 4 KiB, deciso alla nascita. Chi lo supera lo alza nella stessa PR fino a misura x 1,10 e aggiunge una riga di nota nel file d'area (misura, tetto, motivo). Decide il Coordinatore solo oltre +25 % in una PR o se si tocca il motore.
5. **Non alzabile**: solo il motore. Per chem Federico deve scegliere: alzare con la regola oppure toglierlo dal tetto per modulo.

## 6. Cosa cambia se si fonde (non fatto)

| pacchetto | cosa si tocca | rischio | stima |
|---|---|---|---|
| A. gruppo HR | `tools/build_feat_wasm.sh` (7 `wasm_pack` -> 1, chiavi del manifest che puntano allo stesso file, così i caricatori non cambiano); mappa in `tpmine-worker.js` e `feat-worker.js`; `32_pipeline.js`; `tests/test_feat_wasm.py` e `feat_wasm_node.js`; tetto in `e2e_attributio_pipeline.py:38`; `mzfeat-wasm.md`, `mzfinder.md` | basso; la combinazione compila già; ogni modifica invalida tutto il gruppo | 1 PR, ~mezza giornata + CI |
| B. attributio | `crates/mzlab-attributio-wasm/src/lib.rs`: `BRIDGE` ed `export_kind` definiti due volte (E0428 con le feature fuse, righe ~22-24 e ~154-164); 3 pacchetti -> 1; `50_attributio.js` e caricatore Banco; `e2e_attributio_banco.py` (3 tetti), `e2e_attributio.py:38`; 4 doc attributio | basso-medio | 1 PR, ~1 giornata |
| C. table, import, chem, lr, grafo | serve una nuova crate che li raccolga; grafo non va con i moduli admin | medio, guadagno modesto | non consigliato ora |
| D. lettori | nessuna fusione; unire solo i tetti e allineare le doc | basso | 1 PR piccola |
| Comune | sostituire i 14 lettori di `manifest.json` con un caricatore unico (promessa del manifest in cache) | basso | 1 PR |

## 7. Metodo e limiti

- Dimensioni dei moduli pubblicati: scaricati dal sito, gzip misurato (brotli solo informativo). Fusioni: costruzioni locali, stessa catena della CI (cargo release `opt-level z`, lto, `wasm-bindgen --target web`, `wasm-opt -Oz`); scarto sul pubblicato ±0,1 %.
- La fusione di attributio è una stima (non compila senza modificare il codice).
- I profili di rete del punto di pareggio sono ipotesi; l'RTT è misurato dal runner via proxy, non da un client reale.
- Lo storico dei tetti è limitato dal clone superficiale; i 14 commit sono un minimo.
- Documento in `docs/studi/`, quindi fuori dai limiti di `docs/agenti/`.
