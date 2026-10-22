# Costo: modello per sessione, cache, tetti, piano del test

## 1. Prezzi verificati (pagina pricing ufficiale, letta il 22/10/2026, USD per milione di token)

| Modello | Ingresso | Scrittura cache 5 min | Scrittura cache 1 h | Lettura cache | Uscita |
|---|---|---|---|---|---|
| `claude-sonnet-5-5` | 2,00 | 2,50 | 4,00 | 0,10 | 10,00 |
| `claude-haiku-5-5` (prompt ≤ 100 000 token) | 0,10 | 0,125 | 0,20 | 0,01 | 0,50 |
| `claude-haiku-5-5` (prompt > 100 000 token) | 0,50 | 0,625 | 1,00 | 0,05 | 2,50 |
| `claude-opus-5-5` (solo giudice facoltativo) | 4,00 | 5,00 | 8,00 | 0,20 | 20,00 |

- Batch API: −50 % su ingresso e uscita, si somma alla cache (via V3a, scelta: si usa per le valutazioni a chiamata singola T1–T8; la lettura della cache nei lotti è possibile ma non garantita). `inference_geo: "us"` costa ×1,1: non si usa.
- Uso degli strumenti: +286 token di sistema per richiesta con strumenti (`tool_choice` `auto`) su Sonnet 5.5 e Haiku 5.5.
- Il ragionamento (thinking adattivo, sempre attivo su Sonnet 5.5) si paga come uscita: con effort `medium` l'uscita per turno include il ragionamento; `max_tokens` per ruolo lo limita.
- Prefisso minimo per la cache: 512 token su Sonnet 5.5 e Haiku 5.5. La cache conta solo prefissi identici: strumenti → sistema → messaggi.
- Due correzioni al codice esistente (da fare nella PR A1, `07-mappa-codice.md`): `mzlab-tutor::quota::Prices::list` ha `cache_read` 0,2 per Sonnet 5.5 (la pagina dice 0,10) e non ha i prezzi di scrittura; `mzlab-llm::event::Usage` non porta `cache_creation_input_tokens` (5 min e 1 h), quindi oggi il costo di una scrittura in cache non si conta.

## 2. Sessione tipo

Ipotesi: 12 file, una corsa, 25 turni dell'orchestratore, 5 grafici. Prefisso stabile dell'orchestratore ≈ 25 000 token (sistema 3 000, catalogo `strumenti/1` ≈ 20 000, strumenti nuovi 2 000).

| Fase | Chiamate | Token in (di cui in cache) | Token out | Costo |
|---|---|---|---|---|
| Ispezione dei file (Rust) | 0 | – | – | 0 |
| Router (Haiku, low) | 25 | 25 × 1 200 | 25 × 50 | 0,004 |
| Lettore dei metadati (Sonnet, medium) | 1 | 10 000 | 2 000 | 0,040 |
| Controllo cromatografia e QC (Sonnet) | 1 | 8 000 | 1 500 | 0,031 |
| Scrittura del prefisso, TTL 1 h | 1 | 25 000 scritti | – | 0,100 |
| Domande e risposte (orchestratore) | 6 | 6 × (25 000 c + 8 000 c + 1 500) | 6 × 1 000 | 0,101 |
| Workflow e parametri: subagente × 2 | 2 | 2 × 13 000 | 2 × 2 500 | 0,102 |
| Workflow e parametri: orchestratore | 8 | 8 × (25 000 c + 15 000 c + 1 500) | 8 × 1 200 | 0,152 |
| Interprete della feature table | 1 | 15 000 | 3 000 | 0,060 |
| Domande sulla tabella (orchestratore) | 6 | 6 × (25 000 c + 22 000 c + 2 000) | 6 × 1 000 | 0,112 |
| Grafici: Haiku × 5 + orchestratore × 5 | 10 | 5 × 2 500; 5 × (25 000 c + 25 000 c + 800) | 5 × 400; 5 × 600 | 0,065 |
| **Totale** | 60 | | | **≈ 0,77 $** |

- «c» = letto dalla cache a 0,10 $/MTok; la coda nuova di ogni turno si paga a prezzo pieno e la sua scrittura nella cache automatica (5 min) è inclusa con un margine del 25 %.
- Caso pesante (60 turni, tre corse, tabelle grandi, risposte lunghe): ≈ 2,5 $. Caso pessimo con cache che non funziona: ≈ 4 $. Il tetto di 10 $ copre il caso pessimo con margine.
- Senza cache il solo prefisso costerebbe 25 × 25 000 × 2 $/MTok = 1,25 $ per sessione: la cache vale circa il 60 % del costo.

## 3. Regole di cache

- Un punto di cache esplicito con `ttl: "1h"` sull'ultimo blocco del sistema (dopo gli strumenti): l'utente legge e pensa, pause oltre 5 minuti sono normali; tre scadenze da 5 minuti costerebbero di più di una scrittura da 1 h (0,19 $ contro 0,10 $).
- Cache automatica (`cache_control` in cima alla richiesta, 5 min) per la coda della conversazione.
- Catalogo con byte stabili (`llm_tools` lo garantisce già), strumenti in ordine fisso, nessuna data o id variabile nel sistema; le istruzioni a metà conversazione passano come messaggio `role: "system"` (Sonnet 5.5 e Haiku 5.5 lo accettano, non toccano il prefisso).
- Un cambio di effort a metà conversazione invalida la cache dei messaggi: l'effort è fisso per ruolo.
- I subagenti hanno prefissi propri (sistema del ruolo, ≥ 512 token, con cache 5 min): utile quando lo stesso ruolo gira più volte in pochi minuti (router).
- Controllo: dal secondo turno `cache_read_input_tokens` > 0; se è 0 per due turni di fila il contatore lo registra come `warn.llm.cache.miss` e il test si ferma (`prompt/00_esecuzione-test.md`).

