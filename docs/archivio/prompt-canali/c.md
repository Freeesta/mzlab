# CANALI-C · canali UV (PDA/DAD) e analogici
**Ramo**: `claude/canali-uv` da `origin/main`. **Dipende da**: CANALI-A unito. **In parallelo con**: B, D.
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «CANALI-C: ricevuto».

1. **Ruolo**: DEV Sonnet, motore Rust per UV e tracce analogiche.
2. **Contesto**: leggi `docs/agenti/canali-dati.md` (§2), `docs/agenti/dati-3200.md`. Codice: `wiff/dad.rs`, `wiff/dad_uv.rs`, `mzml_dad.rs`, `itemroutes.rs:144-231` (`analog`, `pda_trace`, `delay`), `analog.rs`, `iteminfo.rs` (`Facts.pda`, `Facts.analog`), `canali.rs` di CANALI-A.
3. **Perimetro**: nuovo `crates/mzlab-core/src/canali_uv.rs` (chiamato da `canali.rs` con una riga) e la lettura di `canale=uv:*|an:*` nelle rotte `chrom` e `uvspec`. Non decodificare PDA nuovi (Shimadzu `.lcd`, Agilent `.D`: lavori dei lettori). Niente JS, niente `wiff/` oltre alla lettura di `DadData`.
4. **Lavoro**:
   - `uv:twc` (traccia TWC/PDA del file), `an:<i>` (pressione, flusso, UV singolo: unità e intervallo da `Facts.analog`).
   - `uv:<nm>±<bw>`: canale derivato, non elencato, che risponde la stessa traccia di `chrom?kind=pda&mz0&mz1`.
   - Spettro UV di uno scan con `canale=uv:twc&rt=`; ritardo PDA (`api/delay`) applicato come oggi.
   - Genitore dei canali UV/analogici: nessuno (radici accanto a `tic`).
5. **Accettazione**: parità byte per byte con `api/chrom?kind=pda`, `api/uvspec`, `api/analog`; un mzML DAD sintetico in `tools/dati_sintetici.py`; un file senza UV non ha canali `uv:`.
6. **Prove**: `cargo test -p mzlab-core canali_uv`, `python3 tools/pre_pr.py`.
7. **Consegna**: PR aperta su `claude/canali-uv`; a CI verde UN messaggio al Coordinatore (≤5 righe). Eventi della PR: skill `steward`.
