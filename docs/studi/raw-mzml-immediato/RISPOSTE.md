# Thermo `.raw` → mzML immediato: risposte alle quattro domande

Studio scritto prima di ogni codice. Numeri misurati: `MISURE.md` (con i comandi per ripeterli); criteri con le soglie: `CRITERI.md`; dubbi: `DUBBI.md`; ordine dei lavori e prompt: `LEGGIMI.md`. File: LAB-A (Q Exactive, 7952 scansioni FT solo centroidi, 86,8 MB), LAB-B (Orbitrap Fusion, 16 524 scansioni FT con profilo e centroidi, 82,3 MB), BAF (PRIDE PXD070375, 595 MB), dump RFR della chat locale. Solo Rust nel prodotto, nativo e `.wasm`; RFR e msconvert solo come oracoli locali.

## 0. Cosa ho controllato di `RAW_VELOCE.md` e `PROMPT_LETTORE_THERMO/`
Ho ricontrollato tutto nel codice o con una misura. Quattro affermazioni sono superate:
- **Livello di zlib**: lo scrittore usa il livello 1 (`LIVELLO_ZLIB=1` in `mzmlwrite`), non il 6.
- **RV-02**: è chiuso. In `resolve_scan_arrays` la lista dei centroidi FT esce come Centroid.
- **Letture**: sono a blocchi (`con_blocco`), circa 3 per scansione, non 1060.
- **Apertura del corpus** (R-03 della locale): su 238 `.raw`, RFR ne apre 228 e noi 223. Sette file che RFR apre non si aprono da noi: è il tema di T-5.

Valgono ancora:
- i criteri A1-A7 e B1-B5 di T-7;
- la decisione di T-7: la centroidatura SG è spenta di base e la sua perdita è dichiarata in `spectra.centroid`;
- le regole del Coordinatore sui due bersagli, su `unsafe` e sulle dipendenze (T-7.md, «Regole di velocità»).

Le misure contraddicono un'ipotesi del primo prompt: «la compressione è il collo di bottiglia, quindi va abbassato il livello». Il costo vero è nell'azzeramento del compressore per ogni array (§3.1).

## 1. Significati

### 1.1 «Centroidi corretti»: rispetto a quale verità
Distinguo due casi, perché la verità non è la stessa.
- **Scansioni FT (Orbitrap) con la lista dello strumento.** Il pacchetto contiene già i centroidi. Il firmware li calcola dal profilo completo, che il file non conserva: il file ha solo un profilo ridotto, cioè un pezzo per centroide, ~5,8 bin per picco su LAB-B. Il firmware usa anche il suo modello del rumore. Nessuna centroidatura fatta da noi sul profilo ridotto può avere più informazione di così.
  - Verità operativa: la lista dello strumento, letta bit per bit (m/z f64, intensità f32). RFR e msconvert danno la stessa lista.
  - Già misurato: identità al bit su 4 185 930 centroidi di LAB-A contro msconvert e a 0 ppm contro RFR su INF-A e LAB-B.
  - «Corretto» vuol dire: uguale al bit e completo, cioè con carica, flag, risoluzione, rumore e linea di base per picco (oggi le scartiamo).
- **Scansioni che hanno solo il profilo** (trappola ionica, alcune Fusion, altri produttori in futuro). Qui i centroidi li calcoliamo noi, quindi la verità deve essere indipendente dal nostro software. Ne uso tre, in ordine di forza:
  1. **Profili simulati** con picchi noti per costruzione (posizione, area, larghezza, forma di riga FT presa da LAB-B), rumore e coppie vicine.
  2. **Masse teoriche** di composti noti: calibranti e timololo, con le masse atomiche AME2020 e le abbondanze isotopiche IUPAC. Danno l'errore di massa e i rapporti isotopici.
  3. **La lista dello strumento**, usata come confronto incrociato (non come verità) nelle scansioni FT che hanno entrambi: si centroida il profilo e si confronta con la lista.

