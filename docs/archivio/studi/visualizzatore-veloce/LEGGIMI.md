# Prompt dei pacchetti del Visualizzatore veloce

Criteri, numeri da battere, prove e decisioni: `CRITERI.md` (contratto di tutti i pacchetti). Ciò che il visualizzatore chiede al blocco colonnare di T-7: `INTERFACCIA-T7.md`. Ogni prompt è autosufficiente: si copia la parte dopo «---» in una chat DEV nuova (Sonnet), una PR per pacchetto.

- [V-0](V-0.md): banco di misura riproducibile (sito e nativo) e tabella di partenza; nessuna modifica al prodotto
- [V-1](V-1.md): pulsante «XIC» nello spettro, bassa e alta risoluzione, pannello sotto, ione e tolleranza visibili e modificabili, scansione corrente, apice e area (priorità di Federico)
- [V-2](V-2.md): XIC binario (`api/xicbin`) al posto del JSON, sito e desktop, stessi numeri al bit
- [V-3](V-3.md): disegno: decimazione solo sulla finestra di zoom, bastoncini al massimo per pixel, prova «nessun apice perso», canvas 2D contro WebGL misurato
- [V-4](V-4.md): file grandi e nativo: XIC su indice di un file ≥ 2 GB, più file in parallelo nel desktop, decisione «nessun indice a secchi»
- [V-5](V-5.md): lente: anteprima dello XIC al passaggio sul picco a 60 FPS, una richiesta alla volta, cache
- [V-6](V-6.md): spettri dal blocco colonnare di T-7 (dopo T-7d)

## Ordine e dipendenze
| Pacchetto | Parte | Dopo il merge di | File che tocca (conflitti) |
|---|---|---|---|
| V-0 | subito | – | `tools/bench_visore.py`, `crates/mzlab-core/examples/visore_tempi.rs`, `docs/agenti/visore-veloce.md` (nuovo), `INDICE.md`, `misure.md` |
| V-1 | subito, insieme a V-0 | – (misura con il banco di V-0 quando è in main; prima riporta i numeri del suo e2e) | `mzlab/web/xic-spettro.js` (nuovo), `spettro.js`, `pannelli.js` (barra dello spettro), `carica.js`, `lang/{it,en}.js`, `visore-veloce.md` |
| V-2 | dopo V-1 | V-0, V-1 | `crates/mzlab-wasm`, `mzlab-core` (rotta), `rust-worker.js`, `rust-bridge.js`, `browser.js`, `explore.js` (`getXic`), `grafici.js` (una riga), `crates/mzlab-desktop` (rpc, shim) |
| V-3 | insieme a V-2 | V-0 | `grafici.js` (ciclo dei tracciati), `picchi.js` (spettro), modulo puro nuovo; con V-2 in comune solo `grafici.js` su righe diverse: `git merge origin/main` prima di unire |
| V-4 | dopo V-2 | V-2 | `crates/mzlab-core` (`idxread.rs`, `routes.rs` se serve), `crates/mzlab-desktop` (`rpc_*`) |
| V-5 | dopo V-1 e V-2 | V-1, V-2 | `xic-spettro.js` o un modulo nuovo `xic-lente.js`, `spettro.js`, `lang/` |
| V-6 | dopo T-7d | T-7d, V-2 | `crates/mzlab-core` (lettura del blocco), rotte `spectrum`/`scanbin` |

Priorità se non tutti insieme: V-1, V-0, V-2, V-5, V-3, V-4, V-6. Il motore `.wasm` cambia in V-2, V-4 (se tocca il ponte) e V-6: in serie, ognuno misura i byte prima/dopo (tetto 1 703 936 non alzabile dalla chat, `CRITERI.md` D6). Numeri e decisioni di ogni pacchetto: `docs/agenti/visore-veloce.md` (lo crea V-0; se V-1 arriva prima lo crea V-1 con la stessa struttura).
Decisioni D1-D7 di `CRITERI.md` §6: proposte dall'architetto, valgono finché il Coordinatore non dice altro; la prova d'uso (§8) è da organizzare con Federico dopo V-1 e V-5.
