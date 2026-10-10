# `.mzidx`: l'indice di un file mzML (`mzlab/reader/mzidx.py`)
Stato attuale. Un `.mzidx` si scrive una volta per file (`build(run, out)`) e si legge a fette (`open(path)`): la tabella delle scansioni più, per ogni gruppo di scansioni, i picchi ordinati per m/z. 

## Formato (tutto little-endian, nessuna compressione)
```
0      HEADER, 64 byte:  "MZIDX001"(8) | u32 versione=1 | u32 flags=0 | u64 meta_off | u64 meta_len | u64 scans_off | u64 scans_len | u64 total | 8 byte a zero
64     per ogni sezione, nell'ordine di meta.sections, sette array ciascuno allineato a 64 byte (riempimento a zero):
         rt        n_scans × f8   minuti
         tic       n_scans × f8   TIC dichiarato dal file
         scan_ids  n_scans × i8   indice della scansione nel file
         fence     n_fence × T    primo m/z di ogni blocco di 16384 picchi (n_fence = ceil(n_peaks / 16384))
         mz        n_peaks × T    tutti i picchi, ordinati per m/z (ordinamento stabile: a parità, prima la scansione, poi l'ordine nella scansione)
         inten     n_peaks × T    intensità, nello stesso ordine
         pos       n_peaks × i4   posizione della scansione di appartenenza dentro rt/tic/scan_ids
scans_off  JSON UTF-8 (separatori compatti): lista di righe, una per scansione, nell'ordine del file, con le colonne di SCAN_COLUMNS:
         start,end (byte dell'mzML),level,rt,polarity,tic,precursor,collision_energy,filter,profile,parent,iso,act,res,an,pint,path,precursors,charge,native
meta_off   JSON UTF-8: {format:"mzidx", version, block, source:{name,size,mtime_ns,probe,sha256}, run:{nce,hr1,n_chromatograms},
                        keys:{"<livello>/<all|+|->": indice di sezione}, sections:[{level,dtype,n_scans,n_peaks,n_fence,off:{rt,tic,scan_ids,fence,mz,inten,pos}}]}
total      dimensione del file: un file di dimensione diversa (troncato) non si apre
```
- **T** = `<f4` per l'MS1 di un file ad alta risoluzione (`Run.hr1`), altrimenti `<f8` (stessa regola della tabella in memoria).
- **Sezioni**: una per ogni livello × polarità (`all`, `+`, `-`: `+` prende le scansioni di polarità +1 o 0, come `Run.table(livello, 1)`); gruppi con le stesse scansioni condividono la sezione. `keys` dice quale.
- **Validità**: `source.size`, `source.mtime_ns` (solo con la cartella predefinita) e `source.probe` (SHA-1 di dimensione, primi e ultimi 64 KB) devono combaciare con il file; `source.sha256` è l'impronta dell'intero file (la stessa che userà `.mzflow`).
- **Scrittura atomica**: file temporaneo accanto alla destinazione, poi rinomina; l'intestazione si scrive per ultima.

## Costruzione (`build`)
1. Le scansioni sono già nella tabella di `Run`: con un `indexList` (indexedmzML) le posizioni degli spettri vengono da lì e si leggono solo le intestazioni (fino a `<binaryDataArrayList`), mai i dati binari; senza `indexList` si percorre il file come prima (`Run._spans`).
2. Ogni scansione si decodifica una volta (`Run.read`) e i suoi picchi vanno, a gruppi di ~1 M di picchi, in secchi di 1 unità di m/z (0…9999, i valori fuori scala negli estremi); oltre `BUILD_BYTES` (64 MB, divisi per le sezioni) i secchi si scaricano in un file temporaneo. La memoria resta limitata: non si concatena mai tutto il file.
3. Secchio per secchio, nell'ordine di m/z, si ordina in modo stabile e si scrive nei tre array; il primo m/z di ogni blocco entra in `fence`.
Il risultato è identico alla tabella in memoria (`Run.table`): lo verifica `tests/test_mzidx.py` su tutti i file di esempio e sui sintetici.

## Lettura
- `open(path)` → `MzIdx`: `meta`, `scans()` (le `Scan` come le dà il lettore dell'mzML), `table(livello, polarità)` → `PeakTable` i cui `mz`, `inten`, `pos` sono `LazyArray`.
- `LazyArray`: `a[i:j]` legge la fetta (fino a 2 blocchi dalla cache a blocchi di 16384 elementi, oltre direttamente dal file); `searchsorted` usa `fence` e legge un solo blocco; maschere, operatori e `np.asarray` la leggono intera (temporaneamente).
- **Cache a blocchi**: LRU condivisa da tutti i file, tetto `CACHE_BYTES` = 64 MB (`CACHE_BYTES_SMALL` = 32 MB su tablet e telefono, `set_cache_limit`). Il tetto sta solo in `mzidx.py`.

## Dove sta
`mzidx.cache_dir()` = `$MZLAB_CACHE` (predefinita `~/.cache/mzlab`); `index_name(percorso)` = SHA-1 di percorso reale, dimensione e data; `trim(cartella, limite)` cancella i più vecchi.
