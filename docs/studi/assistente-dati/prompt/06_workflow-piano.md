# 06 — Istruzione di fase «piano» per l'orchestratore

| Voce | Valore |
|---|---|
| Modello, effort | Sonnet 5.5, `medium` (lo stesso orchestratore di `01`); `max_tokens` 6 000 per il turno che propone un piano intero |
| Forma | messaggio `role: "system"` a metà conversazione, aggiunto dal ciclo Rust quando la rotta è `plan`; non tocca il prefisso in cache |
| Token | istruzione ≈ 600; coda del turno ≈ 15 000 (ispezione ridotta, proposte, scelte del subagente 05, piano attuale, modelli di piano); uscita ≈ 1 200–4 000 |
| Costo | ≈ 0,02 $ a turno; un piano tipico = 2–3 turni con `flow::check` ≈ 0,06 $ |
| Accettazione | T10–T19: piano valido al primo tentativo ≥ 80 %, entro 3 tentativi 100 %; nessun passo fuori dal modello scelto senza ragione; ogni valore con `source` |

```text
Phase: workflow. Build or change the processing plan now.
1. Pick one template from <data source="templates"> that matches the acquisition type in the inspection (full scan unit resolution, full scan high resolution, DDA, DIA, MRM, product ion, infusion). If two match, ask the user which goal they have, unless they already said it. Never start from an empty plan.
2. Use the parameter choices from the "param-scelte/1" result. Copy value and source. Do not change a value the parameter subagent kept from a rule unless the user asked.
3. Adapt the template only for a reason the user gave or the inspection shows: remove gap filling when there is one file; remove blank filtering when no blank is confirmed; add the MS2 link only when files have MS2 scans; split the plan per polarity when the inspection says the series must be split.
4. Propose the whole plan with one flow_propose_plan call. For later edits use flow_set_param or flow_add_step, one change per call.
5. If the program returns problems, read each one, fix the cause, and propose again. At most 3 attempts in total; then list the remaining problems in plain words and stop.
6. Then tell the user, in at most 6 lines: the template you used, the steps you added or removed and why, the parameters whose source is "model" with their reason. Do not list every parameter.
Do not call flow_run. The user starts the run.
```

## Esempio di ingresso sintetico

Dopo `03` e `05` sulla serie G01; l'utente ha risposto «transformation products» e «none» per i bianchi.

```json
{"templates":[{"id":"lr_fullscan_tp","steps":["features.estimate@1","features.detect@1","features.align@1","features.gapfill@1","table.filter@1","tp.dig_lr@1"]},
 {"id":"lr_fullscan_screen","steps":["features.estimate@1","features.detect@1","features.align@1","table.filter@1"]}],
 "param-scelte/1":{"choices":[{"step":"features.detect@1","name":"axis","value":"lr","source":"rule:R-axis-1"},{"step":"tp.dig_lr@1","name":"tol_da","value":0.4,"source":"rule:R-da-1"}]},
 "answers":{"goal":"transformation products","blanks":"none"}}
```

## Uscita attesa

Una chiamata `flow_propose_plan` con il modello `lr_fullscan_tp`, i valori di `param-scelte/1` con il loro `source`, `table.filter` senza `blank_ratio`. Dopo `flow::check` senza problemi, il testo:

«I used the template for transformation products at unit resolution. I left out the blank filter because no file is a blank. All parameters come from the program's rules; none was changed. You can start the run when you are ready.»

## Criterio di accettazione

- Il piano passa `flow::check`; la catena coincide con un modello o differisce solo per i casi del punto 3 (controllo in `assist eval`: differenza dei passi motivata da un'etichetta del caso).
- Nessuna chiamata a `flow_run` in questa fase (politica + controllo del registro).
- Testo ≤ 6 righe, nessun elenco completo dei parametri.
