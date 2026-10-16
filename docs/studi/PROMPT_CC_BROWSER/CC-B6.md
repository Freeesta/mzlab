# CC-B6 (facoltativo) — Indici globali: IP, EA, elettrofilicità ω, HOMO/LUMO; BDE C–H solo se validata

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Facoltativo: parte dopo il merge di CC-B2 e con il sì di Federico; in parallelo con CC-B4 e CC-B5.

---

Pacchetto CC-B6 dello studio `docs/studi/CC_MOTORE_NEL_BROWSER.md` (leggi §2E, §5, §7). File d'area: `docs/agenti/reattivita.md`.

## Obiettivo
La scheda «Info» del pannello mostra gli indici globali stimati dallo stesso motore, con l'avviso dei limiti: IP e EA verticali, potenziale chimico μ, durezza η, elettrofilicità globale ω in eV; HOMO, LUMO e gap ci sono già da CC-B2.

## Da fare
1. **Rust** (`crates/mzlab-chem/src/indici.rs`): IP = E(N−1) − E(N), EA = E(N) − E(N+1) dalle energie SCC già calcolate per le Fukui (nessun calcolo in più); correzione empirica per GFN2 come la applica xtb con `--vipea` (trova il valore nel manuale di xtb o nel codice di dxtb, con la fonte; se non lo trovi, mostra i valori senza correzione con la nota «uncorrected»). μ = −(IP+EA)/2, η = IP − EA, ω = μ²/(2η) (Parr et al. 1999, doi:10.1021/ja983494x). Campo `global` di `reatt/1`: tutti i campi indici globali `global.*_ev` (`ip_ev`, `ea_ev`, `mu_ev`, `eta_ev`, `omega_ev`) sono opzionali (`Option<f64>`, `null` se assenti); con il ripiego EHT il blocco `scc` restituisce `{ "converged": [true, false, false], "iterations": [1, 0, 0] }` o `null`.
2. **Prova di senso** (non un criterio di pubblicazione): ω per la serie N-fenilacrilammide, N-(p-metossifenil)acrilammide, N-(p-cianofenil)acrilammide deve seguire l'ordine ciano > H > metossi (ordine qualitativo atteso dagli effetti dei sostituenti); IP del benzene, del fenolo e dell'anilina contro i valori sperimentali del NIST Chemistry WebBook (con URL), scarto scritto nel documento d'area.
3. **BDE C–H** (solo se Federico la chiede dopo aver visto i punti 1-2): E(R•) + E(H•) − E(RH) GFN2 a geometria rilassata, confrontata con una tabella di BDE sperimentali con DOI (almeno 15 legami C–H); si mostra solo se il rango di Spearman ≥ 0,7, come «Prev.».
4. **Pagina**: righe della scheda «Info» con unità, metodo e la nota fissa «Estimates from GFN2-xTB energy differences; compare molecules of the same family only»; valori anche nella tabella TDATI della scheda e nel JSON.

## Prove
- `crates/mzlab-chem/tests/indici.rs`: formule (valori costruiti a mano), ordine della serie delle acrilammidi, IP nel documento d'area.
- e2e di CC-B3 esteso: righe di «Info» presenti con `qqq.admin`.

## Atteso
Indici globali con limiti scritti; BDE solo se validata.

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
