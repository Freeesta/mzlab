# CC-B5 — «Prev. f(−)» accanto alle classi di posizioni (`posiz/1`) e lettura in Attributio

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Parte dopo il merge di CC-B2 (e di CC-B3 per la pagina); in parallelo con CC-B4 e CC-B6.

---

Pacchetto CC-B5 dello studio `docs/studi/CC_MOTORE_NEL_BROWSER.md` (leggi §4, §5 «Scienza», §8 «Con `posiz/1`»). File d'area: `docs/agenti/posizioni.md`, `docs/agenti/livelli.md` (scala «Prev.»), `docs/agenti/reattivita.md`, `docs/agenti/attributio.md` (solo come Attributio legge i contratti).

## Obiettivo
Il pannello «Substitution position» mostra, per ogni classe di posizioni compatibile con la MS2, il valore f(−) del genitore come «Prev.», senza cambiare l'ordine, gli stati o la proposta 3a/3b. Attributio può leggere lo stesso valore.

## Da fare
1. **Rust**: in `crates/mzlab-feat/src/chem/posizioni/` un ingresso facoltativo con i valori per atomo pesante (da `reatt/1`, indice `heavy_index`), che riempie il campo facoltativo `prior:[{atom, value, rank, quantity:"f_minus", method, note:"Prev."}]` di `posiz/1` (resta `version:1`); `rank` tra i siti della stessa trasformazione, pari merito se scarto < 0,005. Si calcola solo per `+O` (•OH, attacco elettrofilo); per `+Cl-H`/`+Br-H` lo stesso f(−) con la nota «electrophilic substitution, not validated here»; per le altre trasformazioni niente.
   `mzlab-feat` non dipende da `mzlab-chem`: i valori entrano come JSON già calcolato (funzione `positions` del `.wasm` `grafo` con un argomento in più, o una funzione sorella), così il `.wasm` `grafo` non cresce del motore; incrementare la costante bridge WASM da `"mzfeat-grafo/4"` a `"mzfeat-grafo/5"` in `crates/mzlab-feat-grafo-wasm/src/lib.rs` e nei documenti collegati.
2. **Pagina** (`TP_Mine/js/44_posizioni.js`): colonna «Prev. f(−)» (valore medio della classe e rango) con l'avviso fisso «Prediction (GFN2-xTB, implicit water), not evidence; does not reorder or exclude classes»; vuota con «computing…» finché CC-B4 non ha il risultato; nella tabella TDATI la colonna c'è sempre.
3. **Attributio**: nessun cambio ai pacchetti AT; documenta in `docs/agenti/reattivita.md` come il Banco 2b (quando esiste) legge `prior` di `posiz/1`.

## Prove
- `crates/mzlab-feat/tests/posizioni_prior.rs`: con i valori di `cc_ref.json` del fenolo e un TP `+O`, `prior` ha il rango atteso (para 1, orto 2, meta 3) e le classi e `proposal` sono identiche a quelle senza `prior` (stesso JSON tolto `prior`).
- `tests_e2e/e2e_posizioni.py` esteso: colonna e avviso presenti con `qqq.admin`, ordine delle righe invariato; senza `qqq.admin` niente.

## Atteso
«Prev. f(−)» visibile accanto alle classi, senza alcun effetto sulla proposta del programma.

