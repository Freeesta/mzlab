# Memoria su Safari, iPad e iPhone
Safari chiude una scheda molto prima che il dispositivo finisca la RAM (iPad circa 1-1,5 GB, iPhone meno) e non espone `navigator.deviceMemory`. Qui: quale budget usa il sito su WebKit, quali percorsi grandi finiscono nel rifiuto calmo `error.memoryBudget` (`budget::refusal`: fase, stima, budget in MB) e quale test lo prova con un budget finto piccolo. Il contratto generale di memoria sta in `docs/agenti/memoria.md`.

## Budget su WebKit
- **Stima del dispositivo**: `rust-budget.js` riconosce iOS/iPadOS (`iPad|iPhone|iPod`, o `MacIntel` con `maxTouchPoints > 1`) e lo tratta come dispositivo piccolo: `budget_auto` dà 192 MB sicuri e 192 alti (`smallDevice`). Safari su Mac (niente `deviceMemory`) parte da 512/2048 MB, abbassati dalla sonda di crescita (`rust-sonda.js`, `probe_mb`).
- **Memoria condivisa**: `SharedArrayBuffer` e i thread del costruttore dell'indice si usano solo con `crossOriginIsolated`; senza, lo stesso codice gira in un solo worker con gli stessi byte.

## Percorsi
| Percorso | Rifiuto calmo prima di allocare | Test con budget finto |
|---|---|---|
| Indice `.mzidx` di un mzML (`build_mzidx`) | sì, `estimate::check_index` (fase `index`) con `cap_mb` della pagina | `tests/test_idxpool.py`, test di `estimate.rs` |
| Indice `.mzidx` dai file del produttore (`build_mzidx_native`) | sì, `estimate::check_reader` (fase `index`) sul numero di scansioni di `Reader::index`, prima di decodificarne una; il doppio indice (profilo e centroidi) conta due volte la parte per scansione; desktop (`idxbuild_nat.rs`) e CLI (`flow_pace.rs`) chiamano la stessa funzione col budget della macchina | `tests/test_mzidx_nativo_budget.py` (8 MB: rifiuto con fase e numeri, file vuoto; 512 MB: stessi byte di nessun budget), `reader_index_refused_in_order_with_a_tiny_budget` |
| Allineamento e riempimento dei buchi di mzFinder | sì (`error.featMemory` dal pool) | `tests/test_feat_wasm.py`, `tests/test_feat_pool_budget.py` |
| Motore che muore (trap) | il supervisore lo riavvia | `tests_e2e/e2e_rust.py` |
| Conversione senza `createSyncAccessHandle` (`rust-opfs.js`) | sì, `rawSink(dest, capMb)`: i pezzi restano in memoria fino a `capMb` (il budget del motore: `BUDGET_MB` nel worker del motore, `MZLAB_RUST.budgetMb` passato da `lettori-modulo.js`), oltre si fermano con fase `mzmlInMemory`; desktop e CLI scrivono l'mzML su disco, nessuna copia in memoria | `tests/test_archivio.py` (2 MB: rifiuto con fase e numeri, pezzi liberati; 0: nessun tetto) |
| Grafo MSn (`mzlab-msngrafo-wasm`) | sì, `msngraph::read` (fase `msnGraph`): 40 B per punto, stima estesa a tutte le scansioni del consenso dopo 8 spettri; la pagina passa `budget_mb`, desktop (`rpc_msn.rs`) e CLI (`msn-graph --budget-mb`) il budget della macchina | `tests/test_msngrafo_wasm.py` (0,0001 MB: stesso rifiuto in CLI e `.wasm`; 64 MB: stessi byte), test di `read.rs` |
| TDATI (mappa 3D, XIC, rete, QC) e «Copia» | sì: una tabella tiene le prime 200 000 righe (mostrate e copiate) con la nota `tdati.capped` (esportazione della vista o app desktop); la vista 3D tiene le 100 celle più intense mentre scorre la griglia, senza l'elenco di tutte le celle (2 M su una griglia fine) | `e2e_tabelle_dati` (tetto finto 50: nota, contatore, tabella e copia di 50 righe), `e2e_mappa3d` (le 100 celle uguali a un ordinamento completo) |
