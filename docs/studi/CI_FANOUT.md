# CI più veloce: compilazioni ripetute e fan-out (misure del 23 ottobre 2026)

Solo proposta e misure: nessun workflow è stato toccato. I workflow sono competenza del Coordinatore; i diff sotto sono pronti da applicare.

## Misure

Fonte: ramo `stat-ci` (`eventi.jsonl`, run dal 21 al 23 ottobre), i log di due job di `main` (`rust-core` e `rust-det`, commit `af10f73`) e `tools/ci_metrics.py percorso --da 2026-10-22T12:00`.

**Cammino critico (push → ultimo controllo richiesto, PR unite)**: PR con Rust mediana 7,3 min (n=71, max 24,9); sito/altro 4,9 min (n=52); solo documenti 0,8 min (n=25). Nelle PR con Rust i job più lenti sono quasi sempre di `rust` (`rust-misc-b`, `rust-misc-a`, `rust-det`), con `pytest-shard solo311` (4,5 min) vicino.

**Job di `rust` sugli ultimi 10 commit di `main`** (mediana su run riuscite, secondi): `changes` 27, `rust` 224, `rust-core` 333 (passo di test 294), `rust-misc-a` 192, `rust-misc-b` 266, `rust-det` 253, `rust-wasm` 126, `rust-wasm-guardie` 191, `rust-lint` 65, `deny` 54. Su 80 run con i job Rust il job più lungo è stato `rust-misc-b` 25 volte, `rust-core` 23, `rust-det` 15, `rust-wasm` 9, `rust-misc-a` 4, `rust-wasm-guardie` 3: nessuno domina. Mediana del job più lungo 308 s.

**Dentro `rust-core`** (log di `af10f73`): il passo dura 256 s, i binari di test 109,7 s (697 test passati), quindi la compilazione è circa 146 s. In `rust-det` il passo `pytest` dura 236 s con il ripristino dalla cache `rust-Linux-wasm-…` (94 MB, nessuna cache `det` utile): compila da zero il nativo e il WASI con LTO.

**`test`** (stessi commit): `browser` 172 s, `pytest-shard` 3.11 rest 169, 3.11 solo311 183, 3.14 rest 154; il ripristino della cache dei shard `rest` pesa 1,4 GB e costa 42-44 s (25 % del job), ma i shard non sono mai il cammino critico quando c'è Rust.

**`pages` / `build`** (119 run su `main` dal 21 ottobre): mediana 499 s, da 433 a 1012 s; passo mzFinder (`build_feat_wasm.sh site/static`, seriale) 376-468 s. Con Rust cambiato la mediana è circa 760 s, con Rust invariato (53 di 119 commit, il 45 %) circa 570 s: **un commit senza Rust ricostruisce comunque tutto**. In più 78 run `pages` sono stati sostituiti da un run più nuovo (3 h 10 min di attesa in coda dal 22 ottobre): la consegna del sito è accorpata e il suo ritardo è un build (circa 8 min) più, se arrivano più unioni di fila, un secondo.

**Cache e mtime**: `rust.yml` e `test.yml` lanciano `tools/ci_cargo_mtime.py restore|record`; `pages.yml` e `sito-rust.yml` NO (`grep -c ci_cargo_mtime` = 0 e 0). Il checkout dà a ogni file l'ora del checkout, quindi in quei due workflow cargo ricompila ogni crate del workspace anche quando `crates/` non è cambiato. `sito-rust` parte anche per soli `mzlab/**`, `tests_e2e/**`, `tools/…`: la maggior parte delle PR di sito. Il cache fallback sul solo prefisso è raro (2-5 % di partenze senza cache nei job `rust`; `rust-core` 18 % da un prefisso estraneo).

**Carico dei runner** (108 campioni): in corso mediana 18, massimo 40; in coda in 18 campioni su 108 (massimo 54 job). Attesa mediana in coda 2 s, p90 5-12 s. Aggiungere job costa quindi poco di norma ma tanto nei picchi.

**Lacuna di misura**: `stat-ci` registra solo `rust`, `test` e `pages`; `sito-rust` e `desktop` non compaiono nei run (807 s di `sito-rust` sono la misura documentata, non rifatta qui). Aggiungere `sito-rust` all'elenco di `stat-ci.yml` è il primo passo per verificare la proposta 1.

## Ricompilazioni degli stessi crate

