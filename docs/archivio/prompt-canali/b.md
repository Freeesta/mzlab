# CANALI-B · canali dei file SCIEX `.wiff`
**Ramo**: `claude/canali-wiff` da `origin/main`. **Dipende da**: CANALI-A unito. **In parallelo con**: C, D.
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «CANALI-B: ricevuto».

1. **Ruolo**: DEV Sonnet, lettore `.wiff` (`crates/mzlab-core/src/wiff/`). Priorità di Federico: lettura `.wiff` QTRAP nativa e `.wiff`→mzML; questo lavoro non deve rallentarle.
2. **Contesto**: leggi `docs/agenti/canali-dati.md` (§1-2), `docs/agenti/wiff.md`, `docs/agenti/wiff-varianti.md`. Codice: `wiff/modi.rs` (sigle, codici confermati 4, 5, 7, 11, 15), `wiff/mod.rs:46-72, 120, 212` (`Experiment`, `Record`, solo `Period0`), `wiff/run.rs:25-113` (`srm_id`, `native`), `canali.rs` di CANALI-A.
3. **Perimetro**: `crates/mzlab-core/src/wiff/*` e, in `canali.rs`, solo la funzione che assegna `sigla` e il raggruppamento IDA (scrivila in un file proprio `canali_wiff.rs` se supera 80 righe). Niente JS, niente `iteminfo.rs`. Il formato `.mzidx` non cambia in questo lavoro.
4. **Lavoro**:
   - Sigla dell'esperimento nel canale: EMS, EPI, MRM, Prec, NL, ER, MS3, Q1, Q3; Q1MI/Q3MI → famiglia `sim:`. La sigla si legge dal testo `native` (`experiment=`) e da `modi.rs`; nessuna duplicazione della tabella.
   - IDA (EMS → EPI dipendenti, precursore che cambia): un canale `dd:2:EPI` invece di un canale per precursore; EPI a precursore fisso resta `ms:`.
   - MRM: etichetta = nome della transizione del metodo se c'è (`name=` dell'id SRM), altrimenti `Q1→Q3`.
   - Periodi > 0: se un campione del repository privato li ha, leggili (ogni periodo con i suoi esperimenti); se nessun campione li ha, un test che lo dichiara e una riga in `wiff.md`.
   - Se il `native` non basta a ricostruire esperimento e periodo, scrivi in `docs/agenti/mzidx.md` la proposta della colonna facoltativa `exp` e fermati lì: la decide Federico.
5. **Accettazione**: i campioni di `dati/3200 qtrap/` (via `tools/dati_percorsi` `percorso()`, mai percorsi a mano) danno i canali attesi (MRM con nome, EMS, EPI); senza dati privati i test si saltano (tabella `NEEDS`); l'mzML scritto da `.wiff` è identico a prima (id SRM come msconvert); nessun nome di `nomi_vietati.txt` nei test.
6. **Prove**: `cargo test -p mzlab-core wiff`, `cargo test -p mzlab-core canali`, `python3 tools/verifica.py --cambiati`, `python3 tools/pre_pr.py`.
7. **Consegna**: PR aperta su `claude/canali-wiff`; a CI verde UN messaggio al Coordinatore (≤5 righe). Eventi della PR: skill `steward`.
