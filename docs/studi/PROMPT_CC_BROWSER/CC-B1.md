# CC-B1 — Crate `mzlab-chem`: geometria 3D da SMILES in Rust e dati di riferimento

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Primo pacchetto: parte subito, con il sì di Federico alla strada principale (studio, Sintesi punto 8a).

---

Pacchetto CC-B1 dello studio `docs/studi/CC_MOTORE_NEL_BROWSER.md` (leggi Sintesi, §2D, §3, §5, §6, §7 punto 3, §8). File d'area da leggere: `docs/agenti/posizioni.md` (numerazione degli atomi, `grafo::Mol`), `docs/agenti/rust.md`. Script della misura: `docs/studi/cc_browser/`.

## Obiettivo
Il crate nuovo `crates/mzlab-chem` sa trasformare uno SMILES in una geometria 3D affidabile (ETKDG + MMFF94, più conformeri), con atomi pesanti nell'ordine dello SMILES di `mzlab_feat::chem::grafo::Mol` e poi gli H. È la base di CC-B2. Nessuna pagina, nessun `.wasm` in questo pacchetto.

## Da fare
1. **Crate** `crates/mzlab-chem` nel workspace (`Cargo.toml` della radice), dipendenze: `mzlab-feat` (feature `chem`, per `grafo::Mol`), `nalgebra`. `LICENSE-sci-form`. Moduli: `mol3d.rs` (struttura `Mol3d { el: Vec<u8>, xyz: Vec<[f64;3]>, bonds: Vec<(usize,usize,u8)>, heavy_index: Vec<Option<usize>> }`, `to_xyz()`), `geom/` (codice ripreso da sci-form 0.15.2: `distgeom/`, `etkdg.rs`, `forcefield/` per MMFF94 e UFF, solo ciò che serve).
2. **Adattatore** `Mol3d::from_mol(&grafo::Mol)`: aggiunge gli H impliciti, conserva aromaticità, cariche formali e ordini di legame; NON usa il lettore SMILES di sci-form (la numerazione deve essere quella di `posiz/1`). Se l'ETKDG ripreso vuole la sua struttura di molecola, scrivi la conversione.
3. **Conformeri**: `embed(mol, seed, n) -> Vec<Conf>`; minimizzazione MMFF94 (UFF se MMFF manca di parametri, con nota); si tiene il conformero di energia minima; `n = 8` predefinito, `seed` fisso per avere risultati ripetibili. Errori `error.chem.embed`, `error.chem.element` con `{error, error_key, params}`.
4. **Strumento** `tools/cc_riferimento.py` (Python, solo strumento; parte da `docs/studi/cc_browser/riferimento_tblite.py`): per le 20 molecole di §6, geometria RDKit ETKDGv3 + MMFF94 (seme 42) e cariche tblite 0.7.0 GFN2 di N, N+1, N−1 in ALPB water e in fase gassosa; scrive `crates/mzlab-chem/tests/data/cc_ref.json` (SMILES, elementi, xyz, cariche, f(+), f(−), versioni di tblite e RDKit, comando).
   L'ordine degli atomi deve essere lo stesso di `Mol3d::from_mol` (controllalo nello strumento con lo SMILES rigenerato da `grafo`, oppure scrivi gli SMILES in modo che RDKit e `grafo` concordino). Nel container: `python3 -m venv` in scratchpad e `pip install tblite rdkit numpy`.
5. **Documento d'area** `docs/agenti/reattivita.md` (nuovo): scopo, crate, numerazione degli atomi, geometria, riferimento; riga in AGENTS.md §5 («motore di reattività del genitore (`crates/mzlab-chem`, `.wasm` `chem`, `reatt/1`, viewer 3D di Tabui) | `docs/agenti/reattivita.md`»).

## Prove
- `crates/mzlab-chem/tests/geom.rs`: per le 20 molecole di `cc_ref.json`, embedding riuscito, nessuna distanza tra atomi non legati < 0,9 Å, lunghezze dei legami entro 0,1 Å da quelle di RDKit; RMSD degli atomi pesanti dopo allineamento (Kabsch) contro la geometria RDKit: mediano ≤ 0,3 Å (studio §7.3), scrivi tutti i valori nel documento d'area.
- Stesso seme = stessa geometria (due chiamate); ordine degli atomi pesanti uguale a quello di `grafo::Mol` (test su paracetamolo e carbamazepina).
- Tempo nativo per molecola nel documento d'area (atteso ≤ 50 ms per 30 atomi con 8 conformeri).

## Atteso
Crate con geometria 3D provata contro RDKit, dati di riferimento tblite nel repository, documento d'area e riga di AGENTS.md. Se il criterio di RMSD non passa: prova COSMolKit o chematic (studio §2D) solo come confronto e PR aperta con la domanda.

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
