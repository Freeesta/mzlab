# Valutazione delle idee Gemini

## Fonte, regole e verdetti
- **Fonte**: il documento Gemini nella radice del repository privato (54 KB). Contiene tre risposte di un LLM: architettura, pipeline di codice, kernel SIMD (peak picking e Numpress). Il quarto prompt, il ponte FFI .NET NativeAOT, ha solo la domanda e nessuna risposta. L'ho letto per intero e l'ho trattato come ingresso non fidato.
- **Regole di prodotto applicate** (REGOLE.md del 22/10):
  - solo Rust nel prodotto, nativo e `.wasm`;
  - .NET, C e C++ solo come oracolo locale;
  - `unsafe` in moduli piccoli, con fuzzing e versione sicura;
  - dipendenze solo con un guadagno ≥ 10 % e licenza permissiva;
  - zero perdite, con la prova.
- **Verdetti**:
  - **V** vero;
  - **F** falso;
  - **NP** non provato;
  - **VN** vero ma non per noi;
  - **GIÀ** lo facciamo già.

  Ogni verdetto ha la sua prova: misura (`MISURE.md`), byte del `.raw`, CV PSI-MS 4.2.6 (`crates/mzlab-core/data/psims-cv.json`) o codice del repository.

## 1. Affermazioni (architettura, centroidi, compressione, metadati)
| # | Affermazione | Verdetto | Prova |
|---|---|---|---|
| G1 | msconvert è lento per FFI/COM e lock del produttore | NP / VN | Nessuna misura. Noi non abbiamo FFI: il lettore è Rust puro (opentfraw). |
| G2 | zlib e base64 sono il 55-65 % della CPU | NP per msconvert; da noi **parzialmente V** | Nel nostro scrittore deflate pesa il 54 %, ma il 43,5 % è il `reset` (memset) e solo il 10,6 % è la compressione. base64 pesa ~1 %. |
| G3 | indexList e SHA-1 richiedono due passaggi | **GIÀ** (da noi un passaggio) | `mzmlwrite/indice.rs`: offset e SHA-1 calcolati mentre si scrive. |
| G4 | Una pipeline con lettore, worker e scrittore ordinato satura l'NVMe a 5000-10 000 spettri/s | Architettura V; il numero è basso | Oggi la CLI fa 2770 spettri/s e lo scrittore 9400 spettri/s (LAB-B, 16 524 in 1,76 s). L'obiettivo V1 è ≥ 27 000 spettri/s (`CRITERI.md`). |
| G5 | Usare `RawFileThreadManager` / `CreateThreadAccessor` di RFR | VN | Codice del produttore: escluso dal prodotto. Da noi il parallelismo nasce da un cursore per worker sull'indice (RM-4). |
| G6 | `thermorawfilereader` (.NET 8 in processo + FlatBuffers) fa ~4000 spettri/s | NP, VN | Ospita .NET e la DLL del produttore: escluso. Il nostro decoder Rust decodifica LAB-B in 0,10 s, cioè 165 000 scansioni/s; con i metadati, 44 000/s. |
| G7 | Ponte NativeAOT a copia zero con array fissati | VN, e non c'è codice | Viola la regola 1. Resta solo l'oracolo locale (dump `scandump/1`), che abbiamo già. |
| G8 | Bruker: `timsrust` | Fuori perimetro | Il compito è Thermo; da valutare in un compito Bruker a parte (licenza e numeri). |
| G9 | Nei profili Orbitrap c'è quasi sempre anche il `CentroidStream` del firmware | **V** | Nel corpus tutte le scansioni FT a profilo hanno la lista (sonda). LAB-B: 1 051 091 picchi su 16 524 scansioni. |
| G10 | Il `CentroidStream` va estratto: istantaneo e «insuperabile» | **V**, con la nostra prova | Il file ha solo il profilo ridotto (un pezzo per centroide, 5,8 bin per picco): il firmware ha visto più dati di noi. Lista uguale al bit a msconvert (4 185 930 picchi di LAB-A) e 0 ppm contro RFR. |
| G11 | Il firmware conosce il modello R ∝ 1/(m/z) | **F** per l'Orbitrap | Nell'Orbitrap R ∝ (m/z)^−½ (frequenza ∝ (m/z)^−½); R ∝ 1/(m/z) vale per l'FT-ICR. |
| G12 | Il CWT di msconvert è il collo di bottiglia del picking | F come bersaglio | Per i Thermo msconvert usa il picking del produttore (`peakPicking vendor`: è nel nostro mzML di riferimento di LAB-A). Il CWT si usa solo se si chiede. |
| G13 | La log-parabola a 3 punti è esatta per una gaussiana | V in teoria; NP per l'FT | I picchi FT apodizzati non sono gaussiani e la spaziatura in m/z non è uniforme. Misura: sul timololo di LAB-B la nostra log-parabola differisce dallo strumento di 0,08 ppm (C3 preliminare). |
| G14 | Un rilevatore di massimi SIMD lavora a ~12-15 GB/s | NP | Il rifiuto a blocchi può avvicinarsi alla banda; il costo vero sta nel raffinamento per picco. Si misura in RM-6. |
| G15 | Con `flate2` e il backend zlib-ng: 2,5-4× più veloce e byte identici a zlib | **F** sui byte; **VN** sul backend | zlib-ng emette byte diversi da zlib (altro motore, `deflate_quick` al livello 1). È C, quindi niente `.wasm` puro. L'alternativa Rust è zlib-rs (feature `zlib-rs`, già nel `Cargo.lock`). Misura: 1,5× più veloce di miniz sui centroidi, ma espande i profili al livello 1 (1,039; `MISURE.md` §3). |
| G16 | Numpress Linear + zlib: −60 % degli m/z, errore < 10⁻⁴ ppm | Dimensione NP; **F** sull'errore | L'errore massimo è 0,5/fp. Con fp ≈ 2³¹/m/z_max, a m/z 100 vale ~5·10⁻³ ppm, sopra 10⁻⁴. È comunque con perdita: solo opzione dichiarata (§3). |
| G17 | base64-simd a 15 GB/s | V plausibile; **VN** oggi | base64 è ~1 % del tempo (16 MB su LAB-B); sotto la soglia del 10 %. Da riprovare sul caso denso (RM-0). |
| G18 | Usare itoa e ryu, non `format!`/`write!` | itoa **V**; ryu cambia i byte | Il formato di ryu (`1E-7`, `100.0`) è diverso da `Display`: cambierebbe tutti i file (F10). `write!` in un `Vec` riusato non alloca. Il costo misurato è in `format!` e `String` (5,6 % + malloc ~10 %). |
| G19 | XML fatto con pezzi statici | V | Va in RM-3 (XML senza allocare). |
| G20 | SHA-1 accelerato con la feature `asm` di `sha1` | F per la versione; **GIÀ** | `sha1` 0.11 non ha `asm`: la 0.10 lo aveva, ma in C. La 0.11 sceglie SHA-NI / ARMv8 con `cpufeatures` (nel `Cargo.lock`). |
| G21 | Creare `ZlibEncoder::new` per ogni array alloca ~256 KB: 50 GB di traffico | V nella sostanza | Coerente con la nostra misura: il `reset` di miniz_oxide azzera tabelle di dimensione simile per ogni array (~33 000 array su LAB-B) e vale il 43,5 %. |
| G22 | Rimedio: `Compress` riusato con `.reset()` | **GIÀ**, e non basta | `mzmlwrite/codifica.rs` lo fa già: è proprio il `reset` che costa. Rimedi da misurare in RM-3: zlib-rs a flusso riusato (26 µs contro 40), livello 0/stored per gli array piccoli, livello scelto dalla dimensione dell'array. |
| G23 | Memoria fissa di 50-80 MB anche su 15 GB | NP; obiettivo sensato | La CLI oggi resta ≤ 66 MB su LAB-B. V6 lo chiede costante fino a 8 GB. |

