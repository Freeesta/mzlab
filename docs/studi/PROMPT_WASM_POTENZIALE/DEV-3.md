# DEV-3 — SIMD128 in tutti i `.wasm`, un solo build

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

**Ruolo.** Sei DEV-3 WASM-SIMD (Sonnet). Accendi una target-feature e il controllo del browser; nessun algoritmo nuovo.

**Contesto.** Lo studio `docs/agenti/wasm-potenziale.md` (§3-4) misura `+simd128` +5-15 % su lettura mzML e indice, peso −1,5 % (`z`), uscite identiche bit per bit; i browser bersaglio (desktop aggiornati, decisione di Federico) lo supportano tutti: Chrome/Edge ≥ 91, Firefox ≥ 89, Safari ≥ 16.4. Un solo build, nessun ripiego (AGENTS.md §4). `docs/agenti/rust-mzidx.md` e `rust-rotte.md` oggi dicono «senza `simd128`»: vanno riscritte. Leggi `docs/agenti/wasm-tetti.md`, `docs/agenti/rust-mzidx.md`, `docs/agenti/lingue.md`. Dipende da DEV-1 (`--enable-simd` in `tools/wasm_opt.sh`): parti quando DEV-1 è in `main`.

**Perimetro.** `.cargo/config.toml` (rustflags del target wasm32); il controllo nella pagina dove oggi si compila il motore (`mzlab/web/rust-supervisor.js`, `compileOnce`) e la chiave i18n nuova in `lang/it.js` e `lang/en.js`; le righe dei documenti `rust-mzidx.md`, `rust-rotte.md`, `ottimizzazioni-tentate.md` (riga del 22/10: diventa adottata con il motivo «bersaglio desktop»), `docs/agenti/wasm-potenziale.md` (stato). Non toccare `Cargo.toml`, `tools/wasm_opt.sh`, i tetti, gli script di build.

**Lavoro.**
1. `.cargo/config.toml`: aggiungi `-C target-feature=+simd128` ai rustflags di `wasm32-unknown-unknown` (vale per tutti i moduli: motore, librerie, mzFinder, lettori). Controlla con `rustc --print cfg` e che ogni `.wasm` contenga la feature `simd128` nella sezione `target_features`.
2. Browser senza SIMD128: `WebAssembly.validate` di un modulo minimo con un'istruzione `v128` (pochi byte, in linea, niente `wasm-feature-detect`): se fallisce, la pagina mostra un errore chiaro con le versioni minime (chiave nuova, in `lang/`), non il generico `error.engineFallback`. Teoria resta senza motore e non è toccata.
3. Pesi: tabella prima/dopo di ogni `.wasm` (manifest); i tetti non salgono (atteso: quasi tutti scendono un poco). Se un modulo supera il tetto, fermati e scrivi al Coordinatore.
4. Parità: tutti i test di parità e i golden e2e devono restare identici (l'autovettorizzazione non riassocia le somme in virgola mobile). Una differenza è un errore da capire, non da accettare.
5. Tempi: con il banco di DEV-5 (se c'è) o `tools/bench.py`, una riga in `docs/agenti/misure-velocita.md`.

**Accettazione.** CI verde (4 job obbligatori + `sito-rust`); golden invariati; nessun tetto alzato; `e2e_tour` valido; chiavi i18n in entrambe le lingue (`tools/controlla_i18n.py`).

**Prove.** `cargo build --release --target wasm32-unknown-unknown -p mzlab-wasm` e `node -e` con `WebAssembly.Module.customSections(m, "target_features")` (atteso: contiene `simd128`); `python3 tools/verifica.py --cambiati`; il messaggio si prova facendo fallire apposta la validazione (parametro di prova solo nei test e2e), perché i browser bersaglio hanno tutti SIMD128.

**Consegna.** Ramo `claude/wasm-simd`, PR aperta; messaggio al Coordinatore (≤ 5 righe).
