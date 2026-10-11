# Tracce di massa in Rust (`crates/mzlab-feat`)
Stato attuale. Il crate porta `TP_Mine/py/tpmine/hr/features.py::detect` (livello L1 del disegno di `studio/PIPELINE_IDENTITA.md` nel repository dei dati): dai picchi MS1 di un `.mzidx` (una polarità) produce la tabella delle tracce di massa di un file. Il Python resta il riferimento dei test finché il modulo non ha la parità. Solo LC-HRMS centroidi, solo MS1; le polarità si trattano separate (`Params.polarity`).

## Moduli
- `trace.rs`: `Params` (predefiniti di `detect`: canali 4 ppm, soglia di rumore 2e4, altezza minima 2e5, punti minimi 5, buchi ≤ 2 scansioni, tracce di fondo > 400 scansioni, `budget` in byte) e `detect(&mut Index<R>, &Params) -> Result<FileFeatures>`. I picchi si leggono con `idxread::Index::each_peak` a pezzi di 64 K, si tengono solo quelli sopra la soglia e si formano lastre di m/z di circa `budget / 64` picchi (al massimo 1,5 M).
- `channels.rs`: `segments` (celle (canale, scansione) sommate, segmenti con buchi ≤ `max_gap`, canali con massimo < altezza minima / 4 scartati subito) e `stitch` (cucitura dei pezzi di uno ione fra canali vicini: componenti connesse con numerazione per primo membro, come `np.unique` sulle etichette di Python).
- `post.rs`: `merge_grids` (stesso ione visto sulle due griglie sfasate: resta quello con più punti), `fourier_artefacts` (lobi FT), `finish` (ordine per m/z stabile e colonne finali).
- `table.rs`: `FileFeatures` (`mz f64`, `rt/left/right f32` in minuti, `apex/scan_lo/scan_hi u32` come posizioni nella sezione MS1, `area f64` in conteggi × secondi, `height f32`, `npts u16`, `flags u8`, `iso_group u32`, `charge i8`, `dt` = intervallo mediano fra le scansioni in secondi) e `to_bytes()` (layout fisso, base dello SHA-256 dell'uscita). Bit di `flags`: 0 fondo, 1 lobo FT, 2 risolta, 3 isotopo, 4 qualità buona, 5-7 riservati; qui si scrivono solo 0 e 1.

## Determinismo e WASM
- Una lastra finisce solo dove due picchi consecutivi sopra la soglia distano ≥ 50 ppm (nessuna traccia attraversa quel salto: i canali sono di 4 ppm), quindi il risultato non dipende dalla dimensione delle lastre. Le righe delle due griglie si raccolgono per griglia e non per lastra, così anche l'ordine di ingresso è indipendente dalle lastre.
- `ln` e `log1p` da `libm` (Rust puro); somme `f64` in ordine fisso; nessun `HashMap`; ordinamenti stabili. La memoria si riserva con `try_reserve` e l'errore è `error.memoryBudget`, mai il panico.
- Dipendenze: `mzlab-core` e `libm`. `cargo check --target wasm32-unknown-unknown -p mzlab-feat` e il controllo del grafo (nessun `memmap2`, `zstd-sys`, `libz-sys`, `cc`) stanno in `rust.yml`.

## Parità con Python
- `python3 tools/feat_riferimento.py OUT [--golden]` scrive cinque file sintetici (ioni inventati: deboli, intensi con un lobo FT a 60 ppm, due ioni a 3 ppm, fondo continuo, file vuoto), i loro `.mzidx` costruiti da Python e `detect_<nome>.json` (con `--golden` anche in `tests/golden/feat/`). Nel file «fondo» `long_trace` vale 60 perché le tracce sintetiche non arrivano a 400 scansioni.
- `cargo test -p mzlab-feat` (`tests/trace_parity.rs`; salta senza python3 e numpy, `MZLAB_GOLDEN_STRICT=1` fallisce): stesso numero di feature, apice e bordi identici, m/z entro 1e-6 relativo, aree entro 1e-4, stessi flag; lastra di mille picchi (budget 1) = lastra grande; due esecuzioni = stessi byte.

## Misura (`examples/feat.rs`)
`cargo run --release -p mzlab-feat --example feat -- FILE.mzidx... [--neg] [--budget-mb N]` stampa per file tempo, picco di memoria (`VmHWM`, un processo per file), numero di feature e SHA-256 della tabella. Su 13 file reali Orbitrap da circa 80 MB (3,8 M picchi MS1 ciascuno): 1 259 216 feature in totale, le stesse di `features.detect` file per file; 0,5-0,7 s e 62-78 MB di picco per file (Python: 3,5 s e 565 MB); due esecuzioni e budget di 8 MB (picco 42-43 MB) danno lo stesso SHA-256.

## Trappole
- Il `.mzidx` di un MS1 ad alta risoluzione tiene m/z e intensità in `f32` (`hr1`): `each_peak` li allarga a `f64`, come fa Python con `astype(float64)`; le soglie (2e4, 2e5) sono esatte in `f32`, quindi i confronti coincidono.
- La somma di una cella o di un segmento è sequenziale; `np.add.reduceat` somma a coppie sopra 8 elementi: la differenza è sotto 1e-15 relativo e le aree si confrontano a 1e-4.
- Il canale di un picco è `floor(ln(m/z) / ln(1 + 4 ppm) + sfasamento)`: `libm` e `numpy` possono differire all'ultimo bit e spostare un picco al bordo di un canale; nei dati provati non è mai successo.
- `tools/controlla.py` boccia file oltre 500 righe e funzioni oltre 80: per questo il crate ha tre moduli e `stitch` è diviso in `link` e `join`.