## 2. Metadati e array extra (verificati sul CV PSI-MS 4.2.6)
| # | Proposta | Verdetto | Da noi oggi |
|---|---|---|---|
| M1 | MS:1000927 ion injection time | V | **GIÀ**: 16 524 su 16 524 (LAB-B). |
| M2 | MS:1000800 mass resolving power | V | **Manca**: msconvert lo scrive su ogni scansione (7952 su LAB-A); noi solo come `userParam` `opentfraw.resolution`. Nuovo criterio in F5 (RM-1). |
| M3 | MS:1000512 filter string | V | **GIÀ** (16 524). |
| M4 | MS:1000827/828/829 finestra d'isolamento, MS:1000744 con il valore monoisotopico del trailer | V | **GIÀ**: `monoisotopic_mz()` in `vendor/opentfraw/src/mzml.rs`; 16 038 su 16 038 MS2. Il ripiego quando vale 0 va provato su tutto il corpus (RM-1). |
| M5 | Attivazione: MS:1000422 HCD più i supplementari | Termini V (MS:1002678, MS:1002679, MS:1002680) | **Errore nostro**: su LAB-B 16 038 filtri `@cid` su 16 038 escono come MS:1000422 (HCD); msconvert scrive MS:1000133. È la priorità 1 (RM-1, D10). |
| M6 | Energie «stepped» | NP | Il CV ha un solo MS:1000045 per scansione. Come le scrive msconvert va chiesto alla locale (R-e, file con NCE a passi). |
| M7 | FAIMS MS:1001581 | V | **GIÀ** (`rawlettore/extra.rs`). |
| M8 | 1/K0 MS:1002815 | V, ma non Thermo | Fuori perimetro. |
| M9 | Array del rumore «MS:1002746 / MS:1000517» | **F** su MS:1002746 | MS:1002746 è «Numpress linear + zlib», non il rumore. I termini giusti sono tutti standard: MS:1002742 noise array, MS:1002530 baseline array, MS:1000516 charge array, MS:1002529 resolution array, MS:1000517 signal to noise array, e i nodi campionati MS:1002743/1002744/1002745. Correggo F2 e RM-2: array CV standard, non MS:1000786. |
| M10 | Array FWHM | V come idea | Nessun termine CV per un array FWHM: va come MS:1000786 non standard, solo per i centroidi nostri (dallo strumento abbiamo la risoluzione, da cui FWHM = m/z / R). |
| M11 | Cromatogrammi non MS (pressione, temperatura, UV/PDA) | V | **GIÀ** in parte: `LoadingPump_Pressure` come MS:1003019 (lettori analogici, `analog_records`). Temperatura (MS:1002715) e UV/PDA da controllare sul corpus (RM-1). |

