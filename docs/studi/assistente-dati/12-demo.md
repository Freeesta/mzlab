# Demo: l'assistente presenta mzFlow

Obiettivo: davanti a un pubblico (seminario, gruppo, visitatori del sito) l'assistente apre i file di esempio, dice che cosa sono leggendo i metadati veri, fa 2–3 domande e prepara mzFlow con motivo e confidenza per ogni parametro. Il modello non misura: ogni numero viene da Rust (`ispezione/1`, `proposta-param/1`, `tabcheck/1`), il modello lo spiega e chiede. Prerequisiti: P1–P5 di `13-piano-dev.md`.

## 1. Copione (file di esempio del sito)

Scenario principale **HR**: gruppo «HRMS» di «Prova con i file di esempio» (4 file Orbitrap DDA). Scenario breve **LR**: gruppo «Full Scan» (5 file, triplo quadrupolo, ESI+), solo per mostrare che a risoluzione unitaria l'assistente propone un candidato e non un'identificazione e non dà i TP (principio 1).

| # | Chi | Cosa succede | Dove si vede |
|---|---|---|---|
| 1 | presentatore | apre l'esempio HRMS e il Flow; l'avviso di consenso mostra «What will be sent» (l'`ispezione/1` con alias `f1…f4`) | dialogo di consenso |
| 2 | Rust (senza modello) | carta «Files»: tipo, polarità, classe di risoluzione con la base, DDA con finestra di isolamento, gradiente, ruoli proposti | pannello, prima di qualsiasi chiamata |
| 3 | presentatore | chip «Explain these files» | – |
| 4 | router + lettore metadati + lettore cromatografia | lettura con 3–5 risultati, ognuno con prova cliccabile e confidenza | carta «Reading» |
| 5 | assistente | `ask_user` con 2–3 domande chiuse: obiettivo (screening non mirato / quantificazione / confronto di gruppi), matrice (acqua, suolo, biologica, standard), file da trattare come bianchi o QC se il nome non lo dice | carta «Questions» |
| 6 | presentatore | risponde con i pulsanti; chip «Set up mzFlow for me» | – |
| 7 | Rust + lettore parametri | `proposta-param/1`: tolleranza m/z, larghezza dei picchi, rumore, `rt_gap`, con `source = rule:R-*`, prova e confidenza; il lettore spiega e segnala i `missing` | carte «Parameter» con «Apply/Discard» |
| 8 | assistente | piano `mzflow/2` (`flow_propose_plan`), controllato da `flow::check`; differenza sulla tela | carta del piano, «Apply» = una voce di «Undo» |
| 9 | presentatore | «Apply», poi «Run» del Flow (corsa normale, senza modello); a fine corsa `tabcheck/1` e un grafico predefinito | tela e tabella |
| 10 | presentatore | spegne la rete (o usa il tetto) e mostra che ispezione, proposte e controlli restano: «Assistant not available» | carta dell'assistente |

## 2. Token e costo di una corsa del copione

Prezzi di `02-costo.md` §1 (Sonnet 5.5: ingresso 2 $, scrittura cache 5 min 2,5 $, 1 h 4 $, lettura 0,10 $, uscita 10 $ per milione; Haiku 5.5 a 0,10/0,50). Prefisso dell'orchestratore con il catalogo demo (P3): ≈ 8 000 token (sistema 3 000, strumenti 5 000) invece dei 25 000 del catalogo intero.

| Fase | Chiamate | Ingresso | Uscita | Costo $ |
|---|---|---|---|---|
| Scrittura del prefisso (1 h) | 1 | 8 000 scritti | – | 0,032 |
| Router (Haiku, low) | 4 | 4 × 1 200 | 4 × 50 | 0,001 |
| Lettori metadati, cromatografia, parametri (Sonnet, medium) | 3 | 3 × (6 000 + 2 000 di sistema scritti) | 3 × 1 500 | 0,10 |
| Orchestratore (Sonnet, medium) | 10 | 10 × (8 000 c + 6 000 c + 1 500 nuovi) | 10 × 1 000 | 0,15 |
| **Totale tipico** | 18 | | | **≈ 0,28** |

- Caso peggiore (cache mancata, un lettore ritentato, due piani rifiutati): ≈ 0,50 $ = tetto di sessione proposto (si ferma lì).
- Senza catalogo demo (25 000 token di prefisso) il tipico sale a ≈ 0,40 $: il catalogo ridotto vale circa il 30 %.
- Controllo durante le prove: dal secondo turno `cache_read` > 0 nel registro `llmlog/1` livello a.

## 3. Prove senza rete e senza rischio

1. **Registrare** (fase 0, solo Federico): con la frase segreta, il pannello in modalità proprietario ha «Record this session» che scrive la sessione come `llmreg/1` (`fake::Replay::record_line`, SHA-256 di ogni richiesta). Si registra il copione HR e quello LR una volta (≈ 0,30 $ ciascuno, 3–4 prove = ≈ 2 $).
2. **Byte stabili**: i messaggi del presentatore sono i chip del copione (testo fisso), non testo battuto: così la registrazione in modo `strict` combacia (`err.llm.replay.mismatch` altrimenti).
3. **Ripasso**: `window.MZLAB_LLM_AGENT.replay(testo, true)` o il menu proprietario «Play a recorded demo» (file in `static/llm/demo/`, prodotto dalla registrazione dopo la revisione) ripete la sessione identica senza rete e senza costo.
4. **Rete di sicurezza dal vivo**: se il proxy non risponde o il tetto scatta, il presentatore passa a «Play a recorded demo» e la sala vede la stessa sessione; l'e2e di P5 prova proprio questo passaggio.
5. CI: `tests_e2e/e2e_llm_demo.py` gioca le due registrazioni, controlla carte, «Apply/Undo», tetto finto, «Assistant not available» e nessuna richiesta fuori dal sito.

## 4. Quanto compra il budget (200 $)

| Voce | $ |
|---|---|
| Prove e registrazioni di Federico (fase 0) | 10 |
| Presentazioni dal vivo (10 × 0,30) | 3 |
| Riserva (prompt rifatti, ri-registrazioni) | 17 |
| Tetto mensile del proxy per il pubblico (incluso il proprietario) | 150 |
| Margine sotto il limite della Console (180 $) e del credito (200 $) | 20 |

| Tetti | Sessioni pubbliche con 150 $ | Al giorno con il tetto di 6 $ | Durata minima |
|---|---|---|---|
| tipico 0,28 $ | ≈ 535 | ≈ 21 | 25 giorni |
| tetto di sessione 0,50 $ (caso peggiore) | 300 | 12 | 25 giorni |
| tetto per IP 1 $ al giorno | ≥ 150 visitatori diversi | ≥ 6 IP | – |

Se il credito di 200 $ scade presto (i crediti di prova citati in `02-costo.md` scadono il 16/11/2026), la fase pubblica dura quanto il credito: è la domanda D7 di `14-decision-card.md`.
