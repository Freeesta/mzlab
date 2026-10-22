# Richieste alla chat locale (bozza)

Il Coordinatore le copia in `coordinamento/SCAMBIO_LOC.md` del repository privato come R-nn. Qui la lettera provvisoria (R-a…R-e). File solo per alias o accession; le uscite restano nel privato (`scratchpad/rfr/`, `scratchpad/raw-mzml/`); nel pubblico solo aggregati. RFR solo fuori da ogni repository; ThermoRawFileParser solo con l'autorizzazione di Federico.

- **R-a Calibranti**: un `.raw` di infusione della miscela di calibrazione positiva (caffeina 195,08765; MRFA 524,26496; Ultramark 1621 a partire da 1022,0) su un Orbitrap del laboratorio o pubblico (accession), a profilo. Serve per C3 ed E4.
- **R-b Dump RFR completi**: per LAB-A, LAB-B, INF-A, BAF e un Q Exactive a profilo, `scandump/1` con:
  - il profilo come lo dà RFR, con gli zeri;
  - `CentroidStream` con Masses, Intensities, Charges, Flags, Resolutions, Noises e Baselines;
  - i nodi del rumore, se RFR li espone;
  - per ogni scansione, il tipo di attivazione e l'energia che RFR legge dall'evento.
  
  Serve per F1-F5 e chiude D1, D2 e D4.
- **R-c msconvert completo e tempi**:
  - per LAB-A, LAB-B (tutte le 16 524 scansioni) e BAF: msconvert pwiz (versione dichiarata) a profilo e con `peakPicking vendor msLevel=1-`, mzML indicizzato zlib;
  - tempi a cache fredda (`sudo purge`, 5 giri, mediana) di msconvert, RFR (aprire e leggere tutte le scansioni), TRFP se autorizzato e `mzlab-cli convert` dello stesso commit;
  - la macchina descritta come in `MISURE.md` §4.
  
  Serve per D3, E2, E3 ed E8.
- **R-d Validatori e feature**: sui nostri mzML (forniti dalla chat DEV come impronte e comando):
  - il validatore PSI, OpenMS `FileInfo -v`, pyteomics, mzR, msconvert che rilegge;
  - il lotto mzmine e SIRIUS su LAB-B, dal nostro mzML e da quello di msconvert.
  
  Serve per F8, E5, E6 ed E7.
- **R-e File pubblici**: da PRIDE/MassIVE, con l'accession e SHA-256, almeno uno ciascuno di:
  - Exploris 480;
  - Astral;
  - DIA;
  - FAIMS;
  - cambio di polarità;
  - MS3 SPS.
  
  Serve per D5, D6 e il corpus di F1-F9.