Per un commit che cambia `mzlab-core` la stessa compilazione (wasm32, profilo `release` con LTO e `codegen-units = 1`) avviene in: `rust-wasm` (110 s di cargo), `rust-wasm-guardie` (famiglia `wasm` dello shim), 3 job `sito-wasm-*` (254 + 166 + 190 s) e `pages` (499 s), cioè circa 1400 s di runner per gli stessi moduli, di cui `pages` e `sito-wasm-*` producono file con gli stessi SHA-256 (verificato in `ci-veloce.md`). Nel nativo `rust-det`, `pytest solo311` e `rust-core` compilano profili diversi (LTO e non), quindi non condividono nulla.

Condividere un artefatto fra workflow resta scartato: `pages` dipende da `sito-rust` solo con `workflow_run`, e la consegna aspetterebbe gli e2e (174 s); la build unica a monte è stata già stimata negativa. Le tre proposte sotto non spostano il lavoro, lo evitano o lo parallelizzano.

## Proposta 1 — mtime nei workflow del sito (priorità, rischio basso)

Cosa: `pages.yml` (job `build`) e i tre `sito-wasm-*` di `sito-rust.yml` usano `ci_cargo_mtime.py` come `rust.yml`: `restore` dopo il ripristino della cache, `record` prima di ogni salvataggio. Ogni cache salvata porta il suo registro; le crate invariate non si ricompilano.

Risparmio atteso: sui commit senza Rust di `main` (45 %) e sulle PR di solo sito, cargo non compila nulla; restano `wasm-bindgen` e `wasm-opt` dei moduli (tempo non misurato, stimo 100-150 s). Per `pages` da circa 570 s a circa 250-300 s (−270 s, −50 %) su quei commit; per `sito-wasm-core` da 254 a circa 150 s e per `feat-a/b` da 166/190 a circa 90-110 s sulle PR di solo sito (−60 / −100 s sul cammino critico di `sito`). Nessun minuto di runner in più, in meno.

Rischio: basso. Cambia solo l'mtime dei file invariati; un registro assente o vecchio lascia il comportamento di oggi. Una volta sola, le cache esistenti non hanno il registro: il primo run di `main` dopo la modifica ricostruisce e salva (cambio di chiave con il suffisso `-m1` per forzare il salvataggio). Controllo: i 48 file di `site/static/wasm*` devono avere gli stessi SHA-256 di una costruzione a freddo (`tools/ci_wasm_dimensioni.py` e il manifest li verificano già); la prova della proposta è confrontare i byte del primo run con registro contro l'ultimo senza.

Diff (stesso schema per `sito-wasm-core`, `sito-wasm-feat-a`, `sito-wasm-feat-b`; per `pages` la chiave diventa `pages-rust-…-m1`):

```diff
       - name: Toolchain (version pinned in rust-toolchain.toml)
         run: rustup show
       - id: rcache
         uses: actions/cache/restore@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0
         with:
           path: |
             ~/.cargo/registry
             ~/.cargo/git
             ~/.cargo/bin/wasm-bindgen
             target
-          key: sito-core-${{ runner.os }}-${{ hashFiles('Cargo.lock', 'rust-toolchain.toml') }}-${{ hashFiles('crates/**', 'vendor/**', '.cargo/**') }}
+          key: sito-core-${{ runner.os }}-${{ hashFiles('Cargo.lock', 'rust-toolchain.toml') }}-${{ hashFiles('crates/**', 'vendor/**', '.cargo/**') }}-m1
           restore-keys: |
             sito-core-${{ runner.os }}-
             sito-rust-${{ runner.os }}-
+      # cargo rebuilds a crate whose source mtime is newer than its cached output, and a checkout touches every file (tools/ci_cargo_mtime.py)
+      - id: mtime
+        name: Reuse the compiled crates of the cache
+        run: python3 tools/ci_cargo_mtime.py restore
       - name: wasm-bindgen CLI (the version of Cargo.lock)
 ...
+      - name: Record the tree of the cache
+        if: github.ref == 'refs/heads/main' && steps.rcache.outputs.cache-hit != 'true'
+        run: python3 tools/ci_cargo_mtime.py record
       # saved right after the build, so a red e2e does not lose the cache (main only)
       - uses: actions/cache/save@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0
         if: github.ref == 'refs/heads/main' && steps.rcache.outputs.cache-hit != 'true'
```

In `pages.yml` il passo `Save the Rust cache` ha già `if: steps.rcache.outputs.cache-hit != 'true'`: aggiungere prima lo stesso passo `record` con la stessa condizione. `restore` va dopo `Restore … rcache` e prima di `Build the WASM engine`.

## Proposta 2 — `pages` in tre job paralleli (dopo la 1, rischio medio)

