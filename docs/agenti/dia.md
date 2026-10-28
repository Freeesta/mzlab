# Pseudo-spettri MS2 da DIA (`dia.rs`)

Da una corsa DIA (SWATH, MSE) ricava uno pseudo-spettro MS2 per ogni feature: i frammenti la cui traccia segue quella del precursore. Spento di default; nessuna deconvoluzione MCR (`deconvolve.rs` resta solo DDA).

## Motore: `crates/mzlab-feat/src/dia.rs`
- Funzioni pure su fette (`DiaScan`, `Prec`, `DiaParams`), niente file né thread, somme in ordine fisso: nativo e WASM danno gli stessi bit. `pseudo_ms2(scans, precs, params) -> Vec<Pseudo>`; picchi `(m/z, intensità all'apice, correlazione)` in ordine di m/z. Le scansioni devono avere i picchi in ordine di m/z.
- Per precursore: finestra = quella degli MS2 che ne contiene l'm/z (col centro più vicino; `iso` assente = tutti gli ioni, MSE); cicli = MS2 di quella finestra con RT nell'intervallo (meno di `min_points`: nessuno pseudo-spettro); traccia del precursore = massimo degli MS1 entro `frag_ppm`, interpolato ai cicli; candidati = picchi del ciclo all'apice ≥ `min_rel` × picco base; tenuto se Pearson ≥ `min_corr` e apice entro `max_shift` cicli (default 0,9; 5; 1; 10 ppm; 0,01).

## CLI: `spectra.ms2@1`
- Parametri `dia_pseudo` (false), `dia_min_corr` (0,9), `dia_min_points` (5), `dia_frag_ppm` (10). Con `true` serve la porta `features` (assente: `err.flow.port.missing`, `port=features`); intervallo RT = RT ± `rt_tol`; un record MGF per feature con pseudo-spettro, TITLE `<file>.<id>.pseudo`, PEPMASS = m/z della feature, RTINSECONDS = apice. Gli MS2 DDA non si scrivono. Con `false` l'uscita è quella di sempre. Finestra di isolamento letta da `MS:1000827` ± offset `MS:1000828/9`.
- Prove: `crates/mzlab-feat/tests/dia_pseudo.rs` (lotto sintetico: 600 s, 4 finestre, 40 precursori, 8 interferenti per finestra a Δapice 6-12 s, fondo e picchi isolati), `crates/mzlab-cli/src/flow_ms2_dia_tests.rs`.

## Misure sul lotto sintetico (frammento per frammento, entro 10 ppm)
recall 0,981 (veri con intensità all'apice ≥ 3× rumore), precisione 0,995, precisione sulle coppie co-eluenti 0,994.

## Limiti
- Isomeri che eluiscono insieme (Δapice 0): i frammenti finiscono in entrambi (asserito dal test).
- Finestre sovrapposte: vale la finestra col centro più vicino, non si uniscono.
- Risoluzione unitaria e dati veri non provati: solo dati sintetici.
