# Studio «Rosina»: leggere i file di tutti gli spettrometri di massa (lettori nativi e convertitore mzML)

Obiettivo (Federico, 19/10): mzLab legge, su qualunque computer, i file di ogni produttore, sia in formato nativo sia come convertitore in mzML (profilo e centroidi), e ne legge PERFETTAMENTE i metadati. Questo studio parte dall'inventario e dalla matrice di `docs/studi/FORMATI_NATIVI_MS.md` (non rifatti) e dal corpus già raccolto nel repository privato (`dati_formati/`, 191 file, 1,3 GB). Base: `main` a `69b8c16`.

Documenti dello studio:
- questo file: i quattro punti di metodo, A (censimento), D (politica dei dati), E (piano di chat), F (protocollo di prova), decisioni;
- `docs/studi/rosina/FONTI.md`: B, catalogo delle fonti per formato ed esperimento;
- `docs/studi/rosina/SCHEDE.md`: C, una scheda per formato con i metadati campo per campo;
- `docs/studi/PROMPT_ROSINA/`: i prompt delle chat DEV e della chat locale.

## 0. Da dove si parte (stato misurato)

- Lettori: motore (mzML, `.wiff` 3200, `.D` GC/MS, lettore esterno Thermo) e otto moduli (`aperti`, `andi`, `thermo`, `agilent`, `sciex`, `waters`, `shimadzu`, `bruker`). Ogni modulo è a circa il 3 % dal suo tetto (`docs/agenti/lettori-modulo.md`: es. `sciex` 319 930 su 327 680 B, `waters` 293 150 su 303 104 B): ogni lettore nuovo o parte nuova pesante va in un modulo NUOVO, non dentro uno esistente.
- Corpus privato `mzlab-dati/dati_formati/` (indice `INDICE.md`, record `_records_*.tsv`, candidati grandi `_grandi_*.tsv`, lacune `_nontrovato_*.md`): Thermo 26 `.raw` (TSQ Vantage/Quantum/Quantiva/Altis, Q Exactive, PDA), SCIEX 34 cartelle (API 4000 → ZenoTOF 7600, `.wiff2`), Agilent 15 `.D`, Waters 11 cartelle, Shimadzu 10, Bruker 12, mzXML 3, numpress 3, netCDF 5, scansioni speciali 7, fixture dei lettori liberi (ProteoWizard 198 file, ThermoRawFileParser, rawrr, timsrust).
- **La lacuna vera non sono i file ma la verità.** Quasi tutti i file sono «senza mzML»: il cloud non può far girare msconvert (niente Windows, Docker non parte). Le prove oggi sono interne (somma delle intensità = TIC dell'indice) o contro msconvert su pochi file: Waters TOF, Shimadzu, mzXML, TSQ, PDA Thermo, TripleTOF sono «non verificati». Rosina è prima di tutto un progetto di **oracoli e verità**, poi di lettori.
- Rete del cloud (provata il 19/10): PRIDE, MetaboLights (API e FTP via HTTPS), Zenodo, Metabolomics Workbench (lento) raggiungibili; MassIVE no (connessione chiusa). Nessun file di dati scaricato dal cloud in questo studio (solo metadati dei depositi, 354 MB in tutto).

## 1. Cosa farebbe un professionista di altissimo livello (e qui non è chiesto)

1. **Oracolo del produttore, non solo msconvert.** msconvert usa le DLL dei produttori (Windows), ma con le sue opzioni e versioni; ThermoRawFileParser usa RawFileReader di Thermo (gira su Linux e Mac con .NET); l'SDK `timsdata` di Bruker gira su Linux e calcola 1/K0; il MassLynx Raw SDK di Waters ha un lettore Linux gratuito ([Waters WKB92255](https://support.waters.com/KB_Inf/MassLynx/WKB92255_Does_the_MassLynx_SDK_support_a_Linux_compatible_Raw_reader_for_example_by_using_NET_core_20)). 
   Ogni oracolo gira SOLO sul Mac o su un PC Windows, mai nel sito; il suo risultato si congela in un file di verità.
2. **Verità congelate e datate**: un JSON `verita/1` per file (conteggi, ogni scansione, campioni di punti, cromatogrammi, metadati) con nome e versione dell'oracolo e le opzioni usate. I test confrontano il lettore con la verità senza rifare la conversione; una verità nuova da un oracolo nuovo non sostituisce la vecchia, si aggiunge (due oracoli che non concordano sono un'informazione).
3. **Triangolazione**: per ogni file almeno due fonti indipendenti tra lettore mzLab, oracolo del produttore, mzML pubblicato dagli autori, lettore libero (opentfraw/openwraw/timsrust/pyteomics). Se mzLab concorda con una sola, il campo è «da chiarire», non «verificato».
4. **Catalogo dei casi limite** come matrice caso × formato, ogni cella con il file che la prova o «nessun file» (sotto, §F.4): taratura per scansione e lock mass, profilo contro centroidi del produttore, cambio di polarità, eventi di scansione, mobilità, unità di tempo e d'intensità, versioni del formato, file oltre 4 GB, più campioni per contenitore, file troncati o in acquisizione, endianness.
5. **Priorità dai dati, non dalle opinioni** (§A): quote misurate su PRIDE, MetaboLights e Metabolomics Workbench, corrette per la distorsione dei depositi (proteomica = Orbitrap; i laboratori di routine e ambientali depositano poco: QqQ e GC-MS sono sottostimati).
6. **Metadati campo per campo** (priorità di Federico): per ogni formato una tabella «campo del produttore → dove sta nel file → percorso in `metodo/1` → come lo mostra il software del produttore → oracolo automatico» (`SCHEDE.md`). Un campo che l'oracolo vede e mzLab no è un difetto (regola «nessuna perdita», `docs/agenti/grezzi-non-estratti.md`).
7. **Questioni legali scritte prima del codice** (§1.1): metodo di lettura (osservazione dei file, mai decompilazione), licenze degli SDK usati come oracoli, licenze dei file, dati personali nei file clinici.
8. **Conformità del convertitore** a mzML 1.1 (schema XSD `mzML1.1.0_idx.xsd`) e semantica PSI (regole di mappatura CV: validatore di OpenMS `FileInfo -v`), versione dichiarata di `psi-ms.obo`, `indexedmzML` con offset e SHA-1 giusti, centroidi di mzLab dichiarati come tali (`dataProcessing`), mai spacciati per quelli del produttore.
9. **Robustezza misurata**: `fuzz.yml` copre oggi `ole`, `raw`, `wiff`, `mzmlhead`, `idxread`, `xmltree_methodinfo`, `mzlib_msp_mgf`; nessun bersaglio per i moduli di `mzlab-lettori` (netCDF, mzXML, SQLite/zstd di Bruker, Waters, Shimadzu, Agilent LC/MS). Un bersaglio per famiglia, con semi tratti dai primi KB dei file veri troncati.
10. **Scala**: un file grande per famiglia (Orbitrap 2-5 GB, timsTOF 10+ GB, `.wiff.scan` > 4 GB) aperto in locale con tempo e picco di memoria scritti in `docs/agenti/misure.md`; offset a 64 bit provati su file veri, non solo costruiti.
11. **Versioni del formato**: matrice versione × lettore (Thermo v57-v66, Analyst 1.4-1.7 / SCIEX OS, MassHunter B.0x-12, MassLynx 4.1-4.2/UNIFI, LabSolutions 5.x/Insight, timsControl); una versione mai vista dà un errore con la versione nel `detail`, non un'ipotesi.
12. **Comunità**: chiedere i dati di prova dei lettori di ProteoWizard (le cartelle `vendor_readers/*Test.data` hanno l'mzML di riferimento fatto con le DLL), proporre a HUPO-PSI i termini CV che mancano (es. nativeID dei moduli nuovi), chiedere ai produttori (SCIEX per `.wiff2`, Bruker per BAF) una documentazione o una licenza per un progetto didattico aperto; la rete NORMAN per file ambientali (il nostro dominio: inquinanti).
13. **CI con i dati privati**: un workflow nel repository PRIVATO `mzlab-dati` (che ha i file) fa checkout del codice pubblico e confronta i lettori con le verità ogni notte; il repository pubblico non vede mai i dati.

### 1.1 Questioni legali (raccomandazione)
- **Metodo**: osservare i file e scrivere un lettore interoperabile (come per `.wiff` e `.D`); nell'UE la direttiva 2009/24/CE ammette l'osservazione del funzionamento (art. 5.3) e, a condizioni strette, la decompilazione per l'interoperabilità (art. 6): non ci serve e non la usiamo. Mai decompilare SDK o software del produttore (le licenze, es. RawFileReader di Thermo, lo vietano esplicitamente).
- **SDK come oracoli**: si usano come utenti finali sul Mac (TRFP con RawFileReader, `timsdata`, MassLynx SDK, msconvert con «i agree to the vendor licenses»), solo per produrre verità; nulla dell'SDK entra nel repository pubblico, nel sito o nell'app desktop. Prima di ogni oracolo la chat locale salva la licenza accettata in `mzlab-dati/dati/formati/_licenze/` e controlla che l'uso per confronto sia permesso; un dubbio va a Federico.
- **File**: CC0, CC-BY, MIT, Apache si ridistribuiscono con attribuzione; CC-BY-SA solo nel privato; «EBI terms of use» e file senza licenza (fixture di ProteoWizard, Panorama, `asalt/mzio`) restano privati. Il pubblico riceve solo file sintetici o, dopo il controllo di Federico, piccoli CC0/CC-BY (§D).
- **Dati personali**: i file clinici portano nomi di campione, operatori, computer, percorsi. Restano nel privato; nel pubblico (documenti, test, commit) solo accession e numeri. `identita` di `metodo/1` resta fuori dall'mzML.

## 2. Il materiale più grezzo
- I **file nativi originali** dei depositi (non gli mzML degli autori), con lo SHA-256 del file o del manifest della cartella (già così in `dati_formati/`).
- I **metadati dei depositi** per contare gli strumenti: faccette di PRIDE (`/pride/ws/archive/v3/facet/projects`), dump EB-eye di MetaboLights (`eb_eye_metabolights_studies.xml`, con i nomi dei file di ogni studio e i protocolli MS), analisi di Metabolomics Workbench (`/rest/study/study_id/ST/analysis`, nome e tipo dello strumento); SDRF e ProteomeXchange per i dettagli.
- **Sorgente dei lettori aperti** come mappa delle stranezze: ProteoWizard (`pwiz_aux/msrc/utility/vendor_api`, `pwiz/data/vendor_readers` con i test), ThermoRawFileParser, OpenMS, opentims/timsrust/alphatims, pyteomics/pymzml, rainbow (DAD Agilent), openwraw/opentfraw/openaraw/opensxraw/openszraw (Apache-2.0, già citati nei moduli): letti per capire, mai copiati.
- I **file con foglio di metodo noto**: i `.wiff` e `.dam` del 3200 del laboratorio, con la stampa del metodo e la «File Information» di Analyst presa sul PC dello strumento.

## 3. Metodi e strumenti
| Strumento | Cosa fa nello studio | Dove gira |
|---|---|---|
| msconvert (ProteoWizard, DLL dei produttori) | mzML di riferimento (64 bit, zlib, senza peak picking) e una seconda conversione con `peakPicking vendor` (centroidi del produttore) | Mac con l'immagine Docker `proteowizard/pwiz-skyline-i-agree-to-the-vendor-licenses` (Wine; su Apple Silicon in emulazione x86, da provare) o un PC Windows dell'università |
| ThermoRawFileParser (RawFileReader) | mzML di riferimento Thermo e metadati in JSON (`-m 0`) | Mac (.NET), nativo |
| `timsdata` (SDK Bruker) via opentims/alphatims | 1/K0 per scansione, m/z dei frame, verità della mobilità | Mac (Linux in Docker se la libreria macOS manca) |
| MassLynx Raw SDK (Waters) | m/z con e senza lock mass, tempo di deriva, intestazioni di funzione | Linux (Docker sul Mac); licenza da leggere prima |
| Software del produttore (Analyst, Xcalibur, MassHunter, MassLynx, LabSolutions, DataAnalysis) | metadati «come li vede l'utente» (File Information, Method Report) copiati a mano in un modulo fisso | PC degli strumenti (Analyst del 3200 al laboratorio); altri solo se Federico ne ha accesso |
| OpenMS `FileInfo -v`, `xmllint --schema mzML1.1.0_idx.xsd` | validazione sintattica e semantica dell'mzML scritto da mzLab | Mac e CI (pacchetti Linux) |
| pyteomics, pymzml, OpenMS, mzmine 4 | rilettura dell'mzML di mzLab (prova di andata e ritorno con lettori terzi) | Mac e CI (`tools/`) |
| API PRIDE/MetaboLights/Workbench/MassIVE | censimento (§A) e ricerca dei file | cloud (MassIVE solo dal Mac) |
| SHA-256 e manifest | identità dei file e delle cartelle | ovunque |
| ImHex / Kaitai Struct | specifica binaria leggibile dei formati osservati (`docs/agenti/*_specifica.md` o `.ksy` nel privato) | Mac |
| cargo-fuzz | un bersaglio per famiglia di `mzlab-lettori` | CI (`fuzz.yml`, domenica) |
| confronto spettro per spettro | `verita/1` contro il lettore (test Rust), F.1-F.5 | CI del privato ogni notte e in locale |

## 4. Cosa dimostra, fuori da noi, che un lettore è corretto
Scala delle prove (dalla più forte):
- **V1**: stesso file, oracolo che usa la libreria del produttore (msconvert con DLL, TRFP, `timsdata`, MassLynx SDK): stessi spettri (ppm, intensità relative, numero di scansioni, TIC) e stessi metadati.
- **V2**: mzML pubblicato dagli autori nello stesso deposito (prodotto da loro, versione dichiarata nell'mzML).
- **V3**: software del produttore a schermo (metadati letti a mano da Analyst/MassHunter/…; numeri di un picco).
- **V4**: lettore libero indipendente (opentfraw, openwraw, timsrust, pyteomics…) concorde.
- **V5**: risultato pubblicato sullo stesso dataset (numero di feature o identificazioni, aree di uno standard) riprodotto entro la tolleranza dell'articolo.
- **V6**: file di un laboratorio terzo (non dello stesso deposito né dello stesso strumento).
- **V7**: coerenza interna (TIC = somma delle intensità, indice = record) e corpus di fuzzing senza crash: necessaria, mai sufficiente.
Il convertitore mzML ha in più: validatore PSI superato, rilettura uguale con 3 lettori terzi.

| Formato | V1 possibile? | V2 | V3 | V4 | Prova minima per dire «L» |
|---|---|---|---|---|---|
| Thermo `.raw` | sì: TRFP sul Mac (anche metadati JSON), msconvert | rara (PXD con mzML) | Xcalibur se disponibile | opentfraw (già la base: non indipendente) | V1 su 1 file per famiglia di strumento (TSQ, Q Exactive, Orbitrap Tribrid, trappola) |
| SCIEX `.wiff` | sì: msconvert (Windows/Wine) | sì: PIS/NL di Zenodo 7897576, MTBLS1381 | **sì: Analyst del 3200 al laboratorio** | opensxraw (dichiarato inaffidabile) | V1 o V2 per strumento ed esperimento |
| SCIEX `.wiff2` | sì: msconvert o SCIEX MS Data Converter | rara | SCIEX OS | nessuno | nessun lettore: solo guida |
| Agilent `.D` | sì: msconvert (MHDAC) | sì: ST003656, ST004061, ST004922 (già usati) | MassHunter se disponibile | openaraw, rainbow (DAD) | V1/V2 per QqQ, Q-TOF profilo, centroidi, DAD |
| Waters `.raw` | sì: msconvert; MassLynx SDK Linux per lock mass e deriva | sì: ST004453 (TQ-XS), ST003942, ST003984 | MassLynx se disponibile | openwraw | V1 per TOF, MSe, DDA, lock mass |
| Bruker TDF/TSF | sì: `timsdata` (1/K0), msconvert | fixture di ProteoWizard | DataAnalysis se disponibile | timsrust, alphatims | V1 per 1/K0 e compressione 2 |
| Bruker BAF | sì: msconvert, `baf2sql` | rara | DataAnalysis | nessuno | nessun lettore: solo guida |
| Shimadzu `.lcd`/`.qgd` | sì: msconvert (DLL Shimadzu, solo LC) | nessuno trovato | LabSolutions se disponibile | openszraw (immaturo) | V1 per LC; per `.qgd` V3 o `.cdf` esportato dallo stesso file |
| netCDF/ANDI | no (formato aperto) | `.D` della stessa corsa (già fatto) | software che l'ha esportato | netCDF4-python, xcms | V2/V4 + specifica ASTM |
| mzML altrui, mzXML, mzData | no (formati aperti) | conversione dello stesso nativo | — | pyteomics, OpenMS | V4 con due lettori terzi |

## A. Censimento: produttori e quote d'uso misurate (19/10/2026)

Script in questo studio: lanciati dal cloud il 19/10/2026; numeri esatti dalle fonti, attribuzione al produttore con regole sui nomi (approssimazione di qualche punto percentuale, scritta sotto).

**A.1 PRIDE** (proteomica). Query: `GET https://www.ebi.ac.uk/pride/ws/archive/v3/facet/projects?facetPageSize=200`, faccetta `instruments` (prime 100 voci, 45 724 coppie progetto × strumento su 42 168 progetti; `.../v3/projects/count` = 42 168).
| Classe | Coppie | Quota |
|---|---|---|
| Thermo Orbitrap/FT (Q Exactive, Fusion, Exploris, Astral, Velos…) | 37 222 | 81,4 % |
| Bruker timsTOF | 2 517 | 5,5 % |
| SCIEX TOF (TripleTOF, ZenoTOF, QSTAR, TOF/TOF) | 2 186 | 4,8 % |
| Waters (Synapt, Xevo, Q-Tof) | 1 118 | 2,4 % |
| Bruker altri (maXis, impact, flex, solariX: BAF/MALDI/FT-ICR) | 1 003 | 2,2 % |
| Thermo trappola e QqQ (LTQ, LCQ, TSQ) | 682 | 1,5 % |
| Agilent | 222 | 0,5 % |
| SCIEX QTRAP/QqQ | 107 | 0,2 % |
| non attribuiti («instrument model» generico ecc.) | 667 | 1,5 % |

**A.2 MetaboLights** (metabolomica). Query: `GET https://ftp.ebi.ac.uk/pub/databases/metabolights/eb-eye/eb_eye_metabolights_studies.xml` (350 MB, modificato il 19/10/2026 07:42 GMT): 3 546 studi pubblici, campi `dataset_file` (nomi dei file) e `mass_spectrometry_protocol`/`instrument_platform`.
Formati dei file depositati (studi con almeno un file di quel tipo, su 3 166 con file MS riconoscibili):
| Formato | Studi | Quota | File |
|---|---|---|---|
| `.raw` (Thermo; o Waters se cartella non elencata) | 1 334 | 42,1 % | 252 421 |
| mzML | 859 | 27,1 % | 205 685 |
| mzXML | 377 | 11,9 % | 62 317 |
| SCIEX `.wiff` | 322 | 10,2 % | 65 620 |
| `.d`/`.D` (Agilent o Bruker) | 322 | 10,2 % | 77 508 |
| Waters `.raw` (cartella riconosciuta) | 244 | 7,7 % | 97 123 |
| netCDF/ANDI | 210 | 6,6 % | 53 026 |
| mzData | 39 | 1,2 % | 6 017 |
| MGF | 34 | 1,1 % | 1 265 |
| Shimadzu `.lcd`/`.qgd` | 34 | 1,1 % | 5 418 |
| imzML | 24 | 0,8 % | 1 337 |
| LECO `.peg`/`.smp` | 12 | 0,4 % | 1 525 |
Produttore dal testo dei protocolli (3 403 studi MS; uno studio può contarne più): Thermo 1 585 (46,6 %), SCIEX 778 (22,9 %), Agilent 508 (14,9 %), Waters 398 (11,7 %), Bruker 235 (6,9 %), LECO 93 (2,7 %), Shimadzu 59 (1,7 %), PerkinElmer 5, JEOL 1. Gli `.d` ambigui sono Agilent 222 contro Bruker 45; i `.raw` ambigui Thermo 1 085 contro Waters 73.

**A.3 Metabolomics Workbench**. Query: `GET https://www.metabolomicsworkbench.org/rest/study/study_id/ST/analysis` (7 321 analisi MS in 4 342 studi), campi `ms_instrument_name`, `ms_instrument_type`.
| Produttore | Analisi | Quota | Studi |
|---|---|---|---|
| Thermo | 3 705 | 50,6 % | 2 130 |
| Agilent | 1 726 | 23,6 % | 1 083 |
| SCIEX | 1 022 | 14,0 % | 611 |
| Waters | 365 | 5,0 % | 232 |
| Bruker | 216 | 3,0 % | 151 |
| LECO | 166 | 2,3 % | 164 |
| Shimadzu | 53 | 0,7 % | 48 |
Per tipo: Orbitrap 3 378, QTOF 1 410, **triplo quadrupolo 1 227** (SCIEX 628, Agilent 315, Thermo 149, Waters 100, Shimadzu 22), quadrupolo singolo 332 (Agilent 243), TripleTOF 291, TOF 218 (Agilent 194), GC-TOF 149 (LECO 130), QTRAP 83, GC×GC-TOF 29.

**A.4 MassIVE**: non raggiungibile dal cloud (connessione chiusa). Lo fa la chat locale (`docs/studi/PROMPT_ROSINA/LOC-1.md`): l'API PROXI non dà lo strumento, quindi si contano le estensioni dei file dei dataset.

**A.5 Lettura e distorsioni**. I depositi sono ricerca: la proteomica è quasi tutta Orbitrap; i laboratori ambientali e di routine (QqQ, GC-MS a quadrupolo singolo) depositano poco, e Shimadzu e PerkinElmer sono quasi assenti dai depositi ma presenti nei laboratori. Per il nostro dominio (inquinanti, LC-QqQ, GC-MS) il peso giusto è quello di Workbench sul triplo quadrupolo e sul GC: **SCIEX e Agilent prima, poi Thermo, Waters, Shimadzu**.
Priorità di Rosina (dati + dominio): 1. Thermo (la metà dei file in ogni deposito); 2. SCIEX `.wiff` (QTRAP del laboratorio e un quarto dei QqQ; TripleTOF/ZenoTOF ancora rifiutati); 3. Agilent; 4. il convertitore mzML (un terzo degli studi deposita mzML/mzXML: conta come gli altri formati); 5. Waters; 6. Bruker timsTOF; 7. Shimadzu; 8. netCDF da LECO/Shimadzu/Thermo; 9. mzData e imzML (≈1 % ciascuno); MGF come corsa e `.wiff2`/BAF solo come guida.

## D. Politica dei dati in `mzlab-dati`

Stato (19/10): il privato ha `dati_formati/` alla radice; il riordino in `dati/formati/` è preparato dal lato pubblico dalla PR #1052 (risolutore unico `tools/dati_percorsi.py`/`.rs` + `tools/dati_mappa.tsv`, aperta, non unita): lo spostamento nel privato viene dopo la sua unione. Rosina usa solo i percorsi del risolutore (`percorso("dati/formati/…")`), mai percorsi a mano.

- **Struttura**: `dati/formati/<produttore>/<strumento>/<accession>_<esperimento>/` con dentro il nativo (file o cartella), `riferimenti/<oracolo>-<versione>/` (mzML del produttore o degli autori, se piccoli) e `verita/<file>.verita.json`. Produttori: `thermo`, `sciex`, `agilent`, `waters`, `bruker`, `shimadzu`, `leco`, `aperti` (mzML, mzXML, mzData, imzML, netCDF senza produttore noto). Strumento = quello letto nel file, non quello dichiarato dal deposito (spesso sbagliato, vedi `INDICE.md`).
- **Indice unico** `dati/formati/INDICE.tsv` (da cui si genera `INDICE.md`): `percorso`, `produttore`, `modello_nel_file`, `versione_formato`, `esperimenti` (vocabolario chiuso: `fullscan`, `sim`, `mrm`, `smrm`, `ms2`, `dda`, `dia`, `mse`, `prm`, `pis`, `nl`, `epi`, `ms3`, `ims`, `uv`, `analogico`, `polarita-mista`, `lockmass`, `profilo`, `centroidi`, `gc`, `imaging`).
  Altre colonne: `fonte`, `accession`, `doi`, `licenza`, `ridistribuibile` (`si`, `no`, `da-verificare`), `byte`, `sha256` (file o manifest), `url`, `riferimento` (`nessuno`, `autori`, `msconvert-<v>`, `trfp-<v>`, …), `verita` (`si`/`no`), `note`.
- **File grandi fuori da git**: `dati/formati/GRANDI.tsv` (stesse colonne + `destinazione`) e lo script `tools/rosina/scarica.py GRANDI.tsv DEST` del repository pubblico (nessun dato dentro: legge l'elenco, scarica con ripresa, controlla byte e checksum, rifiuta oltre il tetto di `controlla_tetto.sh`). Sul Mac i grandi stanno in `~/mzLab/rosina-dati/` (tetto 20 GB); nel privato vanno solo file ≤ 90 MB (regola di `tools/controlla_dati.py`) e le verità.
- **Repository pubblico**: solo file sintetici (`tools/*_sintetico.py`, già per ogni famiglia) e, dopo il controllo della licenza da parte di Federico, piccoli file CC0/CC-BY/MIT/Apache (≤ 1 MB ciascuno, ≤ 10 MB in tutto) in `tests/formati/` con `ATTRIBUZIONI.md`, senza nomi di persone o di campioni clinici. Le fixture di ProteoWizard e i file «EBI terms of use» restano privati.
- **Licenze**: `dati/formati/_licenze/` con il testo della licenza di ogni fonte e degli SDK usati come oracoli.

## E. Piano di chat

Prompt in `docs/studi/PROMPT_ROSINA/` (regole comuni come quelle dei pacchetti DEV dei formati, aggiornate). Ordine e dipendenze:

| # | Pacchetto | Tocca | Dipende da | Chi scarica / produce verità |
|---|---|---|---|---|
| LOC-1 | Scaricare ELENCO-1 (18 voci, ≈ 0,8 GB), contare MassIVE, provare gli oracoli sul Mac (Docker+Wine, TRFP, `timsdata`) | `~/mzLab/rosina-dati/`, privato | — (subito) | chat locale |
| R-00 | Banco comune: formato `verita/1`, `tools/rosina/` (verità da mzML e da TRFP, scaricamento, indice), test Rust `verita.rs` per famiglia, confronto dei metadati, bersagli fuzz di `mzlab-lettori`, workflow notturno del privato | `tools/rosina/`, `crates/mzlab-lettori/tests/`, `fuzz/`, privato | — | — |
| LOC-2 | Oracoli su tutto il corpus: mzML di riferimento (senza peak picking e con `peakPicking vendor`), JSON di TRFP, 1/K0 di `timsdata`, verità con gli script di R-00 | privato | R-00, LOC-1 | chat locale |
| R-01 | Convertitore mzML: validazione XSD e semantica, profilo/centroidi dichiarati, andata e ritorno con lettori terzi | `lettori_mzml`, `lettori_centroidi`, scrittori | R-00 | — |
| R-02 | Thermo: TSQ, PDA, lock mass, trappole, metadati contro TRFP | modulo `thermo`, `metodo_raw` | R-00, LOC-2 | — |
| R-03 | SCIEX `.wiff`: QTRAP 4000-7500, periodi multipli, EPI, vial, DAD; TripleTOF/ZenoTOF in un modulo nuovo `sciex-tof` | modulo `sciex`, nuovo `sciex-tof` | R-00, LOC-2 | Federico: Analyst del 3200 |
| R-04 | Agilent: Q-TOF 284 B e 6546, DAD, tracce dei moduli, 6560 rifiuto onesto, metadati | modulo `agilent` | R-00, LOC-2 | — |
| R-05 | Waters: spettri TOF, MSe, DDA, lock mass, deriva | modulo `waters` | R-00, LOC-2 | — |
| R-06 | Bruker: 1/K0, compressione 2, TSF LC; BAF come guida | modulo `bruker` | R-00, LOC-2 | — |
| R-07 | Shimadzu: parità con msconvert, polarità, CE/dwell, 9030 in un modulo nuovo, PDA | modulo `shimadzu`, nuovo `shimadzu-tof` | R-00, LOC-2 | — |
| R-08 | Formati aperti minori: netCDF di LECO/Shimadzu/Thermo, mzData, `metodo/1` di mzML altrui sui veri, imzML (solo valutazione) | `andi`, nuovo `storici` | R-00 | — |

Ordine: LOC-1 e R-00 subito e in parallelo; LOC-2 dopo R-00; R-01…R-08 dopo LOC-2 (R-01 e R-08 anche prima: non chiedono oracoli dei produttori), in parallelo tra loro perché ognuna tocca solo la sua cartella e il suo file d'area. Priorità quando le chat non sono tutte possibili insieme: R-02, R-03, R-04, R-01, R-05, R-06, R-07, R-08.
**Tetti**: nessuna chat alza un tetto esistente (motore 1 703 936; moduli come `lettori-modulo.md`; `finder` 512 000; `chem` 569 344). Un lettore o un pezzo che non entra va in un modulo nuovo della stessa famiglia (`sciex-tof`, `shimadzu-tof`, `storici`) con tetto proposto = misura + 3 %, scritto dalla chat e approvato dal Coordinatore. Le prove con oracoli stanno in test e `tools/`: peso zero nel sito.

## F. Protocollo di prova unico

### F.1 Il file di verità `verita/1`
JSON per file nativo (uno per campione nei contenitori a più campioni): `schema`, `file` (nome logico, SHA-256), `oracolo` (`nome`, `versione`, `opzioni`, `data`, `piattaforma`), `conteggi` (spettri per livello, cromatogrammi per tipo), `scansioni[]` (`id`, `rt_min`, `livello`, `polarita`, `precursore_mz`, `isolamento`, `ce`, `attivazione`, `profilo`, `n_punti`, `tic`, `bpi_mz`, `bpi`), `punti` (per 8 scansioni scelte: prima, ultima, mediana, TIC massimo, e una per livello: m/z e intensità interi), `cromatogrammi[]` (`id`, `tipo`, `n`, `somma`, `massimo`, 16 punti campione).
`metadati`: campo → valore come lo dà l'oracolo, con il nome del produttore e il percorso `metodo/1` atteso, da `SCHEDE.md`. Si genera dall'mzML di riferimento (`tools/rosina/verita.py MZML`) o dall'oracolo (TRFP JSON, `timsdata`). Dimensione tipica: 50-500 KB.

### F.2 Livelli di prova (ogni lettore, ogni file del corpus)
| Livello | Controllo | Tolleranza |
|---|---|---|
| F0 struttura | `riconosci` giusto, campioni, nessun panico, errori come chiavi | — |
| F1 conteggi | spettri per livello, cromatogrammi per tipo, insiemi di id | uguali |
| F2 scansioni | RT, livello, polarità, precursore, isolamento, CE, tipo, profilo/centroidi | RT 1e-4 min; m/z 1e-4; resto uguale |
| F3 punti | m/z, intensità, numero di punti | HR 0,5 ppm (1 ppm con taratura per scansione dichiarata), LR 1e-3 Da; intensità relativa 1e-5 (f32 esatti se il file ha f32) |
| F4 cromatogrammi | TIC, BPC, SRM/SIM, analogici, UV: stessi id, tempi, valori | tempi 1e-4 min; valori 1e-6 relativo |
| F5 metadati | ogni campo della verità: uguale, mappato con conversione dichiarata, oppure difetto | testo uguale dopo normalizzazione dichiarata (spazi, unità) |
| F6 convertitore | XSD, `FileInfo -v`, offset e SHA-1 dell'`indexedmzML`, rilettura = lettore, profilo/centroidi e `dataProcessing` giusti | nessun errore |
| F7 robustezza | fuzz ≥ 300 s per famiglia, troncamento a 1/4, 1/2, 3/4, memoria entro `Limiti` | nessun panico né memoria fuori tetto |
| F8 scala (solo locale) | file più grande della famiglia: tempo d'apertura, picco di memoria | scritti in `misure.md` |
Esito per file: `L` (F0-F5 contro V1 o V2), `P` (manca un livello, scritto), `NV` (solo V7). La riga del formato in `FORMATI_NATIVI_MS.md` si aggiorna da questo esito, non da un'impressione.

### F.3 Dove gira
- **CI pubblica** (`test.yml`): F0-F4 e F6 sui sintetici e sui piccoli pubblici; F5 sui documenti `metodo/1` dei sintetici.
- **CI privata** (nuovo workflow notturno in `mzlab-dati`, proposto in R-00): checkout di `Freeesta/mzlab` `main`, `cargo test -p mzlab-lettori --features tutte --test verita` su ogni `verita/*.json` del corpus (file ≤ 90 MB); il riassunto (formato × livello) in un artefatto, mai nel pubblico.
- **Locale** (Mac): F8 e i file in `~/mzLab/rosina-dati/`, con lo stesso test e `MZLAB_DATI_GRANDI=<percorso>`.

### F.4 Casi limite: ogni cella vuota è «nessun file», da cercare
| Caso | Thermo | SCIEX | Agilent | Waters | Bruker | Shimadzu |
|---|---|---|---|---|---|---|
| taratura per scansione / lock mass | lock mass LTQ Velos Pro, XL (corpus) | — | Q-TOF `MSMassCal.bin` (corpus) | MTBLS1584, MTBLS178 (corpus) | temperatura per frame (corpus) | — |
| centroidi del produttore contro profilo | Q Exactive (corpus) | QTRAP profilo (laboratorio) | ST003656 profilo + centroidi | G2-S (corpus) | TSF linee/profilo (corpus) | nessun file |
| cambio di polarità | MTBLS7889, MTBLS1752 (corpus) | Zenodo 22673685 (ELENCO-1) | nessun file | nessun file | — | nessun file |
| eventi di scansione multipli / periodi | Tribrid DIA (corpus) | Z15679243 16 esperimenti; periodi multipli: nessun file | segmenti di tempo (corpus) | funzioni (corpus) | — | eventi 2020 (corpus) |
| mobilità ionica | FAIMS: nessun file | SelexION: nessun file | MTBLS427 6560 (ELENCO-1) | Synapt HDMS, cyclic (corpus) | timsTOF (corpus) | — |
| unità di tempo/intensità | min | s/min per esperimento | min | min | s, normalizzazione a 100 ms | ms |
| versioni del formato | v57-v66 (corpus: da contare in R-02) | Analyst 1.4-1.7 / SCIEX OS | B.0x-12 | 4.1/4.2 | timsControl 1-5 | 5.x |
| file > 4 GB | Astral/Orbitrap grandi (`_grandi`) | `.wiff.scan` > 4 GB (`_grandi`: 1,1 GB max) | ST000044 2,1 GB | PXD051707 7,9 GB | alphatims 1,9 GB | nessun file |
| più campioni per contenitore | — | Z22807392 20 campioni (corpus) | — | — | — | — |
| file troncati o in acquisizione | da costruire | da costruire | da costruire | da costruire | PXD074950 `-wal` (corpus) | da costruire |
| endianness | LE | LE | LE | LE | LE (SQLite BE) | LE |

## Decisioni (raccomandazione per ogni scelta aperta)
1. **Oracoli**: la chat locale prova prima Docker+Wine di ProteoWizard sul Mac con un file per produttore; se l'emulazione non va, un PC Windows dell'università con msconvert (installazione gratuita). TRFP e `timsdata` sul Mac in ogni caso.
2. **`.wiff2`**: nessun lettore. Guida di conversione (msconvert o SCIEX MS Data Converter) e richiesta scritta a SCIEX per un progetto didattico; si riapre se SCIEX documenta il formato.
3. **Bruker BAF**: nessun lettore nel sito; guida (DataAnalysis o msconvert). `baf2sql` di Bruker è un SDK nativo: nemmeno nell'app desktop (licenza e peso), salvo richiesta di Federico.
4. **Lock mass Waters**: lettura come oggi (m/z del file) per default; la correzione con la funzione di riferimento diventa un'opzione dichiarata nell'mzML, provata contro il MassLynx SDK (R-05).
5. **Centroidi nel convertitore**: due modi, scritti nell'mzML: «centroidi del produttore» quando il file li ha (mai rifatti) e «centroidi di mzLab» (`lettori_centroidi`, `dataProcessing` con mzLab come software di peak picking). Il default della conversione resta quello di oggi.
6. **Moduli nuovi** al posto di tetti più alti: `sciex-tof` (codec TOF di TripleTOF/ZenoTOF), `shimadzu-tof` (9030/IT-TOF), `storici` (mzData, eventuale MGF come corsa).
7. **CI con i dati**: workflow notturno nel repository privato; nessun dato nel pubblico salvo i piccoli liberi approvati da Federico.
8. **imzML**: fuori da Rosina (0,8 % degli studi e una vista diversa); solo una nota in R-08.
