# Memoria elastica del motore nel browser (studio)

Problema: 12 file `.raw` Orbitrap (≈4,2 milioni di picchi e ≈7950 scansioni ciascuno) fermano il Dig di Tabui in «Alignment and filters» con `error.memoryBudget {fase: align, stima_mb: 188, budget_mb: 192, eccesso_mb: 1}`. Questo studio dice dove va la memoria, perché la stima è 188 MB, come rendere il budget adattivo e in che ordine lavorare. Nessun dato reale: numeri dal codice, dalle guardie, da conti e dalle misure aggregate della chat locale.

## 0. Sintesi
- Il rifiuto non è colpa del tetto: il Dig usa il modo «famiglia» (`Recipe::default`) e in `slabs::align_and_fill_stored` la famiglia è SEMPRE in memoria (lastre solo per «ione»). La famiglia a lastre (M1) fa entrare la serie a 192 MB, stessi byte.
- Il pool delle feature non riceve il budget adattivo (`opts.budgetMb || 0`, nessun chiamante lo imposta): ogni passo gira a 192 MB anche dove ce n'è di più.
- Causa a monte (misurata): 141 000-144 000 feature per file (1,72 M righe) perché la soglia fissa 2e5 sta sotto il rumore di questi dati; con 5e5 ≈50 000 per file e align passa, con 1e6 ≈14 700. La soglia stimata (M7, chat locale) riduce il bisogno di ≈10 volte: vale più di qualunque tetto.
- Il messaggio inganna: `eccesso_mb` è calcolato contro il tetto e non contro lo spazio libero (≈178 MB): «stima 188, budget 192» sembra entrare (M2).
- Dopo M1 si alza il tetto (4 GiB) con un budget a livelli scelto in Rust dai fatti del dispositivo, una prova di crescita all'avvio, un solo worker «grande» alla volta, avviso prima e discesa a gradini dopo una morte.
- Destinazione: Tabui confluisce in mzFlow e non si modifica più. Il contratto vale per i passi di mzFlow sul sito (runner web, da costruire) e nell'app desktop o `mzlab-cli flow run` (nativo, budget = RAM/4): stessa stima per passo, stessa via (memoria o lastre), stessi byte.

## 1. Dove va la memoria oggi
Le guardie (`tests/test_guardie_memoria.py`, `tools/guardie_memoria.json`) restano al pavimento di 32 MB in ogni fase; solo `completo` alle prove sale a 45 154 304 B (43 MB), a 24 MB resta a 32 MB. Provano l'assenza di regressioni, non la crescita con serie vere: per quella valgono i pesi per riga delle stime.

