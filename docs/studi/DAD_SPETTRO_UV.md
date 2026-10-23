# Spettri UV dal DAD (doppio clic sulla mappa PDA e sulla traccia PDA)

**Domanda**: estrarre gli spettri UV-Vis singoli dalla vista DAD con il doppio clic. Con i file di demo non funziona.

## Cosa c'è (verificato nel codice)
- **Matrice tempo × lunghezza d'onda**: solo nel `.wiff` SCIEX del laboratorio con il rivelatore (`wiff/dad.rs`, `DadData`, riga per tempo = `spectrum_at`). Il motore la legge direttamente dal `.wiff`; un mzML convertito porta solo la traccia TWC.
- **Demo**: gli mzML sintetici (`tools/dati_sintetici.py`, `_pda`) hanno solo la traccia PDA «TWC» (1D), mai la matrice. Thermo `UV_VIS_n` (traccia, lo spettro 3D no), Agilent `DAD1.*` e Shimadzu (non decodificati): `docs/studi/FORMATI_NATIVI_MS.md`, riga UV/PDA. Quindi nessuna demo può dare spettri UV; serve un `.wiff` con DAD.
- **Oggi**: la mappa PDA (`/api/map?kind=pda`, selettore MS/PDA) e la traccia PDA (`kind=pda`) si vedono; il doppio clic sulla mappa apriva uno spettro di MASSA a quell'RT (sbagliato) e sulla traccia PDA uno spettro di massa con lo stesso RT.

## Decisioni
1. **API Rust** (wasm + nativo): `/api/uvspec?k=&rt0=&rt1=` → `{nm, mau, rt0, rt1, n, unit}`: media delle righe in [rt0, rt1], la riga più vicina se nessuna cade dentro. Niente rotte nuove nel pannello: stesso percorso di `/api/map?kind=pda` (`itemroutes`, trait `Lettore::dad_spectrum`).
2. **Pannello**: un pannello `spec` con `uv: true` (`uv-spettro.js`, modulo caricato al primo uso): asse nm, assorbanza mAU, tabella dei dati, export, zoom da tastiera. Doppio clic sulla mappa PDA e sulla traccia PDA (azione `chrom.uvAtRt`, registro `azioni/1`); con una selezione sulla traccia, anche la media (`chrom.uvAvgSel`). Il ritardo «Allinea PDA e massa» è tolto dal tempo.
3. **Senza matrice**: messaggio `uv.none` («solo i file `.wiff` SCIEX con il rivelatore DAD») e nessun pannello.
4. **Test**: `.wiff` sintetico con DAD già disponibile (`tools/wiff_riferimento.py`, `dad=`), usato da `tests_e2e/e2e_uv_spettro.py`. Una demo con DAD sintetico dichiarata come tale è possibile con lo stesso scrittore, ma serve una decisione di Federico (i demo sono dati anonimi di `mzlab/web/esempi/`).

## Non fatto / aperto
- Nessun λmax o assegnazione automatica (principio 1): il valore si legge da cursore e tabella.
- Valori del DAD non ancora confrontati con un export di SCIEX OS (`wiff-varianti.md`).
