# Misure di partenza (prima di ogni codice)

Macchina: container Linux, 4 CPU Xeon a 2,8 GHz, rustc stabile, profilo `desktop` (opt 3); commit `ac42758`. Ogni numero è una mediana di 3 giri a cache calda, salvo dove è scritto altro. Le sonde stanno nello scratchpad della chat di studio e non sono nel repository: RM-0 le rifà come esempi in `crates/` (§4). Le misure del Mac a cache fredda sono richieste alla locale (`RICHIESTE_LOCALE.md`).

## 1. Contenuto (sonda byte per byte, contro msconvert pwiz 3.0.24288 e RFR)
| Misura | LAB-A (Q Exactive, «FTMS + c») | LAB-B (Fusion, «FTMS + p») |
|---|---|---|
| Scansioni | 7952 | 16 524 |
| Byte di scarto fra pacchetti | 0 | 0 |
| Picchi della lista | 4 185 930 | 1 051 091 |
| Pezzi di profilo / bin | 0 / 0 | 1 051 091 / 6 061 269 |
| Centroidi uguali al bit a msconvert | 4 185 930 su 4 185 930 | non confrontati (l'mzML di msconvert di LAB-B è a profilo) |
| Punti di profilo non nulli uguali al bit a msconvert | – | 1 616 814 su 1 616 814 (scansioni 1-4718, le sole nell'mzML di riferimento) |
| Zeri aggiunti da msconvert | – | 2 159 784 (57 % dei suoi punti); scansione 1: 7079 punti contro 3430 nostri |
| Centroidi contro RFR | 0 ppm (INF-A, 4727 punti) | 0 ppm (1290 punti) |
| Descrittore: indice nei 16 bit bassi | sì, 0 eccezioni | sì, 0 eccezioni |
| Attivazione della scansione 2 (`@cid26`) | – | nostro: «beam-type CID» (HCD); msconvert: MS:1000133 CID |

## 2. Tempi
`raw_tempi` (`crates/mzlab-core/examples/raw_tempi.rs`):
| Fase | LAB-A | LAB-B |
|---|---|---|
| SHA-256 del file | 0,62 s | 0,62 s |
| Apertura | 0,049 s | 0,144 s |
| Decodifica di tutto | 0,070 s | 0,102 s |
| Tabella | 0,448 s | 0,593 s |
| `.mzidx` | 0,383 s | 0,201 s |
| mzML, non indicizzato | 1,812 s (98,2 MB) | 2,865 s (111,7 MB) |
| Picco di memoria (RSS) | 297 MB | 263 MB |

Banco dello scrittore (`scrivi_con`; lettura dal disco ~0,05 s):
| Misura | LAB-A | LAB-B |
|---|---|---|
| Produttore (scansione + extra) | 0,312 s | 0,375 s |
| Sola scansione | 0,094 s | 0,103 s |
| 1 thread, livello 0 / 1 / 6 | 1,349 / 1,477 / 3,497 s | 1,476 / 1,759 / 2,185 s |
| 3 thread, livello 0 / 1 / 6 | 2,071 / 2,089 / 2,159 s | 2,708 / 2,737 / 2,963 s |

`mzlab-cli convert` (indicizzato, SHA-1, cache `.convert`), poi `mzlab-cli validate`:
| Misura | LAB-A | LAB-B |
|---|---|---|
| `convert --fresh`, livello 1 | 3,96 s, 98,8 MB | 5,97 s, 113,0 MB |
| `convert --fresh`, livello 0 | – | 5,74 s, 118,5 MB |
| RSS durante `convert` | – | ≤ 66 MB, CPU ~130 % |
| `validate` | valido, 0 errori, 1,74 s | valido, 0 errori, 2,35 s |

Callgrind (LAB-B, mzML, 1 thread, livello 1, inclusivo):
| Funzione | Peso |
|---|---|
| deflate | 54 %: `reset` di miniz_oxide (memset) 43,5 % e compressione vera 10,6 % |
| `testata_mzml` | 14,75 %: SHA-1 del `.raw` 7,25 % e `metodo_raw` 7,4 % |
| `spectrum_head` | 11,3 % |
| `extra_scansione` | 9,8 % |
| `esc` | 6,1 % |
| `format` | 5,6 % |
| malloc/free | ~10 % |

## 3. Micro-banchi della compressione (µs per array; MB/s; rapporto compresso/originale)
| Metodo | Centroidi LAB-A (4,2 KB) | Centroidi LAB-B (385 B) | Profilo LAB-B (2,3 KB) | Profilo denso sintetico (200 000 punti, 1,2 MB) |
|---|---|---|---|---|
| miniz L1 con reset (oggi) | 40 µs, 106 MB/s, 0,588 | 16 µs, 0,73 | 26-30 µs, 0,831 | 124-156 MB/s, 0,80 |
| miniz L0 | 33 µs | – | – | 92-162 MB/s |
| zlib-rs L1, flusso riusato | 26 µs, 164 MB/s, 0,726 | 6,5 µs | 1,039 (espande) | 79 MB/s, 1,04 |
| zlib-rs L0, flusso riusato | 3,4 µs, 1250 MB/s | 3,2 µs | 697 MB/s | 6940 MB/s |
| fdeflate | 780 MB/s, 1,127 | 1,278 | 1,425 | 1,40 |
| base64 | 1,5-1,8 GB/s | | | |

Solo Huffman e solo RLE di miniz danno lo stesso rapporto di L1: nessun guadagno. fdeflate espande i float ed è scartato.

Composizione dell'uscita di LAB-B: ~6,3 KB di XML per spettro (soprattutto `userParam` del trailer), cioè ~104 MB di XML su 113 MB. Gli array sono ~12 MB prima di base64. Per i file come LAB-B la compressione cambia poco la dimensione; conta per i profili densi.

Disco del container:
| Operazione | Banda |
|---|---|
| Lettura diretta | 2,0 GB/s |
| Lettura in cache | 5,0 GB/s |
| Scrittura diretta | 1,6 GB/s |

## 4. Come si ripete (da RM-0 in poi, per un terzo)
- **Comando unico**: `python3 tools/bench/raw_mzml.py --file PERCORSO --rip 5 --freddo`, da scrivere in RM-0. Lo script lancia `mzlab-cli convert` e gli esempi del banco.
- **Cache fredda**: su macOS `sudo purge`; su Linux `echo 3 > /proc/sys/vm/drop_caches`. Se il comando non è permesso, si lavora su una copia nuova del file per ogni giro e lo si dichiara.
- **Uscita**: un JSON per giro con macchina (modello di CPU, core, RAM, disco, sistema), versione di rustc, SHA del commit, profilo, SHA-256 dell'ingresso e dell'uscita, tempo per fase, RSS di picco per fase, numero di allocazioni.
- **Strumenti esterni**: `hyperfine` per msconvert e ThermoRawFileParser, con la versione di ogni strumento.
- **Risultato**: la mediana di 5 giri e lo scarto interquartile; il primo giro a parte.
