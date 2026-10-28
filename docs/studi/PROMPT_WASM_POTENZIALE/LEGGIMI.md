# Prompt dello studio «potenziale di WebAssembly»

Misure, audit e strategia in `docs/agenti/wasm-potenziale.md`. Ogni prompt è autosufficiente: si copia la parte dopo «---». Ogni DEV è Sonnet, un prompt per chat, perimetri disgiunti.

Ordine: [DEV-5](DEV-5.md) (banco permanente) per primo; poi [DEV-1](DEV-1.md) (binaryen 132) e [DEV-3](DEV-3.md) (SIMD128) in parallelo; [DEV-2](DEV-2.md) (O3 sui crate caldi) e [DEV-4](DEV-4.md) (zlib-rs) solo dopo la decisione di Federico (§8 dello studio) e dopo DEV-1 e DEV-3, uno alla volta (toccano entrambi i tetti).

`banco.txt`: i sorgenti del banco usato dallo studio (crate di prova, generatore del mzML HR sintetico, suite JS, pagina per Chromium, script delle varianti), punto di partenza di DEV-5.
