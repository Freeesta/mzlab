# Dubbi aperti e decisioni per il Coordinatore

## Dubbi (chi li chiude, con quale prova)
- **D1 Bit alti del descrittore**: nei 16 bit bassi c'è l'indice; nei bit alti ci sono multipli di 0x100 fino a 0x1700, probabilmente la carica (bit 24-31?), e 0x10/0x20/0x40/0x80, probabilmente flag (bit 16-23: monoisotopico, referenza, eccezione, frammentato?). Si chiude con il dump RFR di `CentroidStream.Charges` e `Flags` (R-b), confrontato picco per picco (RM-2).
- **D2 Regola degli zeri di RFR/msconvert**: gli zeri sono bin adiacenti ai pezzi, con corse tipiche di 8 tra i pezzi e di 4 ai bordi, ma la regola esatta non è nota (pezzi vicini che si toccano, bordi del range, minimo). Si chiude derivandola dai dump di profilo di RFR (R-b) su LAB-B e su un file Q Exactive a profilo; prova F3.
- **D3 Riferimento parziale**: l'mzML di msconvert di LAB-B copre solo le scansioni 1-4718 su 16 524. Serve un mzML completo, a profilo e con `peakPicking vendor` (R-c).
- **D4 Rumore per picco**: RFR dà rumore e linea di base per picco (interpolati dai ~40 nodi?) oppure i nodi? Se interpola, la formula va ricavata e verificata al bit, altrimenti si scrivono i nodi (R-b).
- **D5 Exploris e Astral**: layout del pacchetto non visto (analizzatore Astral, nparam, f64). Servono file pubblici (R-e) e la sonda di RM-0.
- **D6 FAIMS, DIA, cambio di polarità, msx**: come appaiono nel filtro e nel trailer, e quali termini CV scrive msconvert. Servono file pubblici (R-e).
- **D7 Lettori di terzi**: accettano array non standard, `arrayLength` diversi nello stesso spettro, zlib a blocchi stored e nessuna compressione? Prova E6 (RM-7). Ripiego: colonne per picco in un file accanto (`.mzidx` con il blocco colonnare di T-7, decisione già presa) e mzML solo standard.
- **D8 Perché più thread rallentano**: l'ipotesi è il memset del `reset` per array (≈ 33 000 array su LAB-B) che satura la banda della memoria, più il produttore seriale. Si chiude con RM-0 (callgrind per thread, contatori di banda) e RM-3 (senza reset).
- **D9 SHA-1**: quello del `.raw` (provenienza) va sul percorso critico solo se il file non è già letto per intero; quello dell'uscita è seriale per definizione. RM-3 misura un thread dedicato; se T_min è dominato dall'SHA-1 su file da diversi GB, si dichiara il limite.
- **D10 Attivazione**: mappatura dal filtro (`@cid`, `@hcd`, `@etd`, `@ethcd`, `@uvpd`, `@sa`) e dal trailer ai termini CV come msconvert. Oggi `@cid` diventa HCD: 16 038 MS2 su 16 038 di LAB-B (msconvert: MS:1000133). Prova F5 su tutto il corpus (RM-1).
- **D11 Costo della CLI**: `convert` impiega 5,97 s contro 2,87 s dello scrittore nel banco, e il livello 0 cambia solo 0,23 s. Va scomposto in RM-0 (SHA-256 per la cache, indicizzazione, copia/collegamento, chiavi `conv/1`).

## Decisioni chieste al Coordinatore (con raccomandazione)
1. **Uscita senza perdite**:
   - **Raccomando**: un nuovo valore `--peaks tutto`. Lo spettro porta il profilo come array standard, la lista come array non standard e le colonne per picco con i termini CV standard (charge, resolution, noise, baseline, sampled noise), con `arrayLength`.
   - Il valore di base `file` resta com'è finché E6 non passa.
   - In alternativa: due mzML (profilo e centroidi). Più semplice per i lettori, ma doppio disco.
2. **Zeri**: di base i soli punti del file (niente inventato); `--set conv.zeri=rfr` per la parità con msconvert e RFR (F3). **Raccomando** questa.
3. **mzML indicizzato anche da `scrivi`** (la CLI lo è già): **raccomando** sì, nella stessa passata con gli offset e l'SHA-1 (RM-3).
4. **Sovrapposizione con T-7b**: le colonne per picco nel lettore (`vendor/opentfraw`) sono lavoro di T-7b (A3). **Raccomando**: RM-2 fa la decodifica del descrittore solo se T-7b non è aperto; altrimenti RM-2 usa ciò che T-7b espone e fa solo l'mzML. Una sola PR per volta su `scan_data.rs`.
5. **Libreria di compressione**: passare a zlib-rs (già nel `Cargo.lock` come backend di flate2) cambia i byte dei file compressi. **Raccomando** una versione `conv/1` nuova e il prima/dopo nella PR (F10).
