# Banco dei picchi spalmati (mzLab e MZmine)

Misura quanto perdono mzLab e MZmine sui picchi cromatografici larghi (FWHM 60-240 s) rispetto ai picchi stretti. Solo strumenti in `tools/`: nessun prodotto cambia. Le metriche e il formato `bench/1` sono in `mzfinder-benchmark.md`; qui il lotto, le configurazioni e il modo di leggere la tabella.

## Lotto sintetico
- **Generare**: `python3 tools/valida/sintetici_mzml.py CARTELLA --seme 11` scrive `blank_01.mzML`, `camp_01..03.mzML` e `verita.json` (37 righe, 4 file, ~70 MB; deterministico a seme fisso, stessi byte su ogni macchina).
- **Composti**: larghi (a: gaussiana o EMG; b: doppio plateau con sella al 60 %; c: banda di 150 s con 1-2 picchi stretti di 8 s entro 3 ppm) su un fondo a gradiente su ~200 canali m/z; i file sono ordinati blank, poi i campioni.
- **Verità** (`verita.json`): per composto `ruolo` (broad/narrow), `rt`, `fwhm_s`, `rt_min`/`rt_max` (il 99 % centrale dell'area) e `area_attesa` (lista per file in intensità·min, 0 sul bianco).
- Il lotto non contiene dati del laboratorio.

## Configurazioni MZmine 3.9.0
Due file in `tools/bench/parametri/mzmine/`, scelti con `--config`:
- `mzmine_default`: il flusso documentato (massa centroide, ADAP, risolutore «minimum search» con soglia cromatografica 0,85, ampiezza fino a 1 min, join aligner, gap filling), uguale a `predefinita`.
- `mzmine_wide`: stesso flusso, cambia solo il risolutore: soglia cromatografica 0,60, intervallo di ricerca 0,5 min, ampiezza fino a 8 min.
MZmine 4.10.90 legge gli stessi due file attraverso `oro_mzmine410.batch_xml`. Il 4.10.90 non ha un risolutore wavelet separato (i moduli sono minimum search, noise amplitude, Savitzky-Golay): nessuna variante wavelet. `tests/test_bench_mzmine_larghi.py` controlla solo la forma (JSON, XML, valori del risolutore); MZmine non gira nel cloud.

## Eseguire
mzLab, ovunque (serve cargo):
`python3 tools/bench/corri_mzlab.py CARTELLA_LOTTO USCITA --dataset SPAL`
MZmine su Mac (il programma scarica solo JDK Linux: su Mac si indicano MZmine e Java a mano):
```
mkdir -p ~/mzmine39 && cd ~/mzmine39
curl -L -o mzmine.tar.bz2 https://conda.anaconda.org/bioconda/noarch/mzmine-3.9.0-hdfd78af_0.tar.bz2
shasum -a 256 mzmine.tar.bz2    # d596078684c7e23af7c7e350dded437ecbf6b2851aadc856c318c3cac2645504
mkdir mzmine && tar -xjf mzmine.tar.bz2 -C mzmine && chmod +x mzmine/bin/MZmine
curl -L -o jdk.tar.gz "https://api.adoptium.net/v3/binary/latest/20/ga/mac/aarch64/jdk/hotspot/normal/eclipse"   # Intel: x64
mkdir jdk && tar -xzf jdk.tar.gz -C jdk
cd -   # radice del repository
J=$(ls -d ~/mzmine39/jdk/*/Contents/Home)
for C in mzmine_default mzmine_wide; do
  python3 tools/bench/corri_mzmine.py CARTELLA_LOTTO USCITA --dataset SPAL --config $C --mzmine ~/mzmine39/mzmine --java-home $J
done
```
Escono `USCITA/mzmine_SPAL_mzmine_default.json` e `..._mzmine_wide.json` (`bench/1`).

## Leggere la tabella
`python3 tools/bench/confronto_larghi.py --verita CARTELLA_LOTTO/verita.json --tabelle "MZmine default=USCITA/mzmine_SPAL_mzmine_default.json" "MZmine wide=USCITA/mzmine_SPAL_mzmine_wide.json" "mzLab default=USCITA/mzlab_SPAL_predefinita.json"` stampa una colonna per strumento:
- **P1_narrow / P1_broad**: recall dei composti stretti e larghi (una banda larga è trovata se |ΔRT| ≤ max(tolleranza, 0,35·FWHM) o se l'IoU degli intervalli è ≥ 0,40). Il divario tra le due righe è la perdita sui picchi spalmati.
- **P2_broad**: mediana dell'errore relativo dell'area sui larghi trovati (0 = esatta; 1 = il doppio o zero; oltre 1 = forte sovrastima).
- **P3_split**: righe per banda (media) e bande spezzate in più di una riga: un risolutore stretto taglia la banda in frammenti.
- **P5_fp**: righe che non corrispondono a nessun composto, in numero e in frazione; con il gradiente di fondo il valore alto di una colonna dice quanto rumore diventa «picco».
Confronto atteso: `mzmine_wide` deve alzare P1_broad e abbassare P3_split rispetto a `mzmine_default`, a costo di più P5_fp; una colonna che non si muove è un risultato, non un errore.
