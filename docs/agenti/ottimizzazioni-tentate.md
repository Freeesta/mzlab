# Registro delle strade di velocità già tentate

Da leggere PRIMA di ottimizzare caricamenti o velocità. Le misure riuscite stanno in `misure-velocita.md`; qui stanno anche le strade scartate, rimandate o rimaste a metà, così non si rifanno. Una riga per strada, la più nuova in fondo; si aggiunge nella stessa PR che tenta o scarta la strada. Prima di riprovare una strada «scartata» serve una condizione nuova (campione diverso, macchina diversa, tetto cambiato) scritta nella PR.

Esito: **adottata** (vedi `misure-velocita.md`), **scartata** (numeri sotto soglia), **rimandata** (serve una decisione di Federico o la Mac), **a metà**.

| Data | Strada | Esito | Perché / numeri | Riprovare solo se |
|---|---|---|---|---|
| 22/10 | `.wasm` con `simd128` (20 Full Scan LR) | scartata | 9,1 s con contro 9,3 s senza; `.wasm` 773 contro 776 KB (`misure-diario.md`) | il carico cambia molto (non i Full Scan LR) |
| 24/10 | `.raw`: tabella e «table + .mzidx» senza SHA-1 né `metodo/1` | adottata | −43…−71 % (raw-fast-mzidx-1) | — |
| 25/10 | SIMD (AVX2) nella decodifica `.raw` (`to_mz_intensity_into`) | scartata | solo m/z 3,04 → 2,06 ns/punto (−14 % della sola conversione), centroidi senza margine, riscrittura sicura a due passate più lenta (9,4 ns/punto) | arriva un corpus con molti `.raw` in profilo |
| 25/10 | Cache a blocchi nel lettore wasm (RAW-FAST-IO-WASM) | adottata, ridotta | prima versione: andi +537 B oltre il tetto; riuscita con 3 blocchi solo per thermo; tetti invariati; andi ha 192 B di margine, ogni modifica a `lettori-wasm/src/lib.rs` lo rischia | un tetto viene alzato con misura (tetto = misura ×1,10 a 4 KiB) |
| 25/10 | zlib-rs al posto di miniz | rimandata | si resta su miniz; feature `banco-zlib-rs` già in `mzlab-core`; decide un guadagno ≥ 10 % su centroidi di `.raw` veri (da misurare sulla Mac) | misura su `.raw` veri ≥ 10 % |
| 25/10 | Allocatore globale (contesa su `realloc` nel pool parallelo) | rimandata | richiede `unsafe` nei binari nativi: decisione di Federico | l'opzione B (meno `realloc`) non basta |
| 25/10 | PARALLEL-CACHE, opzione B: meno `realloc` | a metà | in corso (chat 01WmSnmR); fermarsi se il pool mzML resta sotto 1,5× | — |
| 25/10 | `--threads` dello scrittore mzML (#1731) | adottata | vedi `misure-velocita.md` | — |
