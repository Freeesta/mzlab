# Cicli fra i moduli di `mzlab-feat`: mappa e piano dei tagli
Grafo dei moduli di primo livello di `crates/mzlab-feat/src` (quelli di `lib.rs`, con tutte le feature accese) e piano per tagliarne il ciclo grande senza toccare il corpo di nessun algoritmo: si spostano verso il basso i tipi semplici e le piccole funzioni che creano gli archi di ritorno, i vecchi percorsi pubblici `mzlab_feat::…` restano con un `pub use`. I byte dei risultati non cambiano; la prova sono i test di parità.

## Come si misura
- Nodo = modulo di primo livello (`X.rs` più `X/**`); arco X → Y = un percorso `crate::Y` o `super::Y` (anche dentro `use crate::{…}`) in un file di X, esclusi commenti, stringhe, file e blocchi `#[cfg(test)]` e i `*_tests.rs`. Righe = righe dei file di codice del modulo, senza i test.
- Ciclo = componente fortemente connessa (Tarjan). Un `pub use` che tiene soltanto il vecchio percorso pubblico di un elemento spostato non conta come arco (nessun codice dipende dal modulo di sopra); questi `pub use` sono elencati nella PR del taglio e si tolgono alla separazione in crate, quando i chiamanti puntano alla crate nuova.
- Lo script è usa e getta (non è un tool del repository): il suo esito si incolla nel corpo di ogni PR, prima → dopo.

## Il ciclo oggi
- **51 moduli, 71 387 righe** (su 115 moduli e 108 944 righe di codice). Altri cicli: 4 moduli (`weakfile`, `slabs`, `spill`, `series`, 1 115 righe), 2 (`suspects`, `suspects_listings`). Fuori dal ciclo: `chem`, `steps`, `hypo`, `axis`, `aligned_meta`, `kinetic` e gli altri.
- Membri, righe: `attributio` 27 772 · `legacy` 15 177 · `untargeted` 5 670 · `interp` 3 473 · `evid` 1 932 · `tp` 1 725 · `family` 1 503 · `groups` 1 396 · `transitions` 746 · `network` 692 · `lists` 673 · `broad` 554 · `rtwarp` 550 · `params` 493 · `quality` 487 · `store_read` 459 · `store` 457 · `filters` 447
- (segue) `filters_blank` 445 · `centwave` 421 · `resolve` 412 · `ms2spec` 405 · `align` 389 · `trace` 385 · `sweep` 362 · `broad_gt` 334 · `gapfill` 320 · `famalign` 315 · `channels` 300 · `ms2` 285 · `rtmap` 249 · `isotopes` 242 · `norm` 234 · `export` 231 · `post` 208 · `recipe` 196 · `qc` 159 · `pipeline` 148
- (segue) `table` 140 · `aligned` 132 · `rowmeta` 130 · `broad_blank` 130 · `view` 124 · `filters_nts` 81 · `rowident` 78 · `export_ms` 74 · `lrnoise` 59 · `broad_front` 58 · `params_unit` 47 · `params_height` 45 · `report` 43.
- Mozzi: `trace` e `table` hanno 24 moduli entranti ciascuno, `aligned` 14; `legacy` 7 entranti e 9 uscenti. `table` riesporta `aligned` (`pub use crate::aligned::*`), quindi ogni arco che sale da `aligned` lega al ciclo i 24 utenti di `table`.