## Regole comuni (valgono per tutta la chat)
- Sei una chat DEV di mzLab (repository `Freeesta/mzlab`). Il titolo della chat inizia con «mzLab» (per esempio «mzLab CC-B1 geometria 3D in Rust»). Il Coordinatore è la sessione che ti ha mandato questo prompt (il suo id è nel messaggio di avvio): a fine lavoro UN messaggio con `send_message` (al massimo 5 righe: PR, CI, pages, peso dei `.wasm` toccati, cosa resta). Se manca l'id, scrivi lo stesso riassunto nella descrizione della PR.
- AGENTS.md è nel tuo contesto: rispettalo. Leggi SOLO lo studio `docs/studi/CC_MOTORE_NEL_BROWSER.md` (§ indicati), il file d'area `docs/agenti/reattivita.md` (lo crea CC-B1) e i file elencati nel prompt. Per trovare codice: `python3 tools/indice.py` poi `grep -n`, oppure il subagente `cercatore`. Letture a intervalli, niente file interi grandi.
- **Il programma propone, il chimico decide**: ogni valore di reattività si chiama «Prev.» (previsione); nessuna funzione assegna un livello di Schymanski; nulla riordina, esclude o toglie le classi di posizioni di `posiz/1`.
- **Visibilità**: solo con `uipAdminOn()` (`mzlab/web/settings.js`), in Tabui (mzFinder) e Attributio; niente nella vista Dati, nel Disegno degli studenti, in Teoria o nella relazione. Un e2e controlla che senza `qqq.admin` non ci siano pannello né download del `.wasm` `chem`.
- **Rust per ogni calcolo** (`crates/mzlab-chem`); JS solo per l'interfaccia; Python solo per strumenti e test (`tools/`), mai nel sito. Codice e commenti in inglese. I testi di Tabui sono in inglese senza chiavi `lang/` (eccezione di `docs/agenti/mzfinder.md`).
- **Codice ripreso da sci-form** (MIT, github.com/jigonzalez930209/sci-form, versione 0.15.2): solo i file che servono, copiati nel crate (mai come dipendenza crates.io), ognuno con in testa provenienza (versione e percorso d'origine) e «MIT, see LICENSE-sci-form»; `crates/mzlab-chem/LICENSE-sci-form` con il testo della licenza. Ogni pezzo ripreso entra solo con una prova di parità che lo copre. Unica dipendenza nuova ammessa: `nalgebra` (Apache-2.0, `default-features = false` se basta).
- **Tetti**: il `.wasm` `chem` ha un tetto proprio in `tools/build_feat_wasm.sh` = 1,3 × misura dopo `wasm-opt -Oz`, con la misura nel commento accanto e nel documento d'area; obiettivo ≤ 1 300 000 B (studio §7). Non toccare il tetto comune degli altri `.wasm`, `.github/workflows/` né `.claude/settings.json`.
- Non toccare: `mzlab/web/teoria/`, il motore (`crates/mzlab-core`, `crates/mzlab-wasm`), i `.wasm` diversi da quelli del prompt, i pacchetti AT/MZ di Attributio, i nomi dei dati utente (`qqq_lab`, `qqq.*`).
- **Documenti**: le righe del pacchetto vanno in `docs/agenti/reattivita.md` (≤ 25 KB, righe ≤ 600 caratteri, stato attuale senza «ora/prima» né numeri di PR); la riga di AGENTS.md §5 la aggiunge solo CC-B1. Aggiorna la riga del tuo pacchetto nella tabella di §9 dello studio (stato, misure).
- Composti di prova: solo strutture pubbliche (studio §6); MAI dati del repository privato copiati nel pubblico; nessun cognome di `nomi_vietati.txt` (`python3 -m pytest -q tests/test_esempi_anonimi.py`).
- Funzioni ≤ 80 righe (il codice ripreso si spezza se serve); test in file a sé (`crates/mzlab-chem/tests/*.rs`); rustfmt solo sui file che tocchi (mai `cargo fmt` del repository); `python3 tools/controlla.py` verde; un file JS nuovo con `// @ts-check` in `tools/tipi/pagina.json` e 0 errori; ogni grafico nuovo con la sua tabella `TDATI`, colori da variabili CSS (Okabe-Ito), tastiera e `aria-label`, bersagli ≥ 24×24 px, zoom e rotazione senza trascinamento.
- Prove: `cargo test --release -p mzlab-chem` (e dei crate toccati), `python3 tools/verifica.py --cambiati` (in cloud: `docs/agenti/ambiente-cloud.md`; prima degli e2e di mzFinder `python3 tools/build_site.py`, poi cancella `site/`), un e2e nel browser se tocchi la pagina (un front end non provato nel browser è rotto), `window.MZLAB_LR_MISS` vuoto.
- Flusso: ramo `claude/<argomento>` da `origin/main`, PR non bozza, merge con merge commit quando la CI della PR è verde, poi controlla che il workflow «pages» del commit su `main` sia riuscito. Un criterio dello studio §7 non raggiunto o una scelta tra due comportamenti: PR aperta con la spiegazione e domanda al Coordinatore, niente merge.
- Se il contesto supera 200k token a fine PR: blocco di consegna al Coordinatore (AGENTS.md §2) e fermati.
