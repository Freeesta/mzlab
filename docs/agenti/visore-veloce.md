# Visualizzatore veloce: pulsante XIC dello spettro

- **Modulo**: `mzlab/web/xic-spettro.js` (modulo ES, `window.XICBTN`: `wire`, `click`, `mark`, `sync`, `chosen`, `pending`). Aggancio in `pannelli.js` (HTML della barra, `xicTraceFor`, `xicDirect(mz, after, panel, over)`), `spettro.js` (`specClick` cede a `XICBTN.click`), `picchi.js` (`mark`, `sync` in `drawSpec`), `grafici.js` (banda `p.band`), `azioni.js` (`contestoGrafico`: il picco scelto vince sul più alto), `eventi.js` (`pendKey`). Prova: `tests_e2e/e2e_xic_spettro.py`.
- **Tasti**: pulsante «XIC» = modo «scegli un picco» (Esc esce, catena `xic-scelta`); `,` e `.` picco precedente/successivo; Invio estrae; Maiusc+Invio o Maiusc+clic aggiunge una traccia; Spazio sul pulsante non scorre le scansioni.
- **Finestra**: LR = finestra nominale di `xicTraceFor` ([n−0,2; n+0,8]); HR = ppm del profilo del file (`HR.ionTrace`). Una tolleranza digitata sostituisce quella predefinita e compare nell'etichetta; mai inventata.
- **Linea di scansione**: `xp.cur` (banda se lo spettro è una media di più scansioni); apice e area in `.xb-info`; area subito in HR, in LR solo dopo l'integrazione dello studente.
- **Tipi di spettro coperti**: vedi la descrizione della PR del pacchetto V-1; Infusion MSn e Tabui restano fuori.

## Misure (G2: dal Invio nel pannello all'ultimo disegno dell'XIC, ioni non in cache, n = 30)
Macchina: container cloud 4 CPU, Chromium headless, dati sintetici (`.verifica/dati_sintetici`), sito costruito con `tools/prepara_sito.py`, ramo `claude/viewer-1-xic-button`.

| profilo | n | p50 (ms) | p95 (ms) | esito |
|---|---|---|---|---|
| LR, 1 file | 30 | 20,4 | 26,4 | p95 ≤33 ok; p50 ≤16 NO |
| LR, 7 file | 30 | 21,7 | 27,6 | p95 ≤33 ok; p50 ≤16 NO |
| HR, 1 file | 30 | 21,5 | 27,3 | p95 ≤33 ok; p50 ≤16 NO |

Dove va il tempo: il tempo misurato comprende la richiesta `api/xic` (JSON dal motore) e il disegno del pannello XIC (non scomposti); l'integrazione HR viene dopo l'ultimo disegno. Scendere sotto 16 ms p50 richiede di cambiare il percorso dei dati (fuori da V-1).