## 3. Numpress e perdite
- Numpress Linear (MS:1002312, o MS:1002746 con zlib) e SLOF sono **con perdita**: non entrano mai di base. Esistono già come opzione dichiarata (`--numpress`, `numpress.rs` con codifica e decodifica uguali al riferimento, `conv/1`).
- Sui file come LAB-B la dimensione la fa l'XML: ~104 MB su 113 sono metadati, gli array sono ~12 MB prima di base64. Numpress cambia poco: il «42-48 % della dimensione» di Gemini è **NP** e su LAB-B è improbabile. Il guadagno serve ai profili densi, dove si misura (RM-9).

## 4. Il codice proposto
| # | Punto | Verdetto | Prova |
|---|---|---|---|
| K1 | `from_raw_parts_mut` su capacità non inizializzata (`b64_buf`) | **F**: comportamento indefinito | Un `&mut [u8]` su memoria non inizializzata è UB. Inoltre `extend_from_slice` copia di nuovo, quindi non è a copia zero. Da usare: un buffer ridimensionato o un'API che scrive in coda. |
| K2 | `compress_vec` con capacità riservata | Rischio di determinismo | `codifica.rs` documenta che, con miniz, lo stesso array può dare byte diversi se il buffer d'uscita ha più o meno spazio. Per questo da noi c'è un blocco di dimensione fissa. |
| K3 | `.expect` sulla compressione | **F** per il nostro contratto | Mai panico: errori `err.file.*` o `io`. |
| K4 | Riciclo dei buffer, `BTreeMap` di attesa e worker bloccati su `rx_rec.recv()` | **F**: può bloccarsi | Scenario: il worker A prende il lotto grezzo k e viene sospeso prima di chiedere un buffer. Gli altri consumano i 2N buffer sui lotti k+1…k+2N, che restano nella mappa in attesa di k; A non trova più buffer e lo scrittore aspetta k. È un blocco permanente. Rimedio: prendere il buffer prima del lotto, oppure assegnare i lotti in ordine e limitare la finestra a k+W (RM-4, test con thread rallentati). |
| K5 | `RawScan` con due `Vec` per scansione | **F** rispetto a «zero allocazioni» | Ogni scansione alloca i suoi vettori nel lettore. |
| K6 | Offset dell'indice preso prima dei 6 spazi di `<spectrum` | **F** | L'offset deve puntare a `<` (lo fanno msconvert e i nostri test dell'indice). Un lettore severo legge male. |
| K7 | Intestazione senza `cvList`, `fileDescription`, `softwareList`, `instrumentConfigurationList` e `dataProcessingList`, con `dp_1` non definito | **F** | Lo schema mzML 1.1 rifiuta il file. Il nostro `mzmlwrite` è provato contro lo XSD (`tests_xsd.rs`). |
| K8 | SHA-1 calcolato fino a `<fileChecksum>` incluso | V | Come facciamo noi (`indice.rs`, `tail`). |
| K9 | `fast_ln` «precisione 1e-7» | **F** | La serie di atanh troncata a t⁷ con t ≤ 1/3 ha un errore ~1·10⁻⁵. Sul centroide l'effetto è trascurabile (~10⁻⁵ bin), ma la dichiarazione è falsa. Lato buono: usa solo operazioni IEEE di base, quindi è deterministica (F9), a differenza di `ln` di libm. |
| K10 | Altezza all'apice con `y0·(1 − ¼(ln₋₁ − ln₊₁)·δ)` | Approssimazione | È il primo ordine di `exp`: errore fino a ~2 % quando δ ≈ 0,5, oltre la soglia dell'1 % di C1. Va usata l'esponenziale del vertice. |
| K11 | δ limitato a ±0,5 e FWHM che mescola mezza altezza e curvatura | Rischio | Nasconde i fit cattivi; la colonna FWHM diventa incoerente. Da misurare in RM-6 prima di adottarli. |
| K12 | Soglia di rumore unica per spettro | **F** per l'Orbitrap | Il rumore varia con m/z. Il file porta ~40 nodi di rumore per scansione (triplette), da interpolare. |
| K13 | Fit in m/z con passo «medio» `dx` | Migliorabile | Il `.raw` FT dà i bin in frequenza, a passo uniforme: un fit nel dominio della frequenza, poi convertito con la stessa formula m/z, toglie l'errore di non uniformità. Proposta per RM-6. |
| K14 | Numpress SWAR: header e ordine dei nibble | V | Uguale a `put_int` del riferimento (`numpress.rs`) per lo zero, i positivi, i negativi e il nibble alto 1-e. |
| K15 | Numpress SWAR: scarico di un solo blocco di 32 bit per chiamata (`if`, non `while`) | **F**: corrompe | Con 8 nibble rimasti nel buffer e 9 nuovi, lo spostamento arriva a 68 bit e i 4 bit alti si perdono. |
| K16 | Numpress: nibble dispari finale con 0 basso | F rispetto al riferimento | Il riferimento mette 0x8 (`numpress.rs`, commento sul riempimento). I lettori accettano anche 0, ma i byte non sono uguali a pwiz. |
| K17 | Numpress: punto fisso diverso dal riferimento | NP | Resta decodificabile (fp è nei byte), ma i byte non sono uguali a pwiz. |
| K18 | Tabella dei tempi contro msconvert (45-90 s di picking, 18 s di zlib…) | NP | Nessuna misura né macchina. Le misure contro msconvert e TRFP sono R-c ed E8, sul Mac a cache fredda con 5 giri. |

## 5. Dove siamo pari, indietro o davanti, e cosa è nuovo
- **Pari o meglio**:
  - lettore Rust senza FFI, 40 volte il numero di Gemini;
  - lista dello strumento al bit;
  - indice e SHA-1 in un passaggio;
  - compressore riusato;
  - SHA-1 hardware;
  - FAIMS, monoisotopico, iniezione e pressione;
  - Numpress uguale al riferimento come opzione;
  - XSD provato;
  - memoria ≤ 66 MB.
- **Indietro**:
  - attivazione CID scritta come HCD (16 038 su 16 038 su LAB-B);
  - MS:1000800 assente;
  - colonne per picco scartate;
  - XML con `format!` e allocazioni;
  - `reset` per array;
  - produttore seriale;
  - più thread che rallentano;
  - CLI 2× il banco.
- **Davvero nuovo e utile**:
  - array CV standard per rumore, linea di base, carica, risoluzione e nodi del rumore (M9);
  - MS:1000800 (M2);
  - scelta del livello di compressione per dimensione dell'array (G22);
  - log-parabola senza libm, deterministica (K9), fatta nel dominio della frequenza (K13), e rumore dai nodi (K12);
  - test del blocco nello scrittore ordinato (K4);
  - controllo dei cromatogrammi di temperatura e UV (M11).

## 6. Cosa entra nei prompt, in ordine di priorità
1. **RM-1**: attivazione (M5), MS:1000800 (M2), ripiego del monoisotopico (M4), cromatogrammi non MS (M11).
2. **RM-3**: livello per dimensione dell'array e zlib-rs a flusso riusato (G15, G22); XML con pezzi statici e itoa, float con il formato di oggi (G18, G19); base64-simd solo se il denso mostra ≥ 10 % (G17).
3. **RM-2**: array CV standard (M9) e FWHM non standard solo per i centroidi nostri (M10).
4. **RM-4**: scrittore ordinato con finestra limitata e test del blocco (K4), offset su `<` (K6), nessun `expect` (K3).
5. **RM-6**: le varianti K9-K13 misurate contro C1-C4. Il kernel SIMD (G14) entra solo dopo che lo scalare passa C1, con la versione scalare uguale al bit.
6. **RM-9** (nuovo, ultimo): Numpress veloce come opzione con perdita, byte uguali al riferimento (K14-K17), solo con un guadagno ≥ 10 % sul denso.