| Fase | Peso stimato (codice) | Via oltre il budget |
|---|---|---|
| Indice `.mzidx` (per file, worker dell'indice) | tabella 640 B/scansione + indice 220 B/scansione + al massimo `budget/4` di picchi (48 MB) + 8 MB di partizioni | secchi da OPFS, stessi byte |
| Detect L1-L3 (per file, pool di `feat-worker.js`) | picchi letti a finestre dall'indice; tabella del file 91 B/riga | per file: entra |
| Align L4 «ione» | `s.all` 8 B/riga + 39 (lean) + 40 (lavoro) B/riga + tabella 21 B/cella + 28 B/riga | lastre di m/z, stessi byte |
| Align L4 «famiglia» (predefinito del Dig) | come «ione» | nessuna: in memoria o rifiuto `align` |
| Strato debole L4b (`series::finish`) | 29 B/riga di tutte le tabelle deboli insieme, rifiuto oltre metà del libero | nessuna |
| Riempimento L6, `rowmeta` | un file alla volta, tabella intera | entra se entra la tabella |
| Ipotesi L7 (`target::run_loading`) | una tabella di file alla volta | entra |
| Prove L8-L9 (`run::evidence_to`) | `righe × 2400 B` (`IDENTITY_BYTES`) + tabella | `GraphStream`: due passate, stessi byte |
| Motore Dati (`mzlab-wasm`) | cache LRU 5/6 del budget (160 MB) + lavoro 1/6 (32 MB) | espulsione LRU |

Scheda: motore Dati + fino a `min(core−1, 4)` worker delle feature + fino a 4 worker dell'indice + l'mzML di una conversione in un Blob. La memoria WASM non si restringe: solo chiudere il worker la rende.

## 2. Perché align a 12 file stima 188 MB
- Conto: nel modo famiglia `whole = righe × 79 + tabella`, con tabella = `più_larga × 1,5 × (n × 21 + 28)`. Prima si riservano `righe × 8` per `s.all`, quindi lo spazio libero è `192 MB − 8 × righe`. Il rifiuto arriva quando `whole` supera il libero.
- Con 12 file simili (più larga ≈ righe/12) `whole ≈ righe × 114 B`: 188 MB corrispondono a ≈1,7 milioni di righe, cioè ≈145 000 feature per file; il libero è ≈178 MB, da cui il rifiuto anche se 188 < 192.
- Costo teorico delle righe lean: 39 B × 1,7 M ≈ 63 MB; la stima è 3× (40 B/riga di lavoro e ≈60 MB di tabella), giustificata: `align_grouped` passa due volte su tutti i file e `link` tiene `group_of` per ogni riga.
- Inefficienza strutturale: il modo famiglia ha bisogno di tutto solo per `famalign::link`, che per ogni riga usa `group_of`, `role`, `family_id`, `charge`, `rt`. I gruppi di ioni si formano per m/z: si possono fare a lastre come nel modo ione, tenendo solo la tabella allineata e `group_of` (4 B/riga), e poi contare voti e legami leggendo un file alla volta. Peso a lastre: tabella + 4 B/riga + una lastra + un file, ≈1/4 di oggi (M1).
- Strato debole: se align passa, `finish` tiene insieme le tabelle deboli di tutti i file (29 B/riga, soglia `WEAK_RATIO` 0,3 × 2e5): con molte feature deboli è il prossimo rifiuto `align`. Si sposta fuori dalla memoria lineare e si allinea a lastre (M1, seconda parte).
- Misura nativa (12 file, ricetta predefinita, `--budget-mb 192`): stesso rifiuto del sito (188/192). Senza tetto: detect 40 s, totale 74 s, align 0,4 s per ione e 0,9 s per famiglia; per ione a 192 MB passa già a lastre (71 s). Il rifiuto è solo del modo famiglia, come previsto dal conto sopra.
- Feature di rumore: la soglia 2e5 sta sotto il rumore reale (altezze delle feature: minimo 1,3e5, mediana 4e5); mzmine ne trova ≈7600 per file. Correzione a monte: soglia stimata (M7). M1 resta necessario: serie più lunghe in modo famiglia arrivano allo stesso muro.

## 3. Contratto di memoria v2

### 3.1 Due numeri per istanza
- `sicuro_mb`: budget con cui il motore lavora senza chiedere nulla. `alto_mb`: limite che si può usare solo dopo l'avviso. Tetto di costruzione 4 GiB (`--max-memory=4294967296`), che non riserva memoria: dice solo fin dove `memory.grow` può arrivare.
- Livelli: 96, 128, 192, 512, 1024, 2048, 3072 MB. Mai sopra 3072 nel browser (margine per codice, pila e frammentazione sotto i 4 GiB di wasm32; `usize` è a 32 bit).

| Dispositivo (fatti) | sicuro | alto |
|---|---|---|
| `deviceMemory` ≤ 1 | 96 | 96 |
| `deviceMemory` 2 | 128 | 192 |
| telefono/tablet (`telefono.js`) o Safari su iOS/iPadOS | 192 | 192 |
| computer, `deviceMemory` 4 | 512 | 1024 |
| computer, `deviceMemory` 8 (= 8 GB o più), core < 8 | 1024 | 3072 |
| computer, `deviceMemory` 8, core ≥ 8 | 2048 | 3072 |
| computer, `deviceMemory` sconosciuto (Firefox, Safari) | 512 | 2048 |

- Poi `alto = min(alto, prova)` e `sicuro = min(sicuro, alto)` (`prova` = massimo della prova di crescita, 3.2); affidabilità come oggi.
- Discesa: una morte per memoria (firma del supervisore) porta `sicuro` al livello sotto quello in uso e `alto = sicuro`; la seconda morte entro un minuto porta a 192; la terza ferma con `error.memoryBudget`. La discesa vale per la sessione (`sessionStorage`), si scrive nel registro `mzlab.memBudget.log` con chi = supervisore.

### 3.2 Prova di avvio
- Cosa: un worker usa e getta (`rust-sonda.js`, modulo ES) crea `new WebAssembly.Memory({initial: 1, maximum: 65536})` e chiama `grow(4096)` (256 MB) a gradini fino a 3072 MB o al primo errore; poi il worker si chiude. Non scrive le pagine: misura il limite del browser, non la RAM libera (quella viene da `deviceMemory`).
- Perché un'istanza a parte: la memoria WASM non si restringe; una prova dentro il motore gli lascerebbe gigabyte.
- Costo: atteso < 20 ms sul computer (pagine non toccate); tetto 2 s, oltre si registra `prova: null` e vale la tabella. Si fa una volta per sessione, a pagina ferma (`requestIdleCallback`), prima della prima richiesta grande; risultato in `sessionStorage` `mzlab.memProbe` e nel registro (`inputs.probe_mb`, `inputs.probe_ms`).
- Altri fatti: `deviceMemory`, `hardwareConcurrency`, `telefono.js`, `performance.memory.jsHeapSizeLimit` se c'è (solo registro). `measureUserAgentSpecificMemory` richiede l'isolamento cross-origin, che GitHub Pages non dà.

### 3.3 Chi decide
- Rust decide (`mzlab-core::budget_auto`, esportato come oggi da `budget_choice`): riceve i fatti `{device_gb, cores, small, ios, probe_mb}` e restituisce `{sicuro_mb, alto_mb, livelli, reason, confidence, inputs}`. Anche la discesa (`budget_down(attuale)`) è in Rust. Prova Rust: tabella dei casi.
- JS raccoglie soltanto i fatti, li passa e scrive il registro; non contiene soglie in MB (oggi `rust-bridge.js` ha 192 e 96 scritti a mano: diventano campi della scelta).
- Ogni fase continua a stimare prima di allocare: il budget più grande cambia solo la via (memoria o lastre), mai i byte del risultato.

### 3.4 Più worker, una scheda
- Un solo «permesso grande» per scheda e per origine: un worker usa più di 192 MB solo dopo aver preso il Web Lock `mzlab-mem-grande` (esclusivo, vale anche fra schede); gli altri lavorano a 192. Lo prendono i passi lunghi di mzFlow (`features.align`, `features.gapfill`, prove) e, in seguito, il motore Dati per una mappa grande.
- Chi ha usato il permesso chiude il proprio worker a fine passo se la sua memoria supera 512 MB (la memoria non torna indietro); l'orchestratore lo ricrea alla richiesta successiva.
- Motore Dati: la cache LRU resta a `min(5/6 × budget, 512 MB)`; il resto del budget è lavoro. Il riciclo di `rust-bridge.js` scatta quando `mem` supera `sicuro_mb` (non più 192 fisso) e il motore è fermo da 30 s.
- Somma della scheda: al massimo `alto` + 192 × (worker attivi − 1) + heap JS; i pool restano a `poolSize` di oggi.

### 3.5 OPFS e heap JS come appoggio
- I dati intermedi grandi che una fase rilegge per intero (tabelle lean per lastra, tabelle deboli) non stanno nella memoria lineare: passano per un tratto `Spill` (scrivi un blocco, rileggi un intervallo di m/z) che nel worker è prima un array JS (fuori dai 256 MB, M1) e poi un file OPFS con `createSyncAccessHandle` (M5).
- Già in OPFS: sorgenti, `.mzidx`, scratch dell'indice, `.mzfeatf`. Ma il pool delle feature copia tutti i `.mzfeatf` nell'heap del worker e `Stored::bytes(k)` copia un file intero nella memoria WASM a ogni lastra: M5 li legge a intervalli dall'OPFS.
- Non conviene lo spill della tabella allineata (letta a caso da `finish` e `rowmeta`) né della cache del motore (già rifatta dall'indice).

### 3.6 Browser
- Chromium desktop e Firefox: 4 GiB per istanza; `deviceMemory` solo in Chromium (massimo 8). Chromium Android: `memory.grow` fallisce spesso fra 1 e 2 GB (la prova lo misura; comunque «telefono»).
- Safari macOS e iOS: da provare che un modulo con massimo 4 GiB si istanzi (`.cargo/config.toml` cita 256 MB come limite di iPadOS). Prova in M2 sull'iPad di Federico; se fallisce, ripiego `--import-memory` (la pagina crea la `WebAssembly.Memory` col massimo scelto, stesso `.wasm`). iOS chiude le schede oltre ≈1-1,5 GB: resta 192.

## 4. Esperienza d'uso
- Predefinito sicuro: nessuna domanda finché la stima entra in `sicuro`. Le lastre sono silenziose (stessi byte); si vede solo l'avanzamento.
- Stima nella tela: ogni passo di mzFlow mostra prima di eseguire «≈ X MB» e la via («in memory» o «in parts»); il pulsante Run riassume il passo più pesante. Avviso (prima di partire, una sola volta per corsa): quando la stima di un passo supera `sicuro` ma entra in `alto`. Due pulsanti, Invio = il primo: «Work in parts (slower)» e «Use more memory». Se il passo non ha lastre il primo pulsante è «Cancel». Freno: stima oltre `alto` = Run disattivato con il motivo e il consiglio. La scelta vale per la corsa.
- Testo (tela di mzFlow, in inglese come il resto della tela): «features.align needs about 1.4 GB of memory; this browser safely gives 1 GB. If the tab closes, finished steps are kept.» Pulsanti «Work in parts (slower)», «Use more memory», «Cancel».
- Dati (`lang/`, it/en): «Questa fase richiede circa 1,4 GB di memoria; il browser ne garantisce 1 GB. Se la scheda si chiude, le fasi finite restano salvate.» / «Lavora a parti (più lento)», «Usa più memoria», «Annulla».
- Dopo una morte: riga nello stato della corsa, non una finestra: «The engine ran out of memory in features.align. Finished steps are saved; it restarts with less memory.» + pulsante «Resume» (riparte dal passo, i precedenti dalla cache `cache_key`). Nella vista Dati (`lang/`): «Il motore ha finito la memoria durante l'allineamento. Le fasi finite sono salvate; riparte con meno memoria.» + «Riprendi».
- Rifiuto (anche a lastre non entra): il testo di `error.memoryBudgetFase` con i numeri corretti (`eccesso` contro il libero) più un consiglio: «Try fewer files at once.» / «Prova con meno file insieme.».
- Avanzamento a lastre: «part 3 of 9» accanto al passo nella tela di mzFlow e la fase con «parte N di M» nella barra di `progresso.js` (Dati). Mai errori muti: un passo fallito mostra sempre chiave, numeri e consiglio nella tela.

## 5. Pacchetti DEV (in serie)
Ordine: M7 (chat locale) per prima; M1, M2 e M4a in parallelo (file diversi); poi M3; poi M4b e M4c; M5 dopo M4b; M6 dopo M3.
Chi: M1 nuova DEV Rust; M2 e M3 la DEV «memoria wasm»; M4a la chat «CLI 2»; M4b una nuova DEV del runner web; M4c la chat «Flow grafico 2». Ogni PR misura i `.wasm` prima e dopo (`tools/build_feat_wasm.sh`, `limit*`) e riporta i numeri: finder ≤ 512 000 B, gruppo HR 1 118 979 su 1 212 416, table 447 688 su 512 000, libro 152 008 su 159 744, motore principale ≈231 B di margine (un aumento sopra il tetto chiede l'assenso di Federico nella PR, mai un tetto alzato in silenzio).

### 5.1 M1 · Align «famiglia» e strato debole a lastre (nuova DEV, Rust)
- Obiettivo: il modo famiglia (`famalign::align_families`) e lo strato debole (`series::finish`) seguono la regola di `slabs.rs`: in memoria se entra in metà del libero, altrimenti a lastre di m/z, con gli stessi byte del `.mzfeat`.
- Perimetro: `crates/mzlab-feat/src/{slabs,famalign,align,series,store_read}.rs`, `crates/mzlab-feat/tests/slabs.rs`, il callback `Spill` come argomento facoltativo della funzione WASM (assente = `Vec` in memoria, così `feat-worker.js` non cambia; lo passa il runner web di M4b), `docs/agenti/memoria.md` (riga «slabs» e «Strato debole») e `docs/agenti/mzfeat.md` se cita le vie.
- Disegno: prima passata `align_grouped` senza mappa a lastre (cuts di oggi) → tabella intera → `RtMap`; seconda passata con mappa a lastre, `group_of[k]` (u32 per riga, indice globale del file = partizione + locale); `collect` legge un file alla volta solo `role`, `family_id`, `charge`, `rt` (nuovo `Want`); poi `link` e `rows` come oggi.
- Strato debole: ogni tabella debole va in un `Spill` (tratto nuovo: `put(k, bytes)`, `get(k, range_mz)`); in WASM è un callback verso array JS del worker; poi allineamento a lastre come il modo ione. Nativo e test: `Vec<Vec<u8>>`.
- Vincoli: nessun file della DEV «memoria wasm» (`mzlab-core/src/budget*.rs`, `cache.rs`, `.cargo/config.toml`, `rust-*.js`); stima prima di allocare; niente soglie cambiate; tetto di `finder`/`table` misurato.
- Prove: `cargo test -p mzlab-feat --test slabs` con casi famiglia e debole a 4/8/24 MB e in memoria: stesso SHA-256; `python3 -m pytest tests/test_feat_wasm.py tests/test_guardie_memoria.py -q`; `python3 tools/verifica.py --cambiati`.
- Accettazione: (a) serie sintetica in modo famiglia che oggi a 24 MB è rifiutata in `align` completa a 24 MB con lo stesso SHA-256 del `.mzfeat` in memoria (nativo e Node); (b) guardie entro +10 % (se cambiano, `MZLAB_GUARDIE_CREA=1` motivato); (c) conto nella PR: peso stimato famiglia per 1,7 M righe e 12 file, prima ≈188 MB, dopo (atteso ≤ 60 MB).
- Consegna: PR `claude/align-famiglia-lastre`, messaggio al Coordinatore con SHA, CI, pages e i numeri di (c).

### 5.2 M2 · Tetto 4 GiB e budget a livelli in Rust (riprende la DEV «memoria wasm», session_019kBc5HdjLDihEQX3unwZHq)
- Obiettivo: `.cargo/config.toml` a `--max-memory=4294967296`; `budget_auto` restituisce `sicuro_mb`, `alto_mb`, livelli e `budget_down` (tabella 3.1); `refusal` calcola `eccesso_mb` contro lo spazio libero e aggiunge `libero_mb`.
- Perimetro: `.cargo/config.toml`, `crates/mzlab-core/src/{budget,budget_auto,stima,lettori,mzidx}.rs`, `crates/mzlab-wasm/src/{cache,budget_js}.rs` (cache = `min(5/6, 512 MB)`), `crates/mzlab-desktop/src/ram.rs` solo se cambia la firma; i suoi file già toccati.
- Vincoli: aritmetica delle stime con `checked_mul`/`saturating_mul` (usize a 32 bit: oltre 2 GiB `n × size` trabocca), ogni `Vec` grande con `try_reserve`; i byte dei risultati non cambiano; il motore principale ha ≈231 B di margine: la tabella dei livelli come dati costanti, misura prima/dopo.
- Prove: `cargo test -p mzlab-core budget`, tabella dei casi 3.1 (incluse le combinazioni con `probe_mb`), test di trabocco (stima con 600 M righe → rifiuto ordinato, non panic); `tests/test_guardie_memoria.py`; build dei `.wasm` con `limit*`.
- Accettazione: Chromium del container istanzia ogni `.wasm` con massimo 4 GiB; parità dei byte a 24 MB e in memoria invariata (`test_feat_wasm.py`); ogni rifiuto porta `libero_mb`.
- Prova Safari: Federico (tramite il Coordinatore) apre la build della PR su iPad e Safari macOS; se il modulo non si istanzia, prova `--import-memory` (3.6).
- Consegna: PR, SHA, misure dei `.wasm` prima/dopo, esito Safari.

### 5.3 M3 · Prova di avvio, fatti e supervisore (DEV «memoria wasm», dopo M2)
- Obiettivo: `rust-sonda.js` (modulo ES, `// @ts-check`) come in 3.2; `rust-budget.js` raccoglie i fatti e chiama `budget_choice`; `rust-supervisor.js` e `rust-bridge.js` usano `budget_down` e `sicuro_mb` al posto di 96 e 192 scritti a mano; registro con la prova.
- Perimetro: `mzlab/web/{rust-sonda,rust-budget,rust-supervisor,rust-bridge,rust-worker}.js`, `tools/tipi/` (progetto e 0 errori), `index.html` o `carica.js` per il caricamento, `lang/it.js`/`lang/en.js` per il messaggio dopo la morte (4), `docs/agenti/memoria.md` (righe del budget adattivo, supervisore, riciclo).
- Vincoli: JS senza numeri di soglia; prova fuori dal motore; nessun avviso se la prova fallisce (vale la tabella); i test e2e vogliono `MZLAB_LR_MISS` vuoto.
- Prove: e2e nuovo `tests_e2e/e2e_memoria_sonda.py` (registro con `probe_mb`, tempo < 200 ms in CI); e2e del supervisore con morte simulata due volte: livelli 1024 → 512 → 192, fasi salvate; `python3 tools/controlla_i18n.py`; `tsc` del progetto; `python3 tools/verifica.py --cambiati`.
- Accettazione: in Chromium del container la scelta automatica dà `sicuro`/`alto` coerenti con la tabella (`deviceMemory` forzato con `addInitScript` a 2, 4, 8); dopo una morte simulata il registro ha la voce `supervisor` e il livello sceso.
- Consegna: PR, SHA, CI, pages.

### 5.4 M4 · mzFlow con il contratto di memoria (sostituisce il lavoro su Tabui, che non si tocca)
Tre pacchetti senza file in comune. M4a parte subito (usa `align_and_fill_stored` e il `budget_auto` nativo di oggi; la stima della famiglia a lastre arriva con M1); M4b e M4c dopo M1, M2 e M3; la prova (d) dopo M7.

#### M4a · Runner nativo e stima per passo (chat «CLI 2», session_018kSawbhNiD1N4vwTZbp14Q)
- Obiettivo: `features.align`/`features.gapfill` di `flow run` passano per `slabs::align_and_fill_stored` con un `MemBudget` (oggi `run::align` tiene tutte le tabelle senza budget): nativo = `budget_auto` (RAM/4, 1-16 GB), `--budget-mb N` per le prove. Una funzione Rust `flow::stima_passo(job, ingressi) -> {mb, via}` per `features.detect`, `features.align`, `features.gapfill` e prove (dal sondaggio dei `.mzfeatf` e dalle scansioni del `.mzidx`), usata da CLI, desktop e web; `prov/1` e `corsa/1` riportano `mem_mb` e `via` per passo.
- Perimetro: `crates/mzlab-cli/src/{flow_base,flow,flow_run,flow_out,flow_report}.rs` e i loro test, `crates/mzlab-core/src/flow/{corsa,prov}.rs` (campi nuovi; schema `corsa-1.json` aggiornato), funzione di stima in `crates/mzlab-feat/src/slabs.rs` solo come `pub fn stima(...)` sul sondaggio (coordinarsi con M1: la scrive M1, M4a la chiama), `docs/agenti/cli.md`, `docs/agenti/rust-flow.md`.
- Ricetta: nessuna soglia nei passi; tutto viene da `Recipe::for_index` (ripreso da M7), così sito, CLI e Flow hanno la stessa ricetta. Prova: un `.mzfeatf` di `flow run` e uno di `mzlab-cli features` sulla stessa serie hanno lo stesso SHA-256.
- Prove: `cargo test -p mzlab-cli --lib flow`, `cargo test -p mzlab-core --lib flow::corsa`, `tests/test_corsa_schema.py`; `flow run --budget-mb 24` e senza budget sulla serie sintetica `completo` in modo famiglia: stesso SHA-256 del `.mzfeat`.
- Accettazione (d, nativo): sui 12 file Orbitrap (prova della chat locale, dati privati) `flow run` con il piano equivalente al Dig e `--budget-mb 192` finisce senza rifiuto, tempo ≤ 1,5 × la corsa senza budget (oggi 74 s); in CI la serie sintetica a 24 MB.

#### M4b · Runner web dei passi delle feature (nuova DEV, dopo M3)
- Obiettivo: la terza voce di `F.runners` («This browser»), come in `docs/studi/workflow/WEB_RUNNER.md` ma limitata ai passi `features.detect`, `features.align`, `features.gapfill` e prove: orchestratore JS che esegue i livelli del piano, chiede la stima (`flow::stima_passo` esportata dal `.wasm` delle feature), applica il budget (`sicuro`/`alto` di M3), prende il Web Lock `mzlab-mem-grande` per i passi grandi, chiude il worker oltre 512 MB a fine passo, salva ogni uscita in OPFS con `cache_key` (passi finiti = ripresa dopo una morte).
- Perimetro: file nuovi `flow/lazy/flow-web-run.js` (orchestratore, `// @ts-check`, progetto in `tools/tipi/`) e `flow/lazy/flow-web-worker.js` se serve; usa `TP_Mine/files/feat-worker.js` solo dalla sua API pubblica (`budgetMb` nelle opzioni), senza modificarlo; esportazione della stima in `crates/mzlab-feat-wasm/src/lib.rs`; `docs/agenti/flow-grafica.md` (riga «Esecutore web»), `docs/agenti/mzfeat-wasm.md`.
- Vincoli: nessun file di Tabui (`TP_Mine/js`, `TP_Mine/lazy`) né della tela (lo fa M4c: M4b espone `F.runners` e gli eventi `{passo, stato, mb, via, parte, di}`); `.wasm` del gruppo HR e `table` misurati; `MZLAB_LR_MISS` vuoto.
- Prove: e2e `tests_e2e/e2e_flow_web.py`: piano detect → align → gapfill su 3 file sintetici, `.mzfeat` con lo stesso SHA-256 di `flow run`; serie con budget forzato 24 MB rifiutata in un passo senza lastre e completata a 64 MB (scala del caso 192 → 512), stessi byte; morte simulata del worker a metà align: ripresa dal passo, detect non rifatto.
- Accettazione (d, web): la serie sintetica a 12 file (generatore `tools/feat_mirato.py`, ≈140 000 righe per file come la misura, soglia fissa) gira a 192 MB senza rifiuto; notte (`notte.yml`) la stessa a scala vera.

#### M4c · Tela: stima, avviso, freno, stato (chat «Flow grafico 2», session_01T46TDPXiMnP8hNAn6QwuAz, dopo M4b)
- Obiettivo: la sezione 4 nella tela di mzFlow: «≈ X MB · in memory/in parts» su ogni passo (dagli eventi di M4b o, per il desktop, dalla rotta `flow/run`), avviso a due pulsanti prima di Run, Run disattivato oltre `alto` con il motivo, «part N of M», riga dopo una morte con «Resume», nessun errore muto (chiave, numeri, consiglio).
- Perimetro: `flow/lazy/{flow-canvas,flow-shell,flow-desktop,flow-run-end,flow-issues}.js` e i test della tela (`tests_e2e/e2e_flow*.py`, `e2e_tour.py` se cambia un bersaglio); `docs/agenti/flow-grafica.md`. Non tocca `flow-web-run.js` (M4b) né Rust.
- Vincoli: testi in inglese come il resto della tela; Invio = predefinito sicuro; accessibilità (tastiera, `aria-label`, colori da variabili); il tour resta valido.
- Prove: e2e con budget forzato (`?mem=24`): avviso, scelta «Work in parts» e «Use more memory» danno lo stesso `.mzfeat`; Run disattivato oltre `alto`; `e2e_axe_grafici` invariato.

### 5.5 M5 · Appoggio in OPFS per la serie (dopo M4)
- Obiettivo: `Stored` e `Spill` del runner web (M4b) leggono e scrivono file OPFS con `createSyncAccessHandle` a intervalli, invece di copiare tutti i `.mzfeatf` nell'heap del worker e un file intero nella memoria WASM a ogni lastra.
- Perimetro: `flow/lazy/flow-web-run.js` e il suo worker (dopo M4b, d'accordo con chi lo tiene), `crates/mzlab-feat/src/{slabs,store_read}.rs` (lettura per intervallo di m/z), `archivio.js` solo per i nomi in `feat/`; `docs/agenti/memoria.md`.
- Prove: stessi SHA-256 di M1/M4; misura con `tests_e2e/e2e_memoria_misure.py` (heap del worker prima/dopo); `ensureRoom` prima di scrivere oltre 50 MB.
- Accettazione: heap JS del worker durante align ≤ una lastra + un file; nessuna regressione di tempo oltre +30 % sulla serie `completo`.

### 5.6 M6 · Motore Dati con il budget nuovo (dopo M3, piccolo)
- Obiettivo: il motore Dati usa `sicuro_mb` (lavoro = budget − cache), il riciclo segue `sicuro_mb`; mappe e `subxic` invariati. Perimetro: `crates/mzlab-wasm/src/routes*.rs` e `rust-worker.js` (dopo M3, stessa DEV o una piccola).
- Prove: guardie `engine_*` invariate; `e2e_memoria_riduzioni.py`; misura della scheda prima/dopo.

### 5.7 M7 · Soglia del rumore (chat locale, fuori da questo studio)
- La chat locale porta la soglia predefinita a quella stimata dai dati (`params::noise_from_hist`) e aggiunge il ripiego «famiglia → ione» con avviso. Va messa in `Recipe::for_index`, così sito, `mzlab-cli` e i passi di mzFlow hanno la stessa ricetta (prova in M4a).
- Con M1 unito il ripiego per ione non serve più per la memoria; con M2 l'errore porta `libero_mb` e il testo del ripiego usa quei campi.
