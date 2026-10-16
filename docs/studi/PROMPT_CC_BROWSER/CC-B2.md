# CC-B2 — Motore GFN2-xTB in Rust: carica, ALPB acqua, Fukui condensate, `.wasm` `chem`

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Parte dopo il merge di CC-B1.

---

Pacchetto CC-B2 dello studio `docs/studi/CC_MOTORE_NEL_BROWSER.md` (leggi Sintesi, §2A, §2C, §2E, §3, §4, §5, §7, §8). File d'area: `docs/agenti/reattivita.md`, `docs/agenti/memoria.md` (contratto di memoria dei `.wasm`), `docs/agenti/mzfeat-wasm.md` (come si costruisce e si carica un `.wasm` di mzFinder). Bozza con i valori xtb: `docs/bozze/teoria_cc05.md` §2.

## Obiettivo
Dato uno SMILES, il crate `mzlab-chem` calcola in Rust GFN2-xTB per il neutro, il catione e l'anione alla stessa geometria (ALPB acqua), le cariche di Mulliken, f(+), f(0), f(−), il duale e gli indici globali di base, con parità contro tblite; ripiego EHT. Uscita JSON `reatt/1`, `.wasm` `chem` e funzione nativa per l'app desktop. Nessuna pagina.

## Da fare
1. **GFN2 ripreso** da sci-form 0.15.2 (`xtb/gfn2.rs`, `gfn2_params.rs`, `d4.rs`, `d4_data.rs`, `broyden.rs`, `sto_overlap.rs`, quanto serve) in `crates/mzlab-chem/src/xtb/`. Prima prova: cariche del neutro in fase gassosa contro `cc_ref.json` (lo studio §3 ha misurato < 1e-4).
2. **Parametri**: confronta i numeri di `gfn2_params.rs` con dxtb (Apache-2.0, github.com/grimme-lab/dxtb) e con l'articolo (doi:10.1021/acs.jctc.8b01176); scrivi la fonte in testa al file. Se un numero viene solo dai file LGPL di tblite/xtb, fermati e chiedi (studio §5).
3. **Carica totale e SCC robusto**: `solve(mol3d, charge, solvent)`; numero di elettroni dalla carica; occupazioni di Fermi a 300 K (guscio aperto ristretto con occupazioni frazionarie, come xtb senza polarizzazione di spin); partenza degli ioni dalle cariche convergenti del neutro; Broyden con smorzamento più forte e ripartenza se il residuo cresce; al massimo 250 iterazioni; `converged` sempre nell'uscita. Nessuna variabile d'ambiente (la prova dello studio ne usava una).
4. **ALPB acqua** accoppiato all'SCC (Ehlert et al. 2021, doi:10.1021/acs.jctc.1c00471): raggi di Born, kernel ALPB, energia e potenziale nella matrice di Fock; parametri dell'acqua per GFN2 dalle informazioni di supporto dell'articolo o da dxtb (stessa regola del punto 2). Il modulo `solvation_alpb/` di sci-form può servire come partenza, ma non è accoppiato al suo GFN2.
5. **Fukui**: f(−) = q(N−1) − q(N), f(+) = q(N) − q(N+1), f(0) = media, duale = f(+) − f(−), per ogni atomo (anche H); media sulle classi di simmetria di `posizioni::symmetry_classes` per la tabella dei siti; pari merito se scarto < 0,005 (bozza §2.2). Su più conformeri (CC-B1): f(−) sui 3 più bassi, `f_minus_spread` = differenza massima per atomo.
6. **Indici globali di base**: HOMO, LUMO, gap (eV) del neutro. IP, EA e ω si fanno in CC-B6.
7. **Ripiego EHT** (`eht/`, ripreso da sci-form): f(−) = popolazione HOMO per atomo, con `method:"EHT"` e nota fissa; usato solo se GFN2 non converge o l'elemento manca.
8. **Uscita** `reactivity_json(smiles, opts_json) -> Result<String>`, contratto `reatt/1` dello studio §8 (descritto in `docs/agenti/reattivita.md`); errori `error.chem.*`.
9. **`.wasm`**: crate `crates/mzlab-chem-wasm` (come `crates/mzlab-feat-grafo-wasm`), voce `chem` in `tools/build_feat_wasm.sh` con tetto proprio e manifesto; `version()` = `mzlab-chem/1`; memoria secondo `docs/agenti/memoria.md`. App desktop: stessa funzione nativa dietro la rotta `feat/grafo` con `kind:"reactivity"` (`crates/mzlab-desktop/src/feat_grafo.rs`) o una rotta sorella, come preferisce `docs/agenti/rust-desktop.md`.

## Prove
- `crates/mzlab-chem/tests/parita_gfn2.rs` su `cc_ref.json` (stessa geometria del riferimento): SCC convergente per N, N+1, N−1 in 20/20; |Δq| ≤ 2e-3 e |Δf| ≤ 3e-3 per atomo, ALPB e fase gassosa (studio §7.1). Scrivi nel documento d'area il massimo per molecola.
- `tests/siti.rs`: sulle 10 molecole della bozza la classe prima di f(−) coincide con tblite 10/10 e con il sito sperimentale ≥ 9/10 (studio §3, §7.2); sulla geometria di CC-B1 la classe prima resta la stessa 10/10 (§7.3).
- Se arrivano da Federico le uscite xtb 6.7.1 (`tests/data/xtb/`): stesso controllo contro xtb; se mancano, il test stampa `SKIP` e passa.
- Tempo: nativo nel documento d'area; WASM in Chromium con un piccolo e2e o uno script Playwright (`tests_e2e/e2e_chem_tempo.py`): carbamazepina (geometria + 3 SCC ALPB) ≤ 5 s, 60 atomi ≤ 15 s (§7.4). Peso del `.wasm` dopo `wasm-opt -Oz` ≤ 1 300 000 B (§7.5).

## Atteso
Motore GFN2 in Rust con parità dimostrata contro tblite, `reatt/1`, `.wasm` `chem` misurato. Se i criteri 1-2 non passano dopo un tentativo serio: il ripiego EHT diventa il metodo pubblicato (con il suo punteggio 8/10), il codice GFN2 resta dietro `const PUBLISH_GFN2: bool = false`, PR aperta con la domanda a Federico.

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
