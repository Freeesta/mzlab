# CANALI-F · tabella «1. Data files»: una riga per file, tipo come etichetta
**Ramo**: `claude/canali-tabella` da `origin/main`. **Dipende da**: CANALI-D unito e il menu del tipo di VISORE-ZOOM unito. **In parallelo con**: E.
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «CANALI-F: ricevuto».

1. **Ruolo**: DEV Sonnet, tabella di carico e tipo di esperimento.
2. **Contesto**: leggi `docs/agenti/canali-dati.md` (§3-5), `docs/agenti/dati-carico.md`, `docs/agenti/campioni.md` (sessione). Codice: `shell.js:71-139` (`GORD`, `expCell`, `nameWarn`, `usable`), `esperimenti.js` (`ESP`), `explore.js:158-169` (`KIND`, `tabFiles`, gruppi HR), `iteminfo.rs:92-160` (`file_parts`), il menu del tipo di VISORE-ZOOM, `canali.rs` (`Tipo`).
3. **Perimetro**: `shell.js`, `esperimenti.js`, `explore.js` (solo `KIND`/`kindOf`/`tabFiles`), in Rust solo `Tipo.utente` in `canali.rs` e il suo salvataggio nella sessione; e2e della tabella; chiavi in `lang/it/canali.js`/`lang/en/canali.js`. Non riscrivere il menu del tipo di VISORE-ZOOM: lo riusa e ne legge il valore. Il nodo Raw files del Flow (ARCH-G FLOW-UX, DEV-C) mostra lo stesso tipo leggendo `api/canali`: nessuna classificazione nel Flow; avvisa il Coordinatore se quel nodo calcola il tipo per conto suo.
4. **Lavoro**:
   - Una riga per file: le parti di `file_parts` diventano canali (DDA, MRM+EPI, positivo+negativo = una riga). `file_parts` resta solo per aprire sessioni salvate (migrazione) e per le schede `full/ms2/mrm`, che restano.
   - Nomi visibili del Tipo (uguali in it e en): `lc-full` «LC full scan», `lc-ms2-mirato` «LC MS²», `lc-dda` «LC DDA», `lc-mrm` «LC MRM», `infusione-msn` «MSn infusion», `misto` «mixed», `solo-uv` «UV only»; gli id restano interni.
   - Colonna «Tipo» = `tipo.utente ?? tipo.proposto`, modificabile con il menu esistente; il valore scelto cambia solo i default (canale iniziale, lente proposta), mai quali strumenti esistono.
   - Colonna «Canali»: numero; il `title` elenca i canali (cosa contiene, nient'altro).
   - `ESP` diventa una tabella di default per tipo: niente `spettri:false` che spegne strumenti; `usable` vale per ogni file con almeno un canale.
   - **Regola dei testi di Federico**: pochi testi. Niente paragrafi esplicativi dove l'interfaccia si capisce da sola; tooltip (`title`/`aria-label` descrittivi) SOLO per dire cosa fa un campo o cosa inserirvi, mai opinioni, avvisi o cautele. Un testo che non dice cosa fa o cosa inserire non si scrive (e non ha chiave i18n). Ogni testo nuovo è una chiave in `lang/it/` e `lang/en/` (`docs/agenti/lingue.md`).
5. **Accettazione**: un DDA = una riga; un MRM+EPI = una riga con due famiglie di canali; cambiare il tipo cambia solo il canale di partenza; le sessioni salvate prima si aprono uguali; avviso nome≠tipo invariato; `e2e_tour.py` valido; `controlla_i18n.py` verde.
6. **Prove**: e2e della tabella di carico, `python3 tools/verifica.py --cambiati`, `cargo test -p mzlab-core canali`, `python3 tools/pre_pr.py`.
7. **Consegna**: PR aperta su `claude/canali-tabella`; a CI verde UN messaggio al Coordinatore (≤5 righe, con quello che resta da spegnere: schede per kind, `file_parts`). Eventi della PR: skill `steward`.
