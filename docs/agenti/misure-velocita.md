# Tabella cumulativa delle misure di velocità

Una riga per ogni intervento di velocità (lettori, Flow, motore), dal più vecchio al più nuovo; la riga si aggiunge nella stessa PR che fa l'intervento. Regola del 22/10: anche i guadagni di pochi punti percento contano (dipendenza nuova da ≥ 3 % sul suo bersaglio). Un guadagno entra solo se supera il rumore misurato (`noise` del banco, `thermo-velocita.md`); prima e dopo con lo stesso binario di banco, la stessa macchina, il giro più veloce (`best`) su ≥ 15 giri e lo SHA di 40 caratteri di `main` per la misura «prima».

| Data | PR | Passo | Prima | Dopo | % |
|---|---|---|---|---|---|
| 2026-10-22 | #1521 | tp.link, fase interpretazione (serie sintetica 15 file, 253 MB) | 3,3 s | 0,5 s | −85 % |
| 2026-10-22 | #1524 | tp.link, fase evidenza (stessa serie, 4 worker) | 8,9 s | 4,2 s | −53 % |
| 2026-10-22 | #1524 | piano intero (idx→det→ali→gap→lnk, stessa serie) | 19,5 s | 14,5 s | −26 % |
| 2026-10-22 | #1534 | tp.link, evidenza: identità delle righe su thread (nativo) | 3,80 s | 1,07 s | −72 % |
| 2026-10-22 | #1534 | tp.link, fase evidenza (stessa serie) | 4,2 s | 1,4 s | −67 % |
| 2026-10-22 | #1542 | rilevamento: tracce, canali a 1 picco saltati (somma sui 15 file) | 1,10 s | 1,01 s | −8 % |
| 22/10 | #1552 | index mzXML zlib (`LettoreMzxml::completa`: TIC e picco base in una lettura; 12 file, 4 worker, 15 giri, `best`; `main` `84929497eee9f3ba0769aaa19463291ff070ef51`) | 2.41 s | 1.88 s | −22 % (totale 1.14 → 0.98 s, −14 %) |
| 22/10 | #1552 | index mzXML raw (stesso banco) | 1.71 s | 1.43 s | −16 % (totale 0.98 → 0.93 s, −5 %) |
| 23/10 | RM-1b | `mzlab-cli convert`, impronte una volta sola (mzXML sintetico 320 MB → mzML 323 MB, `--zlib 0`, 3 giri alternati; `main` `788e416b90033bb946d8afe942f0f17870873f76`) | 2.93 s | 2.73 s | −7 % (macchina con SHA-NI; PRIDE non rimisurato) |
| 23/10 | RM-1 | scrittore mzML, pool a blocchi con spettri restituiti al lettore (lab-b sintetico, 69 MB, liv. 0, 4 thread, `--senza-conta`, 7 giri, mediana senza primo giro; `main` `56fc84cacaf4aa67b9041567fb295023a3b27018`) | 3.40 s | 0.61 s | −82 % (1 thread 1.57 → 1.65 s, invariato) |
| 23/10 | RM-1 | idem, lab-a sintetico (70 MB), liv. 0, 4 thread | 0.86 s | 0.21 s | −76 % |
| 23/10 | feature-veloce-1 | `mzlab-cli features`, indice e rilevamento per file su thread (12 mzML HR sintetici da 1,8 MB, ~3000 feature, 4 CPU, profilo `desktop`, 5 giri; CSV identico per `cmp`; `main` `74e59a26d31b8d0e2dea27566c28f6f00aa88459`) | 0.61 s | 0.37 s | −39 % |
