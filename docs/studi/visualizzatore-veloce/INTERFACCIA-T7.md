# Interfaccia fra il blocco colonnare di T-7 e il visualizzatore

T-7 (`docs/studi/PROMPT_LETTORE_THERMO/T-7.md`, fasi T-7d/e) produce senza perdite il blocco colonnare opzionale del `.mzidx`; il visualizzatore lo legge e basta.

Decisioni del Coordinatore già prese e qui solo richiamate: le colonne sono un blocco opzionale nuovo del `.mzidx`, non un terzo formato; i `.mzidx` esistenti restano uguali byte per byte finché il blocco non è chiesto; versione di formato dichiarata (`error.newerFormat`); la centroidatura nuova è spenta di base e dichiarata con perdita nel passo `spectra.centroid`; il lettore fedele non centroida mai da solo. Questo file NON definisce il formato su disco (è di T-7): definisce che cosa il visualizzatore ne chiede.

## 1. Che cosa manca oggi al visualizzatore
Il `.mzidx` ha già l'asse «per m/z» (picchi di una sezione ordinati per m/z, con la scansione in `pos`): XIC e mappa non chiedono altro (sonda in `CRITERI.md` §2: 0,1 ms su 27 M picchi). Manca l'asse «per scansione»: uno spettro di una scansione si rilegge dall'mzML (decodifica base64/zlib) o dal modulo del produttore (99 µs per scansione in WASM per un `.raw`, più la meta JSON). Il blocco di T-7 dà proprio questo.

## 2. Requisiti del blocco, visti dal visualizzatore
- **R1 Accesso per scansione**: per ogni riga della tabella delle scansioni (stessa numerazione di `pos` e di `idxread::Scan`), i picchi in un intervallo contiguo dopo la decodifica: offset `u64[n_righe + 1]`, `mz` e `intensità` in colonne separate, nell'ordine del file. Uno spettro = una fetta.
- **R2 Senza perdite**: i valori decodificati sono quelli del lettore fedele (impronte A4 di T-7 identiche); mai Numpress né arrotondamenti. Tipo dichiarato per colonna (f64 per m/z; f32 o f64 per intensità come la sorgente).
- **R3 Accesso casuale a pezzi**: compressione (delta + rimescolamento dei byte + zstd, B2) per gruppi di scansioni con una tabella dei pezzi (offset e lunghezza in byte, prima riga del pezzo), così uno spettro decodifica al più un pezzo. Obiettivo per il visualizzatore: spettro di una scansione HR da 5 000 picchi in ≤ 1 ms nativo e ≤ 3 ms nel sito, compresa la decompressione (T-7 sceglie la dimensione del pezzo e la riporta).
- **R4 Profilo e centroidi**: per ogni riga un indicatore profilo/centroidi; dove il file ha sia il profilo sia il flusso dei centroidi del produttore, due gruppi di colonne distinti e nominati (il visualizzatore mostra ciò che mostra oggi: i centroidi del file dove ci sono, il profilo su richiesta).
- **R5 Colonne extra opzionali**: rumore, linea di base, risoluzione, carica dei centroidi FT (A3) come colonne nominate e opzionali; il visualizzatore le usa solo nei suggerimenti, mai per disegnare.
- **R6 Lettura**: un'API di `mzlab-core` sopra `ReadAt` (stessa del resto di `idxread.rs`), per esempio `Index::has_cols()`, `Index::scan_peaks(riga, &mut mz, &mut y)` o una vista in prestito senza copie dove la memoria è mappata (desktop). Nessuna allocazione proporzionale al file; pezzi in una cache con limite (come i blocchi di 16 384 di `search`).
- **R7 Presenza dichiarata**: il blocco si riconosce dal meta (chiave nominata e versione); senza il blocco ogni rotta resta quella di oggi. Un `.mzidx` con il blocco letto da un programma più vecchio dà `error.newerFormat` solo se T-7 lo decide; il visualizzatore non dipende da questa scelta.
- **R8 Asse m/z invariato**: le sezioni attuali (ordinate per m/z) restano la via dello XIC; se T-7d costruisce anche la matrice scansione × secchio (B3, per la feature detection), il visualizzatore non la usa per lo XIC (finestra esatta, `CRITERI.md` D3).

## 3. Che cosa fa il visualizzatore con il blocco (V-6)
- `api/spectrum` di una scansione, `api/scanbin`, lo scorrimento con le frecce e la lente (V-5, spettro accanto) leggono dal blocco quando c'è; uscita uguale byte per byte a quella della via mzML/produttore sugli stessi file (prova di parità), nessun ripiego silenzioso: senza blocco, la via di oggi.
- Numeri: G1 e G6 di `CRITERI.md` prima/dopo, sui sintetici e (chat locale) su un Orbitrap del corpus.