Le grandezze e le soglie stanno in `CRITERI.md` §C. Ecco perché ho scelto quei valori.
- **Errore sulla posizione ≤ 0,2 ppm**, cioè un decimo dell'errore tipico dell'Orbitrap (1-3 ppm con calibrazione esterna). Così la centroidatura non sposta mai l'assegnazione di una formula.
- **Area entro 1 %**: è la precisione che serve ai rapporti isotopici (³⁴S al 4,5 %, ¹³C al 14 % per il timololo) per distinguere formule.
- **Risoluzione di due picchi a 1 FWHM**: è il caso degli isotopologhi fini a risoluzione 120k.
- **Richiamo sopra 10× il rumore**: un picco perso è un TP perso.
- **Fantasmi**: un falso picco è un falso candidato.

Prova già fatta, ma da confermare con R2 e su più dati: sul timololo di LAB-B (11 MS1), la nostra log-parabola sul profilo e i centroidi dello strumento danno errori quasi uguali.

| | Errore mediano | ³⁴S | ¹³C |
|---|---|---|---|
| Log-parabola mzLab | −1,44 ppm | 4,20 % | 14,46 % |
| Centroidi dello strumento | −1,36 ppm | 4,20 % | 14,47 % |
| Teorico | – | 4,474 % | 14,06 % |

L'errore di −1,4 ppm è dello strumento, non della centroidatura.

### 1.2 «Senza perdere informazione»
**Deve poter essere ricostruito dall'mzML, sempre**:
- ogni scansione, nell'ordine del file;
- ogni punto di profilo presente nel file, con m/z f64 uguale al bit a quello di RFR e intensità f32 esatta;
- i bin di frequenza originali: con i coefficienti e il fudge scritti per scansione si devono poter ricalcolare per il 100 % dei punti (`CRITERI.md` F4);
- ogni centroide della lista, con tutte le sue colonne per picco;
- i nodi del rumore e della linea di base;
- il filtro, tutti i campi del trailer, lo stato e la sintonia più vicini, il metodo, la provenienza (SHA-1 del `.raw`);
- l'attivazione, il precursore, la finestra d'isolamento, la polarità, l'analizzatore.

**Perdita accettabile solo se dichiarata** (nel file e nel registro `conv/1`):
- gli zeri di bordo aggiunti come fa RFR. Non sono nel file, quindi vanno marcati come aggiunti e devono essere ricostruibili come tali;
- l'allargamento da f32 a f64 dell'm/z (è esatto);
- la conversione da frequenza a m/z: è una funzione deterministica, invertibile secondo F4;
- l'opzione `--numpress` (con perdita, mai di base);
- `--peaks mzlab`, cioè centroidi nostri al posto della lista, che è una perdita del profilo;
- il passo `spectra.centroid` di Flow, che è fuori da questo lavoro.

**Mai**:
- cambiare un valore numerico in silenzio: arrotondare, togliere punti bassi, riordinare, togliere duplicati;
- togliere scansioni o colonne senza dirlo;
- inventare un valore che il file non ha (un campo mancante resta assente);
- scrivere un mzML che dipende dal numero di thread o dalla macchina.

Oggi abbiamo tre perdite:
- `peaks=file` su LAB-B scrive la lista e butta il profilo;
- le colonne per picco sono scartate (lo stream del descrittore viene saltato);
- l'attivazione `@cid` di LAB-B esce come HCD.

### 1.3 «Il più veloce possibile»: il limite fisico
Il tempo minimo è il massimo fra quattro pavimenti, che non si sommano perché gli stadi si sovrappongono a flusso:

`T_min = max( B_in / BW_lettura , B_out / BW_scrittura , W_cpu / N_core , W_seriale )`

- B_in e B_out sono i byte letti e scritti.
- BW sono le larghezze di banda misurate sulla macchina (dd diretto, cache fredda).
- W_cpu è il lavoro parallelizzabile, misurato stadio per stadio con micro-banchi da soli: decodifica, conversione m/z, XML, base64, zlib.
- W_seriale è ciò che non si divide: l'SHA-1 dell'uscita, che `indexedmzML` chiede su tutto il file in ordine, e la scrittura ordinata.

Stima per LAB-B, container a 4 core:

