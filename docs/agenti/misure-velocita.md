# Tabella cumulativa delle velocità

Una riga per PR di velocità del Flow: % sul passo e sul totale (muro, 4 worker), mediana di 5 giri, output byte-identico (`tools/confronta_profili.py`). Il banco è `tools/misura_formati.py` (serie sintetica di 12 file, profilo `cli`).

| leva | bersaglio | passo | totale |
|---|---|---|---|
| mzXML: TIC e picco base in una sola lettura delle scansioni (`LettoreMzxml::completa`) | mzXML zlib / raw | index 2.77 → 2.11 s (−24 %) / 1.86 → 1.54 s (−17 %) | 1.28 → 1.06 s (−17 %) / 1.10 → 1.02 s (−7 %) |
