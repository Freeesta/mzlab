# Misure del Flow per formato di ingresso

Tempo di index → detect → align → gapfill su 12 file sintetici (1365 scansioni, 185544 picchi ciascuno, serie di `tools/dati_sintetici.py`) riscritti nel formato da provare con `tools/misura_formati.py genera`; `tools/misura_formati.py misura BIN DIR...` fa un solo `mzlab-cli flow run` con 4 worker e somma i `ms` per istanza di `prov.json`. Binario `--profile cli`, main `3e5dd52cc51d6b74305af40a1558a4de0d387e8a`, 4 CPU, senza SHA-NI, file nella cache del sistema (cache fredda non ottenibile nel contenitore); il rumore tra due giri è ±15 %.

| formato | MB | index s | detect s | align s | gapfill s | totale s (muro) | MB/s (muro) | index MB/s per core |
|---|---|---|---|---|---|---|---|---|
| mzML raw | 74 | 1.29 | 0.51 | 0.05 | 0.09 | 1.07 | 69 | 57 |
| mzML raw indicizzato | 75 | 1.10 | 0.45 | 0.05 | 0.09 | 0.98 | 77 | 68 |
| mzML zlib | 73 | 1.63 | 0.47 | 0.05 | 0.09 | 1.14 | 64 | 45 |
| mzML zlib indicizzato | 75 | 1.39 | 0.44 | 0.04 | 0.10 | 1.03 | 73 | 54 |
| mzXML raw | 54 | 1.73 | 0.48 | 0.05 | 0.09 | 1.01 | 54 | 31 |
| mzXML zlib | 44 | 2.49 | 0.47 | 0.05 | 0.09 | 1.36 | 32 | 18 |
| ANDI/AIA .cdf (*) | 19 | 0.88 | 1.17 | 0.47 | 0.61 | 1.82 | 10 | 22 |

Come si leggono le colonne: i secondi per passo sono la somma sulle 12 istanze (tempo di CPU, non di muro); «totale» è il muro dell'intera corsa con 4 worker; «index MB/s per core» è MB / index s. (*) Il .cdf è scritto da un codificatore CDF1 di `misura_formati.py` (non è un file di strumento) e non ha l'analizzatore: la ricetta di rilevamento è quella a risoluzione unitaria, quindi detect, align e gapfill NON sono confrontabili con le altre righe; vale solo la colonna index.

## Il peggiore e perché
- **Più lento: mzXML con zlib, passo index, 18 MB/s per core** (mzXML raw 31, mzML 45-68). Profilo (callgrind, un file): `LettoreMzxml::completa` gira 3 volte per ogni scansione (da `scansione`, `indice` e `cromatogramma`, 21 % ciascuna del totale) e ogni volta decodifica base64 e inflate dei picchi; in più memset/memcpy 24 %, ricerca di caratteri e conversione UTF-8 12 %, SHA-256 + SHA-1 del file 26 % (in mzML la SHA-1 non c'è più, e senza SHA-NI il costo qui è sovrastimato).
- Il passo detect è uguale per tutti i formati centroidati (0.44-0.51 s): il formato pesa solo sull'index.
- Align e gapfill non dipendono dal formato.

## Non misurato (servono i file del Mac di Federico)
Thermo .raw (lettore nostro), Agilent .D, Bruker .d, SCIEX .wiff/.wiff.scan, .cdf di strumento e MGF: nel contenitore non ci sono file veri né i convertitori. Il rapporto con mzmine (serie 23-29: 0.56-0.64) non ha un numero nel repository, per cui non è riportato.