## 4. Tetto per sessione e per persona

- Contatore in micro-dollari (`mzlab-tutor::quota` riusato e spostato in `mzlab-llm::spend`), aggiornato a ogni evento `usage` con ingresso, uscita, lettura e scrittura della cache.
- Prima di ogni chiamata: costo massimo = ingresso stimato (`ceil(byte/3)`, prudente) × prezzo pieno + `max_tokens` × prezzo di uscita. Se speso + massimo > tetto la chiamata non parte (`err.llm.limit.spend`): il tetto non si supera mai, anche con una stima sbagliata dei token in uscita.
- Arresto morbido: all'80 % (8 $) l'assistente lo dice una volta e propone di riassumere; al 100 % nessuna chiamata nuova, la risposta in corso finisce, restano attive le parti Rust (ispezione, proposte, controlli, grafici predefiniti) e l'esportazione della conversazione.
- `max_tokens` per ruolo: router 200, grafici 1 000, subagenti 4 000, orchestratore 3 000 (più lunghi solo per un piano intero: 6 000).
- Per persona: con chiave propria (sito V1a, desktop) il limite è quello della persona nella Console; con chiave condivisa i contatori per persona e per sessione li tiene il Worker (V2a, `09-worker.md`), con riserva del costo massimo prima dell'inoltro e saldo con `usage`. Nel test c'è una sola persona (Federico): tetto per sessione nel client + limite di spesa del workspace dedicato (200 $) nella Console; il 400 «specified workspace API usage limits» diventa `err.llm.limit.workspace` e ferma tutto senza ritentare.
- Il 429 di frequenza con `retry-after` si ritenta al più 2 volte; il 429 senza `retry-after` (`enforced_spend_limit_reached`) non si ritenta mai.

## 5. Piano del test con 200 $ (ogni sessione ≤ 10 $)

| # | Sessione | Modelli | Chiamate | Stima | Tetto |
|---|---|---|---|---|---|
| T0 | Prova di fumo: chiave, `usage`, cache letta al 2° turno, `refusal` e `max_tokens` gestiti | Haiku, Sonnet | 10 | 0,30 | 1 |
| T1 | Lettore dei metadati sui 24 casi d'oro × 3 ripetizioni | Sonnet medium | 72 | 2,90 | 10 |
| T2 | Stessi casi con Haiku low (serve per decidere il ruolo) | Haiku | 72 | 0,15 | 2 |
| T3 | Controllo cromatografia e QC, 12 casi × 3 | Sonnet medium | 36 | 1,10 | 5 |
| T4 | Proposta dei parametri, 16 casi × 3 | Sonnet medium | 48 | 2,45 | 10 |
| T5 | Effort `low`/`medium`/`high` sui casi T4 (un giro ciascuno) | Sonnet | 48 | 2,80 | 10 |
| T6 | Interprete della feature table, 12 tabelle × 3 | Sonnet medium | 36 | 2,16 | 10 |
| T7 | Grafici: 30 richieste × 2, Haiku e Sonnet low | Haiku, Sonnet | 60 | 0,30 | 2 |
| T8 | Router: 60 messaggi etichettati | Haiku low | 60 | 0,01 | 1 |
| T9 | Attacchi rossi di iniezione, 40 casi | Sonnet medium | 40 | 1,40 | 5 |
| T10–T19 | 10 sessioni intere simulate (utente a copione, `prompt/00`) | tutti | ≈ 60 ciascuna | 10 × 1,00 | 10 ciascuna |
| T20–T29 | 10 sessioni di Federico con file veri (risultati solo nel privato) | tutti | – | 10 × 2,50 | 10 ciascuna |
| – | Giudice Opus facoltativo su 40 risposte aperte | Opus low | 40 | 1,00 | 5 |
| | **Totale previsto** (con Batch su T1–T8: ≈ 44 $) | | | **≈ 50 $** | |
| | Riserva per rifare T1–T9 dopo correzioni dei prompt (due giri) | | | ≈ 30 $ | |
| | Margine non speso | | | ≈ 120 $ | |

- La spesa reale si confronta con la stima dopo T0 e dopo T1: se lo scarto supera il 50 % si ricalcola la tabella prima di andare avanti.
- Con la Batch API (V3a) T1–T8 costano la metà (≈ 11,9 $ → ≈ 6 $) e i risultati arrivano entro 24 h (di solito meno di un'ora); T0, T9 e le sessioni intere restano chiamate dirette perché hanno più turni.
- Data limite dei crediti: 16/11/2026. Ordine pensato perché T0–T9 (≈ 14 $, ≈ 8 $ con Batch) dicano entro pochi giorni se l'architettura regge.

## 6. Costo per compito (metrica del test)

- `costo_per_compito` = spesa della sessione / compiti d'oro completati nella sessione (un compito incompleto o sbagliato non conta al denominatore).
- Obiettivi: lettura dei metadati ≤ 0,05 $, proposta dei parametri ≤ 0,10 $, lettura della tabella ≤ 0,10 $, grafico ≤ 0,01 $, sessione intera ≤ 1,50 $ (mediana) e ≤ 5 $ (massimo).
