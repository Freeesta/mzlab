# scandump/1: il dump per scansione, scritto uguale da RFR e dal nostro lettore

Oracolo RFR (RawFileReader di Thermo, solo sul Mac, mai nel repository) e nostro lettore scrivono lo stesso formato; `tools/rosina/scandump_compare.py` li confronta campo per campo. Studio: `docs/studi/LETTORE_THERMO_PERFETTO.md` §3. Uscite: `thermo_scandump` (nostro), `scandump_compare.py` (confronto e aggregato), `verita_scandump.py` (→ `verita/1` per il banco F0-F5), test `crates/mzlab-lettori/tests/scandump_thermo.rs` (dump RFR del privato in `scratchpad/rfr/<file>.scandump.jsonl.gz`, salta senza `MZLAB_DATI`).

## Regole del formato
- JSON Lines, UTF-8, gzip (`.scandump.jsonl.gz`); ogni riga è un oggetto con `type`. Chiavi in inglese, tempi in minuti, m/z in Th, intensità come le dà il file.
- **Un campo che il produttore del dump non espone si omette** (mai `null`, mai un valore inventato). Una lista vuota è un'informazione (es. `reactions: []` in una MS1).
- Ordine: `file` per prima, poi le righe `scan` per `n` crescente; le righe opzionali dopo, in qualunque ordine.

## Riga `file` (una, la prima)
`schema` (`scandump/1`), `name` (nome logico, senza percorso), `sha256`, `format_version` (versione del formato `.raw`), `instrument` (modello), `controller` (indice del controller MS), `n_scans`, `producer` (`rfr` o `mzlab`), `producer_version`, `open_ms` (apertura e indice), `read_all_ms` (lettura di tutte le scansioni), `read_one_ms` (una scansione a metà). Con `--downstream` anche `downstream: true`.

## Riga `scan` (una per scansione)
- `n` (numero di scansione del file), `rt`, `level`, `polarity` (`+` o `-`), `analyzer` (ITMS, TQMS, SQMS, TOFMS, FTMS, Sector), `mode` (`profile` o `centroid`), `filter` (stringa del filtro Thermo), `scan_type` (full, zoom, sim, srm, crm, q1, q3), `segment`, `event` (come nell'indice delle scansioni), `master` (numero della scansione da cui è scelto il precursore), `tic`, `bp_mz`, `bp_int`, `mz_low`, `mz_high`, `n_peaks`.
- `reactions[]`, una voce per reazione, in ordine: `precursor` (centro d'isolamento), `iso_width`, `iso_offset`, `activation` (CID, HCD, ETD, EThcD, UVPD, ...), `energy`, `energy_unit` (`NCE` o `eV`, omessa se il file non la dice), `multiple` (booleano), `supplemental` (energia supplementare). Vuota nelle MS1.
- `sps_precursors[]` (m/z dei precursori SPS-MS3), `faims_cv`.
- `trailer`: oggetto etichetta → valore come testo; etichette senza i due punti finali, nell'ordine del file.
- Con `--downstream` (solo nostro) l'oggetto `down`: `filter` (filtro come esce dall'indice del lettore), `mzml` (`precursors[]` letti dall'mzML scritto: `spectrum_ref`, `selected_mz`, `iso_target`, `iso_lower`, `iso_upper`, `charge`, `activation[]`, `energy{name,value}`), `rawfilter` (`spectrum_ref` del primo precursore dopo `MzmlFilter`), `mzidx` (`level`, `precursor`, `rt`: la riga della tabella del `.mzidx`), `msngraph` (`node`, `level`, `chain[]` di `mz`, `activation`, `energy`: il nodo d'ingresso di `msngrafo/1`).

## Righe opzionali
- `peaks`: `n`, `mz[]`, `int[]` per 8 scansioni scelte (prima, ultima, mediana, TIC massimo, una per livello); tutte con `--full`.
- `chromatogram`: `id`, `kind` (tic, bpc, pressure, analog, pda, ...), `unit`, `time[]`, `values[]`.
- `status`: `scan`, `rt`, `values{etichetta: testo}`, campionata (al massimo 16 istanti).
- `error`: `rt`, `message` (una per voce del registro errori).
- `sequence`: la riga di sequenza (`row_number`, `injection_vial`, `injection_volume`, `sample_weight`, `sample_volume`, `istd_amount`, `dilution_factor`, `id`, `comment`, `instrument_method`, `processing_method`, `raw_file_name`, `path`, `vial`, `user_text[]`).
  Il nostro lettore aggiunge `unknown_numbers{}` e `unknown_texts{}` per posizione (campi di cui non si conosce il significato, extra per il comparatore: servono ad accoppiarli con Sample Type, Level, calibrazione del dump RFR). `asinfo`: `preamble` (24 byte in esadecimale), `text`.
- `method`: `block` (indice del blocco di testo), `length`, `sha256`, e `text` solo con `--text`.
- `tune`: `values{etichetta: testo}` (il nostro lettore non la espone ancora: omessa; il punto del file con i valori di tune per segmento non è stato trovato senza un dump RFR).

## Soglie del confronto (F.2)
RT 1e-4 min; m/z 0,5 ppm (`--tol hr`) o 1e-3 Da (`--tol lr`); intensità 1e-5 relativo; testo uguale dopo normalizzazione (spazi, maiuscole, due punti finali delle etichette). Un campo presente solo in B (nostro) è `extra`, non un errore. Le righe che non sono scansioni (`sequence`, `asinfo`, `error`, `status`, `tune`) si confrontano chiave per chiave come il trailer (`<tipo>.<chiave>`; `status` per istante campionato).

## Comandi
```
cargo run --release -p mzlab-lettori --features thermo --example thermo_scandump -- FILE.raw OUT.scandump.jsonl.gz [--full] [--text] [--downstream]
python3 tools/rosina/scandump_compare.py RFR.jsonl.gz NOSTRO.jsonl.gz [--tol hr|lr] [--out report.json] [--aggregate DIR]
python3 tools/rosina/verita_scandump.py DUMP.jsonl.gz > FILE.verita.json
```
`--aggregate DIR` riunisce i `report.json` della cartella in una tabella campo × file (solo conteggi, nessun nome di file privato nel pubblico). Le reazioni del nostro dump sono quelle che `opentfraw` espone oggi (una per scansione); quando la catena per reazione entra nel lettore, `reactions_of` nell'example le allarga.