## Cosa compila oggi una build leggera
- `attributio` sta già dietro la feature `attributio`: `mzlab-feat-wasm --no-default-features --features untargeted` (wasm32, release) legge 305 file e 62 883 righe di `mzlab-feat`, nessuno di `attributio/` (dep-info); `flow_tab` e `flow_stat` (stesse feature di `mzlab-feat`, un'unica unità) 280 file e 57 176 righe.
- Dopo una modifica di `attributio/mod.rs` le tre build non ricompilano `mzlab-feat` (0,1-1,6 s); dopo una modifica di `legacy.rs` lo ricompilano in 40 s (4 CPU). Il peso delle build leggere è il resto del nodo: `legacy` 12 460 righe lette, `untargeted` 5 475, `steps` 4 039, `evid` 1 932, `tp` 1 725.
- Chiusura dei moduli che usano `run_tab.rs` e `run_stat.rs` (`flow_drift`, `flow_export_annot`, `flow_stat`, `ionnet`, `methods`, `plausible`, `runorder`, `transitions`): 80 moduli, 86 938 righe nel grafo con tutte le feature; dopo i tagli 1-8 sotto: 68 moduli, 46 267 righe, ancora tirate da `transitions` → `legacy` e `groups` → `legacy`.

## Archi di ritorno, per quanti moduli tengono nel ciclo
Togliendo un solo arco dal grafo di oggi, il ciclo più grande diventa di (moduli, righe):

| Arco | Elementi usati | Dopo |
|---|---|---|
| `recipe` → `pipeline` | `Steps` | 44, 69 941 |
| `pipeline` → `lrnoise` | `reject` | 47, 70 743 |
| `lrnoise` → `params` | `median`, `strongest` | 48, 70 802 |
| `legacy` → `network` | `arcs` (`best_pairs`, `counts`, `neutral_mass`) | 49, 67 222 |
| `legacy` → `lists` | `from_legacy_text` | 49, 69 968 |
| `table` → `aligned` (cioè `aligned` → `tp`, `rowident`) | `*` | 49, 70 998 |
| `network` → `interp` | `RowInterp`, `store` | 50, 67 914 |
| `untargeted` → `attributio` e `interp` → `attributio` (insieme) | `to_attributio`, `ms1_window_metrics` | 50, 43 615 |
| `sweep`, `channels` … → `trace` | `filled`, `reserve`, costanti | 50, ~71 000 ciascuno |

Togliendo gli archi uno dopo l'altro (il migliore a ogni passo): `recipe` → `pipeline` (44), `legacy` → `network` (42), `trace` → `untargeted` (40 moduli, 32 334 righe), `table` → `aligned` (22), `store` → `recipe` (11), `legacy` → `lists` (10), poi cicli locali ≤ 9.

## Piano dei tagli (una PR per taglio, ≤ ~600 righe cambiate)
Numeri = ciclo più grande dopo il taglio, cumulato nell'ordine qui sotto (simulato sul grafo; la PR misura sul codice).

1. **`attributio` fuori dal ciclo** (51 → 50 moduli, 71 387 → 43 615 righe). `git mv crates/mzlab-feat/src/untargeted/to_attributio.rs crates/mzlab-feat/src/attributio/from_untargeted.rs` (i suoi `super::ident`, `super::isotopes` diventano `crate::untargeted::…`; `#[cfg(feature = "untargeted")]`); `interp::ms2link::ms1_window_metrics` (~50 righe) va in `attributio/review/window.rs` (`#[cfg(feature = "interp")]`).
   Cambiano: `untargeted/mod.rs` e `interp/ms2link.rs` (il `pub use` del vecchio percorso), `attributio/mod.rs`, `attributio/review/mod.rs`. Chiamanti dei vecchi percorsi: `tests/untargeted_to_attributio.rs`, `mzlab-cli/src/flow_chimeric.rs` (invariati fino alla separazione).
2. **`aligned` diventa base** (→ 46, 42 750). La struttura `TpBlock` con `TpParams`, `RoleCounts`, le costanti `NO_PARENT`, `ST_*`, `NOTE_*`, `VIA_*`, `KIN_*` (da `tp/mod.rs`) e `FitCandidate` (da `tp/candidates.rs`) vanno nel modulo nuovo `tp_block.rs`, senza dipendenze; la struttura `RowIdent` e `SRC_DETECTED` vanno in `aligned_meta.rs` (in `rowident.rs` restano `impl RowIdent { compute… }`, `adduct_names`, `code_of`). Cambiano: `tp/mod.rs`, `tp/candidates.rs`, `tp_block.rs`, `aligned.rs`, `aligned_meta.rs`, `rowident.rs`, `lib.rs`.
3. **`pipeline::Steps` in basso** (→ 42, 42 008). `Steps` con il suo `Debug` e `Default` va in `pipeline_steps.rs` (dipende solo da `quality`, `isotopes`, `ms2`, `resolve`, `align_mzmine`); `pipeline.rs` lo riesporta. Cambiano: `pipeline.rs`, `pipeline_steps.rs`, `recipe.rs`, `lib.rs`.
4. **`network::arcs` in basso** (→ 39, 36 228). `git mv crates/mzlab-feat/src/network/arcs.rs crates/mzlab-feat/src/netarcs.rs` (82 righe, usa solo `hypo`); `network/mod.rs` ha `pub use crate::netarcs as arcs;`. Cambiano: `network/mod.rs`, `legacy/network.rs`, `lib.rs`.
5. **Base di `trace`** (→ 33, 34 210). `reserve`, `filled` e le costanti `SLAB_PEAKS`, `BYTES_PER_PEAK`, `FLOOR`, `MIN_HEIGHT`, `TOUCH_HEIGHT` vanno in `trace_base.rs` (solo `mzlab_core::error`); `trace.rs` le riesporta; `Params` resta in `trace` (porta `centwave::Peak`). Cambiano: `trace.rs`, `trace_base.rs`, `lib.rs` e i ~30 file che importano solo quegli elementi da `crate::trace` (`sweep`, `isotopes`, `broad*`, `post`, `rowmeta`, `align`, `famalign`, `slabs`, `tp`, `untargeted/*`, `family/roles.rs`, …).
6. **`trace` → `untargeted`** (→ 13, 23 327): la via ADAP di `trace::detect_with` chiama `untargeted::flow::detect`, che usa `untargeted::pipeline`. Strade, da scegliere leggendo `untargeted::pipeline`: la parte di `untargeted` che serve a `flow` scende in un modulo sotto `trace`, oppure la scelta della via passa a chi chiama (`pipeline`, `series`) come parametro.
7. **`store` → `recipe`** (→ 9, 22 421): `recipe::fingerprint` (funzione pura) scende in `store`/base e la funzione di `store.rs` che legge un `&Recipe` riceve i testi già pronti.
8. **`family` → `legacy`** (→ 9, 2 847): `legacy::edge::{Iimn, ISOTOPES}` usati da `family/rel.rs` scendono in un modulo base.

Dopo l'8 restano cicli locali di ≤ 9 moduli: `trace`, `centwave`, `channels`, `broad*`, `post`, `quality` (2 847 righe); `legacy`, `groups`, `lists`, `transitions` (17 992); `store`, `store_read`, `evid`, `rowident` (2 926); `weakfile`, `series`, `spill`, `slabs`; `filters*`; `params*`.

## Dopo i tagli: separazione in crate
- La crate del motore di attribuzione prende `attributio/` con `flow_regulatory`, `flow_rtpredict`, `normative` e i file di `flow_formula`; dipende dalla `mzlab-feat` snella, che non ne riesporta nulla; i ponti (`mzlab-attributio-wasm`, `mzlab-normative-wasm`, CLI, app) e i test la usano direttamente. Una crate con cicli caldi riceve la stessa riga `opt-level` di `mzlab-feat` in ogni profilo di `Cargo.toml`; dopo la separazione si misura ogni `.wasm` contro il suo tetto.
- L'ordine delle separazioni seguenti va per righe tolte alle build leggere (`untargeted`, `flowtab`, `flowstat`): il primo candidato è `legacy` (12 460 righe lette da `untargeted`), raggiungibile da `flowtab` per `transitions` → `legacy::lr` e `groups` → `legacy::lr`.
