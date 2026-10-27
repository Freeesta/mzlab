# CANALI-D · selettore dei canali, cromatogramma e XIC sul canale
**Ramo**: `claude/canali-selettore` da `origin/main`. **Dipende da**: CANALI-A unito e VISORE-ZOOM (`claude/xic-infusione`, PR #1844) unito: parti da main che li contiene. **Prima di**: E, F.
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «CANALI-D: ricevuto».

1. **Ruolo**: DEV Sonnet, pagina Dati (JS solo colla; ogni calcolo è già nel motore).
2. **Contesto**: leggi `docs/agenti/canali-dati.md` (§2 API, §3 interfaccia), `docs/agenti/dati.md`, `docs/agenti/accessibilita.md`. Codice: `pannelli.js:643-689` (selettori `kind`, `prec`, `tr`), `grafici.js:53-113` (`rawSeries`, `ms2trSeries`, MRM), `tabs.js:86-156` (pannello `mrm`, `ms2Exps`), `explore.js:341-406` (`getChrom`, `getMrm`), `xic-lente.js`, `xic-spettro.js`, `esperimenti.js`.
3. **Perimetro**:
   - nuovo `mzlab/web/canali.js` (modulo `m:`, `// @ts-check`, in un progetto di `tools/tipi/`, 0 errori); in `pannelli.js`, `grafici.js`, `tabs.js`, `explore.js` solo le righe del cromatogramma e del pannello `mrm`;
   - `xic-lente.js`/`xic-spettro.js` solo per passare `canale=`; lingue in un file proprio `lang/it/canali.js` e `lang/en/canali.js`.
   - Non toccare: albero e lista scan (`infusione*.js`, `banco.js`: CANALI-E), tabella di carico (`shell.js`: CANALI-F), barra dello spettro, titolo MSⁿ e mirino (VISORE-ZOOM).
   - `xic-spettro.js` è anche di VISORE-ZOOM: tocca solo la costruzione della query.
4. **Lavoro**:
   - `canali.js`: `canali(k)` (cache di `api/canali`), `traccia(k, id, opz)`, `omologhi(files, omo)`.
   - Selettore ad albero (`role="tree"`, tastiera con frecce, `aria-label`) nel pannello cromatogramma: TIC in cima, figli per genitore, colonne Canale · Sigla · n · RT; con più file le righe si uniscono per `omologo` e mostrano quanti file le hanno. Più canali spuntati = più tracce.
   - L'`omologo` usa la CE arrotondata all'intero; la CE esatta del file sta nel `title` della riga del canale.
   - Il pannello salva `omologo`, non l'indice; un file senza quel canale compare in legenda con «—». Colori Okabe-Ito da variabili CSS; tabella dei dati `TDATI` con una colonna per file e canale.
   - Il pannello `mrm` diventa un pannello cromatogramma con canali `srm:` (stesse tracce, stessa scelta quant/qual di `calLoad`).
   - XIC e lente: `api/xic?canale=` del canale selezionato.
   - Default per tipo (`tipo.proposto` di `api/canali`) in `esperimenti.js`: canale iniziale e lente proposta sì/no (la condizione «infusione → lente spenta» di VISORE-ZOOM diventa una riga di questa tabella). Nessun tipo spegne uno strumento.
   - I selettori `kind/prec/tr` restano dietro un adattatore finché un e2e li usa; quelli non più usati si tolgono con le loro chiavi.
   - **Regola dei testi di Federico**: pochi testi. Niente paragrafi esplicativi dove l'interfaccia si capisce da sola; tooltip (`title`/`aria-label` descrittivi) SOLO per dire cosa fa un campo o cosa inserirvi, mai opinioni, avvisi o cautele. Un testo che non dice cosa fa o cosa inserire non si scrive (e non ha chiave i18n). Ogni testo nuovo è una chiave in `lang/it/` e `lang/en/` (`docs/agenti/lingue.md`).
5. **Accettazione**: i sette casi di §3 (timololo MSn a segmenti, bentazone MS2 mirato, MRM QTRAP, EMS/EPI, DDA Orbitrap, DAD, SIM; sintetici dove mancano i dati) aprono il canale di default; overlay della stessa transizione su 3 file; bersagli ≥24×24 px; `e2e_axe_grafici` verde; `window.MZLAB_LR_MISS` vuoto; `tour.js` e `e2e_tour.py` validi; `tools/controlla_i18n.py` verde.
6. **Prove**: nuovo `tests_e2e/e2e_canali.py`, `python3 tools/verifica.py --cambiati`, job `tipi`, `python3 tools/pre_pr.py`.
7. **Consegna**: PR aperta su `claude/canali-selettore`; a CI verde UN messaggio al Coordinatore (≤5 righe). Eventi della PR: skill `steward`.
