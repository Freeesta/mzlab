# CANALI-E · albero dei precursori e lista scan come viste del canale
**Ramo**: `claude/canali-viste` da `origin/main`. **Dipende da**: CANALI-D unito e ALBERO-PRECURSORI-UI (session_01Ly8ApkjDoaTTpDJiPcbjag) unito. **In parallelo con**: F.
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «CANALI-E: ricevuto».

1. **Ruolo**: DEV Sonnet, pagina Dati: albero dei precursori, lista scan, filtro del Banco.
2. **Contesto**: leggi `docs/agenti/canali-dati.md` (§3), `docs/agenti/infusione-grafo.md`, `docs/agenti/msn-grafo-vista.md`, `docs/agenti/dati-barra.md`. Codice: `infusione.js` (`isInf`, `msn_albero`, `filt=msn:`), `infusione-albero.js`, `barra.js:105-106`, `banco.js:38-91, 252-272` (`groupsOf`, `setFilter`, `api/scanlist`), `canali.js` di CANALI-D.
3. **Perimetro**:
   - `infusione.js`, `infusione-albero.js`, `barra.js` (solo le condizioni delle schede), `banco.js` (solo filtro e lista scan), i loro e2e, chiavi in `lang/it/canali.js`/`lang/en/canali.js`.
   - Non toccare `canali.js` (salvo esportare una funzione), i pannelli cromatogramma (CANALI-D), la grafica dell'albero (ALBERO-PRECURSORI-UI), il pannello «file e strumento» `fil` (FILE-STRUMENTO-UI, session_01Ly8ApjSQyuqbLkkPmdEFrK).
   - `banco.js` lo tocca anche il DEV-C di ARCH-G FLOW-UX (un'esportazione per disegnare `fil|lst|alb` nel nodo Raw files): se è già unito, usa la sua funzione; altrimenti tocca solo le righe 38-91 e 252-272 e dillo nella PR.
4. **Lavoro**:
   - Scheda «alb» presente per ogni file con canali `ms:` di livello ≥2 (non più `INF.any()`); «gra» resta per un'infusione singola.
   - Clic su un nodo dell'albero = selezione del canale corrispondente nel cromatogramma e nello spettro (`msn:<nodo>` resta il canale derivato con tolleranza/CE/somma).
   - Lista scan sempre del canale selezionato (`api/scanlist?canale=`), anche in bassa risoluzione; il select `data-bf="filt"` del Banco elenca i canali di `api/canali` invece di `api/filters`.
   - DDA: l'aggregato `dd:` mostra i precursori come figli a richiesta (dall'albero), non un canale ciascuno.
   - **Regola dei testi di Federico**: pochi testi. Niente paragrafi esplicativi dove l'interfaccia si capisce da sola; tooltip (`title`/`aria-label` descrittivi) SOLO per dire cosa fa un campo o cosa inserirvi, mai opinioni, avvisi o cautele. Un testo che non dice cosa fa o cosa inserire non si scrive (e non ha chiave i18n). Ogni testo nuovo è una chiave in `lang/it/` e `lang/en/` (`docs/agenti/lingue.md`).
5. **Accettazione**: bentazone MS2 mirato e timololo mostrano l'albero; un file solo MS1 non mostra «alb»; un DDA mostra l'aggregato; e2e dell'infusione e del Banco invariati nei risultati; `tour.js` valido; `e2e_axe_grafici` verde.
6. **Prove**: e2e dell'albero, del Banco e di `e2e_canali.py`, `python3 tools/verifica.py --cambiati`, `python3 tools/pre_pr.py`.
7. **Consegna**: PR aperta su `claude/canali-viste`; a CI verde UN messaggio al Coordinatore (≤5 righe). Eventi della PR: skill `steward`.
