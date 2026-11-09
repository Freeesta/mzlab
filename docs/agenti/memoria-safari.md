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
| Conversione senza `createSyncAccessHandle` (`rust-opfs.js`) | no: i pezzi restano in memoria fino al Blob | nessuno |
| Grafo MSn (`mzlab-msngrafo-wasm`) | no: nessun budget passato; un trap riavvia il motore | nessuno |
| TDATI (mappa 3D, XIC, rete, QC) e «Copia» | no: nessun tetto di righe | nessuno |
