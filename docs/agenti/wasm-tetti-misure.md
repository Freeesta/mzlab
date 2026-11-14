# Misure dei moduli .wasm
Il record di un modulo è il campo `misura` del suo `tools/moduli/<nome>.toml` (byte dopo `wasm-bindgen` e `tools/wasm_opt.sh`, binaryen 133) più la riga prima → dopo nel corpo della PR che lo fa crescere. Il campo lo scrive solo `python3 tools/moduli.py --misura <site/static>` da una build vera (motore e librerie con i comandi del passo «libraries» di `sito-rust.yml`, il resto con `tools/build_feat_wasm.sh <dir> all`); a due esecuzioni di fila il secondo risultato è identico, e `--controlla` fallisce se una misura supera il tetto. I valori del passo «size» della CI fanno fede.
Una riga di misura in PR dice: modulo, byte prima → dopo, crescita in %, e se è «grezzo» (`tools/cargo_wasm.sh build --release` prima di `wasm-opt`) o ottimizzato. Il tetto non si cambia qui: regola e controllo in `wasm-tetti.md`.

## Cosa costa di più (regole che le misure passate hanno fissato)
- Il controllo e la ricerca per sottostruttura di `mzlab-mol` pesano circa 95 KB nel modulo che li contiene: per questo `molkit` è un modulo a parte e non sta dentro `grafo` (+61 %). Dentro `molkit` ogni funzione cresce meno del 10 % (CIP +7 %, export MOL/SDF +6 %, colori +3 %), la ricerca per sottostruttura da sola +44 % su un modulo ancora piccolo.
- Tre passi di tabella (`table.stats`, `table.pca`, `table.kmd`) dentro `flowtab` lo portavano da 373 402 a 541 412 B (+45 %): stanno in `flowstat`. Un `HashSet` in `ionnet::group` costava +12 KB: si usa la bisezione.
- Un modulo che riceve l'intero lettore SCIEX porta le tabelle Unicode (+9,4 KB in `waters`): `FileSet::find` nel rifiuto resta solo con il lettore SCIEX.
- `slice::sort*` di std costa 2-10 KB per istanza; `sortc` e `order_by_f64` (`wasm-tetti.md`) evitano la copia.
- `libm` al posto di `exp`/`ln`/`pow`/trig di std cambia il `.wasm` di meno dell'1 % (`mzlab-chem` tiene std per `fm.rs`).
- Un errore con i valori del suo testo (`Error::new` con parametri) costa circa 0,01-0,1 % per modulo: nessun modulo si avvicina al tetto per questo.
- Nel motore una funzione di lettura costa in media 0,1-0,3 %; le crescite oltre l'1 % (`.wiff` dispositivi +1,3 % in `sciex`, `isotopes = finder` +4,4 % in `hr`) hanno ognuna la propria riga nella PR.

## Passaggi di spostamento (perché un percorso sta dove sta)
- Le rotte (`stat_post`, `molecules`, `campioni`, `flow_*`, integrazioni manuali) sono fuori dal motore: il motore passò da 2 612 789 a 1 973 927 B (gz −23 %, br −23 %, V8 7,5 → 5,7 ms); `rotte` pesa 800-860 KB.
- `mzlab-core` a opt-level 3 sposta i tetti di `hr`, `untargeted`, `table`, `finder-lr`, `msngrafo`, `msnregole` di circa +10 % ciascuno (misura × 1,10 a 4 KiB); i lettori `aperti` e `sciex` contano la scansione danneggiata lasciata fuori (circa +2 KB ciascuno).
- Le parti danneggiate che un lettore lascia fuori e conta (`bruker`, `agilent`, `waters`, `.wiff`) costano +0,3-0,6 % ciascuna sul lettore e +0,1-0,3 % sul motore.
- L'indice a blocchi, la stima della lettura (`budget_mb`) e i riempimenti dei vuoti spostano `hr`, `table`, `msngrafo`, `formula` e `flowtab` di +0,2-0,5 %, mai oltre il tetto.

## Misure per PR
- `untargeted`, opzione `ident.min_corr`: grezzo (`tools/cargo_wasm.sh build --release`, prima di `wasm-bindgen` e `wasm-opt`, che non c'erano nella misura) 1 343 982 → 1 349 550 B (+0,41 %); la misura ottimizzata (470 779, tetto 585 728) resta sotto il tetto con ampio margine.