| Voce | Tempo |
|---|---|
| Lettura (82 MB a 2,0 GB/s) | 0,04 s |
| Scrittura (113 MB a 1,6 GB/s) | 0,07 s |
| Lavoro CPU: decodifica 0,10, extra 0,27, XML ~0,10, zlib dei soli array ~0,12 | ~0,6 s·core, quindi 0,15 s su 4 core |
| SHA-1 dell'uscita (~1 GB/s) | 0,11 s, sovrapponibile |
| **Pavimento** | **≈ 0,15-0,2 s** |

| Oggi su LAB-B | Tempo | Volte il pavimento |
|---|---|---|
| Scrittura dell'mzML (`raw_tempi`) | 2,87 s | 15-20× |
| `mzlab-cli convert`, indicizzato | 5,97 s | 30-40× |

Per LAB-A il pavimento è ≈ 0,15 s, contro 1,8 s (`raw_tempi`) e 3,96 s (CLI).

**Caso denso** (≥ 2 GB, 200 000 punti per scansione a profilo).
- Senza compressione è legato al disco: ~5 GB di base64 in uscita, ~3 s a 1,6 GB/s.
- Con zlib è legato alla CPU: 4 GB di array a ~140 MB/s·core, quindi ~7 s su 4 core. zlib guadagna solo il 20 % sui profili densi (rapporto 0,80).
- La scelta della compressione predefinita si decide con la misura (`CRITERI.md` V4), non a priori.

