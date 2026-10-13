# Estendere mzLab: i lettori dei file dei produttori

Stato attuale. Codice: `crates/mzlab-core/src/lettori.rs` (contratto `CONTRATTO = 1`); i lettori già scritti (`raw.rs`, `wiff/`, `agilent/`, `mzml.rs`) si portano sul trait a uno a uno, il registro `LETTORI` ne elenca solo quelli già portati (finora `agilent-d`, `agilent/lettore.rs`).

## Il contratto
- `Lettore`: `formato`, `campioni` (almeno uno; un `.wiff` ne ha più), `indice(c)` (una `VoceScansione` per scansione: id, RT in minuti, livello, polarità, TIC, picco base, filtro, precursore, `Posizione` nel grezzo), `scansione(c, k)` (legge solo i byte di `k`), `intestazione(c, k)` (`Campo`: nome, valore, unità, CV: energia di collisione, isolamento, trailer), `cromatogrammi(c)` / `cromatogramma(c, id)`, `metodo(c)` (documento `metodo/1`).
- Pagina: `rust-lettori.js` (`lettoriOp` nel worker del motore, caso `lettori2mzml`; `api.convertLettori(nome, [{name, blob}], progresso, segnale, {campione})` dà un Blob di mzML su OPFS come `convertAgilent`). Il `.wasm` dei lettori si scarica da `wasm-feat/manifest.json` (voce `lettori`, SHA-256 controllato) solo alla prima apertura di un insieme di un produttore. Prova: `tests_e2e/e2e_lettori.py`.
- Lo scrittore mzML: `testata_mzml(c)` (testata del file mzML: strumento, sorgenti, SHA-1; `None` = testata semplice) ed `extra_mzml(spettro)` (SIM, finestra). `lettori_mzml::scrivi(lettore, c, out, progress)` è l'unico scrittore mzML sopra `Lettore`: per Agilent `.D` e `.wiff` dà gli stessi byte dei convertitori diretti (prova in `tests/lettori_mzml.rs` e `tests/wiff_lettore.rs`); il Thermo `.raw` ha ancora il suo in `vendor/opentfraw`.
- Thermo `.raw` (`rawlettore.rs`): `intestazione` di una scansione FT porta anche `thermo:noise_function` (i nodi `m/z,rumore,baseline` separati da `;`, il rumore e la baseline di un picco sono l'interpolazione), `thermo:noise` e `thermo:baseline` (mediana dei nodi); senza nodi (trappola ionica, SRM) niente. I picchi non cambiano.
- Accesso casuale: lo streaming è `scansione(c, 0..n)`. Un errore è una chiave `err.file.*`, mai un panico; ciò che il file non ha è assente dalla risposta.
- `FileSet`: i file di un insieme (`X.wiff` + `X.wiff.scan`, i membri di una cartella `.raw`/`.D`, un file solo) con nome logico e `ReadAt`; `testa(i, n)` dà i primi byte.
- Nel sito: `LettoreWasm.open(nomi, dimensioni, read(i, offset, lunghezza))` (`crates/mzlab-lettori-wasm`, un `.wasm` a parte costruito da `tools/build_feat_wasm.sh` in `wasm-feat/lettori`, circa 566 KB dopo `wasm-opt`, tetto di 600 000 B in `build_feat_wasm.sh`) apre un insieme di file dal registro; `formato`, `campioni`, `indice`, `scansione`, `intestazione`, `metodo`, `cromatogrammi`, `cromatogramma` rispondono per ogni formato;
- `LettoreWasm.mzml(c, sink, progress)` scrive il campione come mzML a pezzi di circa 64 KB (prova: `tests/test_lettore_wasm.py`).
- `Limiti { profondita, voci, byte }` (`Limiti::sito()`, `Limiti::desktop()`) passato a ogni `apri`: un lettore confronta con `voci_ok` e `profondita_ok` i numeri letti dal file prima di allocare o ricorrere.

## Aggiungere un formato
1. Un modulo che implementa `Lettore` sopra `ReadAt` (mai `read_to_end` oltre 8 MB), e una `VOCE: VoceLettore { id, contratto, estensioni, riconosci, apri }`. `riconosci` guarda i byte iniziali e i nomi dei membri, l'estensione da sola non basta (`Fiducia::Si` solo con indizi nei byte).
2. Una riga in `LETTORI`. Il riconoscimento (`lettori::riconosci`, `lettori::apri`) e le estensioni dei selettori di file (`lettori::estensioni`) vengono dal registro.
3. Prove: un file costruito nel test, il confronto con l'mzML di msconvert dello stesso file (dati privati, salta senza `MZLAB_DATI`) e, per ogni numero letto dal file, il caso ostile (ciclo, profondità, conteggio enorme).
4. Chiavi di testo `err.file.*` in `lang/it.js` e `lang/en.js`.

## Difetti dei parser corretti per costruzione
`ole.rs` accetta settori di 512 o 4096 byte e mini-settori di 64, visita ogni settore DIFAT una sola volta (al più quanti ne ha il file) e percorre la directory senza ricorsione, una voce una volta; `xmltree.rs` rifiuta oltre 256 livelli di annidamento (il metodo si legge come «assente»).