Cosa: `pages.yml` ricalca `sito-rust.yml`: `pages-wasm-core` (motore, librerie, lettori), `pages-wasm-feat-a`, `pages-wasm-feat-b` costruiscono i `.wasm` e li passano come artefatto; `build` (con `needs` dei tre) scarica gli artefatti, esegue `tools/build_site.py`, `tools/build_feat_wasm.sh site/static manifest`, i passi delle librerie e dei sospetti (invariati) e `upload-pages-artifact`. Le tre cache Rust possono restare di sola lettura per i job paralleli leggendo i prefissi `sito-core-/sito-feat-a-/sito-feat-b-` (nessun salvataggio in più nei 10 GB) o, più semplice, avere chiavi `pages-core-/pages-feat-a-/pages-feat-b-` proprie come oggi `pages-rust-`.

Risparmio atteso sui commit con Rust (55 % delle costruzioni, mediana circa 760 s): da 500-760 s a circa 254 (`sito-wasm-core`, il più lungo) + circa 65 (checkout, `build_site.py`, scarico degli artefatti, manifest, cache delle librerie, upload) = circa 320 s, cioè −180/−440 s sul ritardo di consegna (il metro di AGENTS.md) e un tempo di coda più corto per le unioni di fila. Costo: circa +3 minuti-runner per costruzione (due setup in più, artefatti, tre toolchain) e fino a +3 job nei picchi di coda (18 % dei campioni).

Rischio: medio. (a) Equivalenza dei byte: `build_feat_wasm.sh a|b` + `manifest` e `all` danno gli stessi file (documentato in `ci-veloce.md`), ma la prova va fatta con `sha256sum site/static/wasm*` su due costruzioni. (b) La concurrency `pages-build` va spostata a livello di workflow (`concurrency: {group: pages-build, cancel-in-progress: false}`), altrimenti i tre job paralleli di due run diversi si sovrappongono. (c) Dopo la proposta 1 il guadagno sui commit senza Rust si riduce al solo pacchetto; si decide dopo aver misurato.

Schema:

```yaml
concurrency:
  group: pages-build
  cancel-in-progress: false
jobs:
  wasm-core:      # = sito-wasm-core di sito-rust.yml, con la cache pages-core-…, artefatto "wasm-core"
  wasm-feat-a:    # = sito-wasm-feat-a, artefatto "wasm-feat-a"
  wasm-feat-b:    # = sito-wasm-feat-b, artefatto "wasm-feat-b"
  build:
    needs: [wasm-core, wasm-feat-a, wasm-feat-b]
    steps:
      - checkout, setup-python
      - run: python tools/build_site.py
      - uses: actions/download-artifact   # pattern: wasm-*, merge-multiple: true, path: wasm-artifacts
      - run: |
          cp -r wasm-artifacts/. site/static/
          tools/build_feat_wasm.sh site/static manifest
      - (passi invariati: librerie, sospetti, upload-pages-artifact)
  deploy: (invariato)
```

## Proposta 3 — piccole sistemazioni di `rust.yml` (rischio basso, guadagno piccolo)

1. `changes` usa `fetch-depth: 0` (checkout 12 s di mediana, 22 s sugli ultimi commit di `main`, sul cammino critico di ogni run). Con `fetch-depth: 1` più `git fetch --depth=1 origin "$BASE"` il diff dei percorsi resta identico (usa solo `git diff --name-only BASE SHA`): −8/−15 s su ogni run di `rust`, 0 minuti-runner in più. Se `$BASE` non si scarica, il ramo esistente già imposta `rust=true core=true`.
2. Riequilibrio `rust-misc-a` (192 s) e `rust-misc-b` (266 s): spostare da `-b` ad `-a` crate per circa 35 s di compilazione e test (il passo di `-b` dura 231 s contro 161). Va scelto con i tempi per binario che `ci_binari_test.py` già stampa nei titoli dei gruppi; l'effetto sul cammino critico è circa −20 s mediani perché subito dopo vengono `rust-core` e `rust-det`. Le liste di crate stanno in `rust.yml` e in `tools/ci_binari_test.py` (argomenti `-p`), non nel codice di prodotto.

Scartato dopo la misura: dividere i binari di `rust-core` in due job (la compilazione, 146 s, si duplicherebbe: −55 s al più solo su quel job, +1 runner); ridurre il ripristino della cache dei shard `rest` (1,4 GB, 42-44 s) senza prima misurare cosa pesa dentro `target/fam/*` (non è sul cammino critico).

## Ordine consigliato

Prima aggiungere `sito-rust` a `stat-ci.yml` (nessuna decisione), poi la proposta 1 in una PR, tre giorni di misure con `ci_metrics.py percorso` e `riepilogo`, poi 2 e 3.
