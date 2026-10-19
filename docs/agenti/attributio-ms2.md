# Attributio: motore MS2, frammentazione in silico e plugin (pacchetti MS)

Studio e piano: `docs/studi/ATTRIBUTIO.md` §6 (righe MS-00..MS-04). Tutto in Rust (`crates/mzlab-feat/src/attributio/ms2_engine/`, feature `attributio`), nessun Java, nessuna rete a runtime. Le uscite sono PROVE mostrate accanto alla cascata: non fanno identità, non assegnano livelli (il 2b resta del chimico). Spento di serie (`[annotation] use_ms2_engine = false`), visibile solo con `uipAdminOn()`.

## MS-00: motore MS2
- **Compilatore** (`compile.rs`): legge librerie GNPS JSON/JSON-lines e MSP con i parser di `mzlab-lib` (`parse_all`), tiene polarità, precursore, addotto, picchi; spettri senza polarità, picchi o id sono scartati. `clean_library_name` toglie dal nome i nomi di file (`.mzML`…), il tag `CollisionEnergy…` e i suffissi di energia (` - 30.0 eV`), poi `names::clean_raw`.
- **Matcher** (`matcher.rs`): `Ms2Library::new` pulisce e ordina per precursore (una lista per polarità); `search` pulisce la query come `ms_entropy` (m/z 10-1000, float32, rumore 1 %, 0,05 Da), cerca nella finestra in ppm (10) e punteggia con l'entropia di `mzlab-lib` a 0,02 Da. Soglia predefinita 0,6, almeno 3 ioni condivisi (`min_shared`: gate nuovo, l'originale non l'aveva).
- **Uscita** (`hits.rs`): `engine_<pos|neg>.csv` (`_pol,_id,engine_name,engine_score,engine_precursor_mz,engine_id`); lettura senza scrivere nella campagna.
- **Non portato**: la ricerca MS2DeepScore (rete neurale; il punteggio non è un'identità).
- **Parità**: `cargo test -p mzlab-feat --features attributio --test attributio_ms2_engine`; dati d'oro `tests/data/attributio/ms2_engine/golden.json` da `tools/ms2_engine_riferimento.py` (richiede `ms_entropy`): stessi insiemi di risultati e punteggi entro 1e-6.

## MS-01: prove nella cascata
- `readers`: `ms2_engine::load_engine_hits` legge `<data>/engine/engine_<pos|neg>.csv` in `PolSources::engine` (file assente = nessuna riga; il conteggio «engine hits» compare solo se ci sono righe).
- `annotate/join.rs`: con `[annotation] use_ms2_engine = true` il migliore risultato di ogni feature va in `Feature::engine` (`EngineEvidence`: nome, punteggio, precursore, id di libreria); senza l'opzione i file sono ignorati. Nessun cancello, nessun livello, nessuna identità cambia (provato: livelli uguali con e senza).
- `Feature::engine_agrees()` confronta il nome del motore con identità o nome mostrato (lettere e cifre, senza maiuscole): `Some(false)` è un disaccordo da far vedere al chimico, non un conflitto che sposta il livello. Il nome entra nel conflitto solo come prova mostrata; la colonna nella tabella e nel Banco arriva con la voce di amministratore (MS-04).
- Prova: `attributio_ms2_engine::engine_hits_are_evidence_not_levels`.
