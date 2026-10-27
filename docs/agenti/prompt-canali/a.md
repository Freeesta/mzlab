# CANALI-A · motore dei canali (Rust)
**Ramo**: `claude/canali-motore` da `origin/main`. **Dipende da**: nulla. **Prima di**: B, C, D, E, F (tutti leggono `api/canali`).
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «CANALI-A: ricevuto».

1. **Ruolo**: DEV Sonnet, motore Rust (`crates/mzlab-core`, `crates/mzlab-cli`). Logica solo in Rust; niente JS.
2. **Contesto**: leggi `docs/agenti/canali-dati.md` (§1-2, il modello è lì) e `docs/agenti/rust-rotte.md`, `docs/agenti/cli.md`. Codice: `iteminfo.rs` (`Facts`, `file_parts`), `infusione.rs`, `routes.rs:118-268` (`xic_raw`, `chrom_raw`, `filters`), `msnroute.rs:55` (`Filtro`), `idxread.rs:132` (`Scan::key`), `mzmlhead.rs:96` (`Chrom`), `fileroutes.rs` (`api/mrm`), `itemroutes.rs:144-231` (`analog`, `pda_trace`).
3. **Perimetro**:
   - nuovi `crates/mzlab-core/src/canali.rs` (≤500 righe) e `canali/tests.rs`; in `lib.rs` la riga `mod`; in `routes.rs` solo la rotta `/api/canali` e la lettura di `canale=` in `chrom_raw`, `xic_raw`, `scanlist`, `spectrum`; sottocomando `mzlab-cli canali FILE` (Regola CLI); una riga in `docs/agenti/rust-rotte.md` e in `cli.md`.
   - Non toccare `wiff/` (CANALI-B), la parte UV/analogica di `canali.rs` oltre allo scheletro delle famiglie (CANALI-C), né JS.
   - `infusione.rs` e `iteminfo.rs` li tocca anche VISORE-ZOOM (`claude/xic-infusione`): leggili soltanto; se quel ramo è unito, parti da main aggiornato.
4. **Lavoro**:
   - `Canali::da(scans, chroms, facts) -> Canali` con `Canale{id, famiglia, genitore, livello, pol, sigla, etichetta, omologo, n, rt, spettri, sorgente}` e `scan_ids: Vec<u32>` (partizione degli scan MS per canale, fette `Sorgente::Scan{da,a}`), come in §2.
   - Famiglie: `tic` (tutti gli scan MS; per un file solo SRM il TIC del file se c'è, altrimenti somma delle transizioni), `bpc`, `ms:<Scan::key>`, `dd:<livello>` quando le chiavi MSn di un livello sono > 64, `srm:<q1>><q3>[@ce]` dai cromatogrammi SRM, `sim:` dai SIC. `uv:`/`an:` come varianti vuote che CANALI-C riempie.
   - `genitore`: MS1 e `dd:` → `tic`; MSn → il canale del precursore di livello n-1 (percorso più corto), altrimenti `tic`.
   - `omologo`: livello + masse nominali del percorso + attivazione + CE intera; `srm` q1/q3 a 0,1.
   - `Tipo{proposto, motivo}` (`lc-full`, `lc-ms2-mirato`, `lc-dda`, `lc-mrm`, `infusione-msn`, `misto`, `solo-uv`) da `infusione::infusione` e dai canali; `utente` lo aggiunge CANALI-F.
   - `GET /api/canali?k` → `{"v":"canali/1","tipo":{...},"canali":[{id,fam,gen,liv,pol,sigla,et,omo,n,rt0,rt1,spettri}]}`, costruito una volta per file e tenuto nelle `Routes`.
   - `canale=<id>` su `chrom` (tic/bpc), `xic` (somma solo negli scan del canale), `scanlist`, `spectrum`: per `ms:` traduci in `Filtro::Testo`, `msn:` resta, `srm:` risponde la traccia della transizione; i parametri vecchi (`level`, `prec`, `filt`) restano identici.
5. **Accettazione**:
   - Sui file di `tools/dati_sintetici.py` (aggiungi i casi che mancano: MS2 mirato in LC, MSn a blocchi, DDA con > 64 precursori, MRM) l'elenco dei canali è quello atteso, con test di parità contro un riferimento scritto a mano nel test.
   - `chrom?canale=ms:<k>` = `chrom?filt=<k>` byte per byte; `xic?canale=` su un file con due precursori MS2 dà tracce diverse fra loro e dalla traccia per livello.
   - Memoria oltre la tabella degli scan ≤ 8 B/scan (test con 100 000 scan sintetici).
   - Nessun test esistente modificato; `flow_rule_tests.rs` verde.
6. **Prove**: `cargo test -p mzlab-core canali`, `cargo test -p mzlab-cli`, `python3 tools/pre_pr.py`.
7. **Consegna**: PR aperta su `claude/canali-motore`; a CI verde UN messaggio al Coordinatore (≤5 righe: PR, SHA di 40 caratteri, cosa resta). Eventi della PR: skill `steward`.