«Veloce» ha anche una seconda misura: il **tempo al primo spettro scritto** (≤ 50 ms dopo l'apertura). L'mzML a flusso si può leggere mentre è scritto, ma l'indice e l'SHA-1 arrivano solo alla fine.

## 2. Da dove parto

### 2.1 Il materiale più grezzo e cosa c'è byte per byte
Il `.raw` non contiene il transiente. Il materiale più grezzo è il pacchetto di ogni scansione, ed è già il prodotto del firmware (FFT, apodizzazione, calibrazione, riduzione). Sezioni del file: intestazione Finnigan, `RawFileInfo`, sequenza, metodo OLE2, `RunHeader`, indice delle scansioni, eventi (filtri), trailer extra, status log, tune, error log, pacchetti dei dati.

Il pacchetto FT (layout misurato con una sonda su LAB-A e LAB-B: 0 byte di scarto fra pacchetti consecutivi, dimensione uguale a `data_size`):

| Parte | Contenuto | Chi l'ha elaborato |
|---|---|---|
| Intestazione, 40 B | dimensioni delle parti, layout, low/high m/z f32 | firmware |
| Profilo | nel dominio delle frequenze: first_value, step, pezzi (first_bin, nbins, fudge f32 se layout&0xFF, segnale f32); su LAB-B un pezzo per centroide, 5,8 bin per picco, cioè profilo ridotto | firmware: FFT, apodizzazione, riduzione |
| Lista dei picchi | count, m/z f32 o f64 (layout&0x10000), intensità f32 | firmware: centroidatura e calibrazione |
| Descrittore | un u32 per picco: indice nei 16 bit bassi (0 eccezioni), bit alti probabilmente carica (multipli di 0x100, fino a 0x1700) e flag (0x10, 0x20, 0x40, 0x80) | firmware, da confermare con RFR (D1) |
| «Unknown» | 1 + risoluzione f32 per picco (n+1 in tutte le scansioni di LAB-B) | firmware |
| Triplette | ~40 nodi (m/z, rumore, linea di base) | firmware: modello del rumore |

Formula verificata al bit contro msconvert: m/z = B/f² + C/f⁴ + fudge (nparam 7; 4 = LTQ-FT con A + B/f + C/f²). Usa solo operazioni IEEE di base, quindi è deterministica su ogni macchina se nessuno la contrae in FMA.

Gli zeri: RFR e msconvert riempiono il profilo con zeri sulla griglia dei pezzi; noi diamo solo i punti presenti. Su 4718 scansioni di LAB-B, msconvert aggiunge 2 159 784 zeri (57 % dei suoi punti); il file stesso ne contiene 12 003. Gli zeri hanno gli stessi residui di fudge dei punti veri, cioè sono bin adiacenti (corse tipiche di 8 tra i pezzi e di 4 ai bordi). La regola esatta resta da derivare (D2).

### 2.2 Dati con una verità indipendente dal nostro software
1. **File costruiti nei test**: pacchetti scritti dal test con valori noti, già usati in opentfraw.
2. **Profili simulati** (RM-6), con la forma di riga presa dal profilo reale di LAB-B.
3. **Composti noti in file veri**: il timololo in LAB-B (laboratorio); i calibranti (caffeina 195,08765, MRFA 524,26496, serie Ultramark 1621), da chiedere alla locale (`RICHIESTE_LOCALE.md` R-a); standard aggiunti in dataset PRIDE pubblici.
4. **Oracoli del produttore, indipendenti da noi ma non dalla Thermo**: RFR (dump `scandump/1` della locale) e msconvert con `peakPicking vendor`. Sono la verità per la lettura, non per la centroidatura.
5. **Lettori mzML di terzi** per la validità: PSI `mzML validator`, OpenMS `FileInfo`, pyteomics, mzR, msconvert che rilegge.

## 3. Come lavoro

### 3.1 Trovare i colli di bottiglia senza fidarsi dell'intuizione
- Uso tempi per stadio con il banco: produttore da solo, scrittore da solo, livelli 0/1/6, 1 e 3 thread.
- Conto le istruzioni (callgrind, deterministico) e le confronto con il tetto di ogni stadio misurato da solo (roofline).
- Faccio ablazioni (spegni uno stadio e misura la differenza) e una curva di Amdahl con 1…N thread.

Cosa ha già trovato:
- **L'azzeramento del compressore domina.** Il 54 % del tempo di scrittura è deflate, ma il 43,5 % è il `reset` di miniz_oxide per ogni array (memset delle tabelle) e solo il 10,6 % è compressione vera.
- **Più thread rallentano.** Con 3 thread la scrittura è più lenta che con 1: LAB-A 2,09 s contro 1,48 s; LAB-B 2,74 s contro 1,76 s. La spiegazione probabile: il produttore è seriale (0,375 s) e i memset di 16 524 × 2 array saturano la banda della memoria (D8).
- **L'intestazione costa il 14,75 %.** È `testata_mzml`: SHA-1 del `.raw` 7,25 %, metodo 7,4 %, entrambi sul percorso critico.
- **L'XML pesa più dei dati.** `spectrum_head` 11,3 %, `esc` 6,1 %, `format!` 5,6 %, malloc/free ~10 %.
- **La CLI raddoppia il tempo del banco** (5,97 s contro 2,87 s), con zlib quasi ininfluente (5,74 s a livello 0). Il costo sta fuori dallo scrittore e va scomposto (D11).

### 3.2 Alternative per pezzo
| Pezzo | Alternative | Costo / rischio | Misura che decide |
|---|---|---|---|
| Lettura | `read` a blocchi (oggi); `mmap` nativo (`memmap2`, già nel lock); OPFS `createSyncAccessHandle` a offset nel browser | `mmap`: rischio SIGBUS se il file cambia, quindi solo in un modulo `unsafe` isolato | MB/s a cache fredda contro dd |
| Decodifica | seriale (oggi 0,10 s per LAB-B); per scansione in parallelo con un cursore proprio | basso, l'indice dà gli offset | Amdahl 1…N |
| Metadati per scansione (`extra_scansione`) | oggi 0,27 s seriali, il 72 % del produttore; in parallelo nel worker; tabelle dei trailer pre-decodificate una volta | basso | ablazione |
| Centroidi | copiare la lista (costo ~0) / nostra solo per le scansioni solo profilo | la nostra porta un rischio di perdita, quindi è solo dove serve | `CRITERI.md` §C |
| XML | `format!` + `String` (oggi); scrittura diretta in un buffer riusato con interi e float formattati senza allocare (`itoa` è nel `Cargo.lock`, `ryu` no: un float si formatta con lo stesso codice di oggi) | nessun rischio sui byte se il formato dei numeri resta identico | allocazioni per scansione = 0 |
| Base64 | attuale ~1,5-1,8 GB/s; SIMD | guadagno piccolo | roofline |
| Compressione | miniz L1 con reset (oggi); zlib-rs con flusso riusato (L1 26 µs contro 40; L0 3,4 µs); livello 0 stored; nessuna compressione; fdeflate (espande i float: scartato) | zlib-rs (Zlib) è già nel `Cargo.lock` come backend di flate2 (feature `zlib-rs`): nessuna dipendenza nuova, ma va misurato il `.wasm` (+10 % dichiarato); stored e nessuna devono essere letti da tutti i lettori (D7) | µs per array e dimensione sul corpus |
| SHA-1 | in serie (oggi, sul percorso critico); in un thread dedicato che consuma i byte in ordine; SHA-1 del `.raw` calcolato mentre si legge | basso | tempo con e senza |
| Scrittura | `BufWriter` seriale; scrittore ordinato con gli offset dell'indice calcolati nella stessa passata e back-pressure in byte | basso | memoria di picco costante |
| Parallelismo | pool sull'encoder (oggi, finestra 4×thread spettri); pipeline lettore→N worker (decodifica+metadati+XML+zlib)→scrittore ordinato; rayon | ordine e determinismo da provare | F9, V5 |

### 3.3 Casi limite
**In ingresso**:
- scansioni senza profilo, senza lista o vuote (0 picchi);
- m/z f64, assenza del fudge, nparam 0/4/5/7;
- analizzatori misti nello stesso file (IT+FT nel Fusion);
- cambio di polarità, FAIMS (CV per scansione), SIM/tSIM, msx/DIA, MS3 SPS, ETD/EThcD/UVPD, lock mass;
- Exploris e Astral (D5);
- file ancora in acquisizione o troncati;
- indice che punta fuori dal file, numeri enormi (passano da `Limiti`);
- versioni del formato 57-66.

**In uscita**:
- offset dell'indice oltre 4 GB (u64);
- `defaultArrayLength` = 0;
- array di lunghezze diverse nello stesso spettro (`arrayLength`);
- escape XML di filtri e trailer: caratteri di controllo, non ASCII, `&`, `<`;
- NaN e Inf nei valori;
- id `controllerType=0 controllerNumber=1 scan=N` unici;
- riferimenti al precursore;
- SHA-1 calcolato fino a `<fileChecksum>` incluso;
- formattazione dei numeri identica su ogni macchina.

### 3.4 Cosa resta uguale cambiando thread, macchina e ordine
Gli **stessi byte** (SHA-256 dell'uscita) devono restare con:
- 1, 2, 4 e N thread;
- x86-64 Linux, arm64 macOS e `.wasm` nel browser;
- l'ordine degli ingressi permutato;
- due esecuzioni di seguito.

Non deve entrare nei byte niente che dipende dall'orologio, se non la data letta dal file. Mezzi:
- una formula m/z senza FMA né funzioni libm;
- un formato dei numeri senza dipendere dalla locale;
- un ordine di scrittura uguale a quello del file, qualunque sia l'ordine di completamento dei worker;
- una compressione deterministica per livello e libreria (cambiare libreria cambia i byte: va dichiarato e fissato nella versione `conv/1`).

## 4. Prove esterne a noi
Ogni prova dice cosa si confronta, con quale tolleranza e se può fallire. L'elenco completo con i numeri è in `CRITERI.md` §E.
- RFR (dump `scandump/1`) per la lettura: profilo, lista, colonne per picco, al bit.
- msconvert con `peakPicking vendor` per i centroidi e l'mzML di riferimento: spettri, punti, zeri, metadati.
- I calibranti per l'errore di massa e i rapporti isotopici.
- Cinque lettori di terzi per la validità e gli offset.
- mzmine e SIRIUS per «stesse feature»: dal nostro mzML e da quello di msconvert si devono ottenere le stesse feature entro le tolleranze dichiarate.
- Tempi sul Mac a cache fredda: 5 ripetizioni, mediana, script e impronte pubblicati (`MISURE.md` §4), così un terzo li ripete.

**Prove che possono fallire**, dichiarate ora:
- F2: colonne per picco uguali a RFR (dipende da D1);
- F3 con gli zeri;
- C4: la nostra centroidatura IT contro quella del produttore, che usa un algoritmo ignoto;
- V5: scalabilità ≥ 0,7 N, se il disco o l'SHA-1 seriale saturano prima;
- V8: browser ≤ 1,7× il nativo;
- E6: un lettore di terzi che rifiuta array non standard o di lunghezze diverse (D7).

Tutti i numeri vanno riportati, anche quelli sfavorevoli.
