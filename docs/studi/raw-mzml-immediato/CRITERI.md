# Criteri di accettazione (scritti prima del codice)

Ogni criterio ha un numero, una prova e un esito atteso. Le soglie si cambiano solo con una decisione del Coordinatore scritta qui, mai per far passare una PR. «Corpus HR» indica: LAB-A, LAB-B, BAF, INF-A, i file Thermo del corpus della locale (R-03) e i file pubblici di `RICHIESTE_LOCALE.md` R-e. Le regole di T-7 (A1-A4 prima di ogni velocità; due bersagli; `unsafe` isolato; dipendenze ≥ 10 %) valgono anche qui.

## F — Fedeltà dell'mzML
- **F1 Lista dei centroidi**: m/z (f64) e intensità uguali al bit a RFR e a msconvert `peakPicking vendor` sul 100 % delle scansioni FT del corpus HR, con lo stesso numero di picchi. Già vero su LAB-A contro msconvert; atteso: passa.
- **F2 Colonne per picco** (carica, flag, risoluzione, rumore, linea di base): uguali a `CentroidStream` di RFR sul 100 % dei picchi. Carica e flag interi esatti; float al bit o, se RFR interpola, differenza dichiarata con la causa. Scritte con i termini CV standard (MS:1000516 charge array, MS:1002529 resolution array, MS:1002742 noise array, MS:1002530 baseline array; i nodi del rumore con MS:1002743/1002744/1002745), con la lunghezza della lista; i flag senza termine CV come MS:1000786 con nome. **Può fallire** finché D1 non è chiuso.
- **F3 Profilo**: i punti non nulli sono uguali al bit a RFR e msconvert sul 100 % delle scansioni. Con l'opzione degli zeri «come RFR», l'intero array (numero di punti e valori, zeri compresi) è uguale a msconvert sul 100 % delle scansioni; gli zeri aggiunti sono dichiarati con un `userParam` per spettro e contati in `conv/1`. **Può fallire** finché D2 non è chiuso.
- **F4 Ricostruibilità del profilo**: dall'mzML (m/z, coefficienti e fudge scritti per scansione) si ricalcolano i bin di frequenza del file per il 100 % dei punti; le intensità sono f32 esatte (array a 32 bit). È un test di giro in Rust.
- **F5 Metadati**: filtro, tutti i campi del trailer, lo stato e la sintonia più vicini e il testo del metodo sono presenti. Su 100 % delle scansioni sono uguali a msconvert: livello MS, polarità, analizzatore, potere risolutivo (MS:1000800), attivazione (CID/HCD/ETD/EThcD/UVPD), energia, m/z e carica del precursore, finestra d'isolamento, RT. Oggi fallisce: su LAB-B 16 038 MS2 `@cid` su 16 038 escono come HCD; `is_dia=true` su MS2 mirate; MS:1000800 assente (msconvert lo scrive su ogni scansione).
- **F6 Conti**: numero di spettri = numero di scansioni del file; TIC e BPC del cromatogramma uguali a msconvert (f32 esatto).
- **F7 Niente inventato**: un campo assente nel file è assente nell'mzML. Gli zeri esistono solo con l'opzione F3 e sono marcati.
- **F8 Validità**: 0 errori con il validatore PSI (schema 1.1.0 + regole semantiche), OpenMS `FileInfo -v`, pyteomics (`mzml.read` + `IndexedMzML`), mzR (`openMSfile`) e msconvert che rilegge. Tutti gli offset dell'indice puntano al primo byte di `<spectrum ` / `<chromatogram `; l'SHA-1 di `fileChecksum` è giusto. Oggi la CLI: valido, 0 errori (validatore nostro).
- **F9 Determinismo**: SHA-256 dell'uscita uguale con 1/2/4/N thread, su x86-64 Linux, arm64 macOS e `.wasm`, con gli ingressi permutati e in due giri. Atteso: 100 %.
- **F10 Compatibilità**: per i file e le opzioni che la PR non cambia, `raw_equivalenza.rs`, `raw_parity.rs` e `msntree_parity.rs` restano verdi. Dove cambia, prima/dopo su un file nella PR e versione `conv/1` aumentata.

## C — Centroidatura nostra (solo scansioni senza lista)
Si applica solo a scansioni senza lista dello strumento o a `--peaks mzlab`. Mai nel percorso di base delle scansioni FT.
- **C1 Simulati** (RM-6, ≥ 10 000 picchi, forma di riga FT da LAB-B, rumore gaussiano + poissoniano, rapporto S/N da 3 a 10⁴):
  - |Δm/z| con mediana ≤ 0,2 ppm (o ≤ 0,02 FWHM, la più larga) e 99° percentile ≤ 1 ppm;
  - area con mediana ≤ 1 % relativo;
  - due picchi uguali a 1,0 FWHM risolti in ≥ 95 % dei casi;
  - fantasmi ≤ 0,1 % dei picchi restituiti sopra 3σ;
  - richiamo ≥ 99,5 % per S/N ≥ 10.
