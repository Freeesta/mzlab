# DEV-1 — binaryen 132 al posto di `wasm-opt@1.4.0` (binaryen 112)

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

**Ruolo.** Sei DEV-1 WASM-BINARYEN (Sonnet). Aggiorni lo strumento che ottimizza i `.wasm`; il codice Rust non cambia.

**Contesto.** `tools/wasm_opt.sh` installa da npm `wasm-opt@1.4.0`, cioè binaryen version_112 del 2023. Lo studio `docs/agenti/wasm-potenziale.md` §3 ha misurato binaryen version_132 (tarball ufficiale GitHub `binaryen-version_132-x86_64-linux.tar.gz`): stesso peso (−0,1 %), stessa velocità del codice, `wasm-opt` 8× più veloce (1,5 contro 12,8 s su un modulo da 450 KB). Leggi `docs/agenti/wasm-tetti.md`, `docs/agenti/ci-veloce.md` (installazione una volta per `$RUNNER_TEMP` con `flock`), `docs/agenti/lettori-modulo.md` (opzioni in più dei lettori).

**Perimetro.** Solo `tools/wasm_opt.sh` e le righe che lo descrivono in `docs/agenti/rust.md`, `docs/agenti/ci-veloce.md`, `docs/agenti/wasm-tetti.md` (versione dello strumento). Non toccare i workflow (lo script si installa da sé), i tetti, `Cargo.toml`, `.cargo/`.

**Lavoro.**
1. Installazione: scarica il tarball `version_132` da `https://github.com/WebAssembly/binaryen/releases/download/version_132/`, controlla lo SHA-256 fissato nello script (calcolalo una volta e scrivilo; se il controllo fallisce lo script fallisce), estrai sotto `$RUNNER_TEMP/binaryen-132` con lo stesso `flock` di oggi. Su macOS (agente locale) usa il tarball `arm64-macos`/`x86_64-macos` con il suo SHA-256; se nessun tarball va bene, ripiego attuale (npm) con un `::warning::`.
2. Opzioni: tieni `-Oz` e le quattro `--enable-*` di oggi; aggiungi `--enable-reference-types --enable-multivalue` (Rust 1.97 le emette di suo: `rustc --print cfg --target wasm32-unknown-unknown`) e `--enable-simd` (non cambia nulla nei moduli senza SIMD; serve a DEV-3). Verifica che le opzioni in più dei lettori (`tools/build_lettori_wasm.sh`) esistano ancora in binaryen 132.
3. Misura: costruisci il sito (`tools/build_feat_wasm.sh` + motore e librerie come in `pages.yml`) prima e dopo e metti in PR la tabella byte per byte di ogni `.wasm` (manifest) e il tempo totale di `wasm-opt`. Nessun modulo deve crescere oltre il suo tetto; se uno cresce più dello 0,5 % spiegalo.
4. Aggiorna le righe di documento che nominano `wasm-opt@1.4.0` / «wasm-opt 1.4.0» (riscritte, niente «prima era»).

**Accettazione.** CI della PR verde (job `sito-rust` compresi); ogni `.wasm` dentro il suo tetto; tabella prima/dopo in PR; `tools/pre_pr.py` verde.

**Prove.** `RUNNER_TEMP=$(mktemp -d) tools/wasm_opt.sh <un .wasm> 99999999 prova` (atteso: «optimised», versione 132 nel log); `sito-rust.yml` con `workflow_dispatch` `solo=e2e_lr,e2e13`.

**Consegna.** Ramo `claude/wasm-binaryen`, PR aperta; messaggio al Coordinatore (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta).
