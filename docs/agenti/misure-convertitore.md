# Misure: conversione in mzML (codifica in parallelo)

`convert_tempi` (`crates/mzlab-core/examples/`) converte un mzML già in memoria con `Conversione::scrivi` e stampa tempo, picco dell'heap, picco del processo, byte e SHA-256; il SHA-256 gira su un thread a parte come farebbe una scrittura su file. Container di 4 core, mzML sintetici di `tools/mzml_enorme.py` (0,29 GB: 5271 scansioni, `--peaks1 20000 --peaks2 2000`; 0,4 GB: scansioni piccole, predefiniti), stesso SHA-256 con 1, 2 e 4 thread:

- spettri grandi: 1 thread 4,09 s, 2 thread 2,83 s, 4 thread 1,75 s (2,3×); picco dell'heap 5,5 MB → 18 MB (in gran parte la coda del SHA dell'esempio);
- spettri piccoli: 1 thread 6,66 s, 2 thread 3,95 s, 4 thread 3,18 s (2,1×); picco del processo invariato (237 → 239 MB, il file letto).

La finestra è 4×`thread` spettri: gli MS1 costano 10 volte un MS2 e lo spettro in testa all'ordine ferma i thread, quindi 2× non basta (1,5-1,7×).