- **C2 Contro la lista dello strumento** (scansioni FT con entrambi; LAB-B e corpus): precisione e richiamo ≥ 0,98 entro 1 ppm per S/N ≥ 10; |mediana del rapporto d'intensità − 1| ≤ 2 %.
- **C3 Calibranti e timololo**:
  - |mediana errore nostro − mediana errore dello strumento| ≤ 0,1 ppm;
  - rapporti isotopici entro 1 % relativo di quelli dello strumento.
  
  Verifica preliminare su 11 MS1 di LAB-B: Δ 0,08 ppm; ³⁴S 4,20 % contro 4,20 %; ¹³C 14,46 % contro 14,47 %. Atteso: passa, ma il campione è piccolo.
- **C4 Trappola ionica** (small.RAW, IT-HCD-SPS): picchi a ±0,05 Th da quelli di msconvert `peakPicking vendor`, precisione e richiamo ≥ 0,95. **Può fallire** (l'algoritmo del produttore è ignoto): se fallisce, riportare i numeri e decidere con il Coordinatore.

## V — Velocità e memoria
Numeri nel container a 4 core di `MISURE.md` (nativo) e sul Mac della locale (cache fredda, 5 giri, mediana). Il pavimento T_min si calcola con la formula di `RISPOSTE.md` §1.3, dai micro-banchi di RM-0 sulla stessa macchina.
| N | Misura | Oggi | Obiettivo |
|---|---|---|---|
| V1 | LAB-B, `mzlab-cli convert` indicizzato, livello predefinito, 4 core | 5,97 s | ≤ 0,6 s e ≤ 3× T_min |
| V2 | LAB-A, come sopra | 3,96 s | ≤ 0,5 s e ≤ 3× T_min |
| V3 | BAF (595 MB), come sopra | da misurare (RM-0) | ≤ 3× T_min; più veloce di msconvert e di TRFP sullo stesso Mac |
| V4 | Denso sintetico (≥ 2 GB, ≥ 200 000 punti per scansione a profilo) | da misurare (RM-0) | senza compressione ≤ 1,5× B_out/BW_scrittura; con zlib ≤ 1,3× W_cpu/N |
| V5 | Scalabilità 1…N thread | 3 thread più lenti di 1 | speedup ≥ 0,7·N fino a 4 core; mai più lento di 1 thread su nessun file. **Può fallire** se il disco o l'SHA-1 saturano prima: allora si dimostra con la roofline |
| V6 | RSS di picco, per fase | 263-297 MB (`raw_tempi`); ≤ 66 MB (CLI) | ≤ 64 MB + 16 MB × thread + 4 × la scansione più grande, costante con la dimensione del file (prova sul denso da 2 GB e su uno da 8 GB) |
| V7 | Allocazioni a regime | ~10 % del tempo in malloc/free | ≤ 1 allocazione ogni 1000 scansioni (allocatore contatore nel banco) |
| V8 | Browser (`.wasm`) | da misurare (RM-5) | 1 worker ≤ 1,7× il nativo a 1 thread; W worker ≥ 0,6·W; byte uguali al nativo (F9); `.wasm` +≤ 10 % dichiarato |
| V9 | Tempo al primo spettro scritto | da misurare | ≤ 50 ms dopo l'apertura su ogni file |
| V10 | Compressione predefinita | miniz L1 | si sceglie zlib-rs L1 a flusso riusato se sul corpus HR riduce la dimensione ≥ 15 % con tempo ≤ 1,3× senza compressione; altrimenti nessuna compressione. Decisione con la tabella di RM-3 |

## E — Prove esterne (chi le fa, cosa si confronta, tolleranza)
- **E1 RFR** (locale): dump `scandump/1` completi di profilo e `CentroidStream` (R-b) confrontati con `tools/rosina/scandump_compare.py`. Tolleranza: al bit (F1-F3).
- **E2 msconvert** pwiz con `peakPicking vendor` e senza. Si confrontano: numero di spettri, punti, zeri, metadati CV (F3, F5, F6). Tolleranza: al bit sugli array, uguaglianza dei termini CV.
- **E3 ThermoRawFileParser**: solo con l'autorizzazione di Federico (SCAMBIO_LOC). Stessi confronti di E2 e tempi.
- **E4 Calibranti** (R-a): errore di massa e rapporti (C3).
- **E5 Validatori**: i cinque di F8 e gli offset.
- **E6 Lettori con array non standard**: OpenMS, pyteomics e mzR leggono gli spettri con colonne per picco e array di lunghezze diverse senza errore. **Può fallire**: allora le colonne si spostano in un file accanto (D7).
- **E7 Stesse feature**: mzmine (lotto di `tools/bench/corri_mzmine.py`) e SIRIUS su LAB-B, dal nostro mzML e da quello di msconvert. Feature uguali entro 1 ppm e 0,05 min, aree entro 1 %; differenze riportate per numero.
- **E8 Tempi sul Mac** (R-c): nostro, msconvert, RFR (lettura di tutto), TRFP; cache fredda; 5 giri; script e impronte di §4 di `MISURE.md`.
