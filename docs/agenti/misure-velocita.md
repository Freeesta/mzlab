# Tabella cumulativa delle misure di velocità

Una riga per ogni intervento di velocità (lettori, Flow, motore), dal più vecchio al più nuovo; la riga si aggiunge nella stessa PR che fa l'intervento. Regola del 22/10: anche i guadagni di pochi punti percento contano (dipendenza nuova da ≥ 3 % sul suo bersaglio). Un guadagno entra solo se supera il rumore misurato (`noise` del banco, `thermo-velocita.md`); prima e dopo con lo stesso binario di banco, la stessa macchina, il giro più veloce (`best`) su ≥ 15 giri e lo SHA di 40 caratteri di `main` per la misura «prima».

| Data | PR | Passo | Prima | Dopo | % |
|---|---|---|---|---|---|
| 22/10 | #1552 | index mzXML zlib (`LettoreMzxml::completa`: TIC e picco base in una lettura; 12 file, 4 worker, 15 giri, `best`; `main` `84929497eee9f3ba0769aaa19463291ff070ef51`) | 2.41 s | 1.88 s | −22 % (totale 1.14 → 0.98 s, −14 %) |
| 22/10 | #1552 | index mzXML raw (stesso banco) | 1.71 s | 1.43 s | −16 % (totale 0.98 → 0.93 s, −5 %) |
