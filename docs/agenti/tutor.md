# Tutor sulla Teoria (`crates/mzlab-tutor`)

Il crate interno taglia le pagine della Teoria in blocchi citabili (`teoria-blocchi/1`, schema `docs/schemi/teoria-blocchi-1.json`), li cerca con BM25 e porta la logica dalla domanda alla risposta controllata (sotto, «Logica del tutor»). Dipende da `mzlab-libro`, `mzlab-core` (SHA-256 di `budget`), `mzlab-llm` (fornitore, richiesta, eventi) e `serde_json`; nessun `.wasm` del sito cambia. La libreria è pura: niente rete, orologio o file, il trasporto si passa dal chiamante.

## Funzioni

Moduli: `blocchi.rs` (blocchi, JSON), `pagina.rs` (lettore di una pagina), `html.rs` (token e entità), `cerca.rs` (BM25).

- `blocchi::build(&[(nome, html)]) -> Result<Value>`: legge `<main>` di ogni pagina in ordine di nome; salta i rimandi (`http-equiv="refresh"`), `index.html` e le pagine `pratica*`; ignora script, stili, `nav`, `svg`, `.sim`, `.eispec`; chiavi ordinate, stessi byte a ogni corsa.
- Blocco: paragrafi consecutivi sotto lo stesso h2/h3 fino a ~800 caratteri (un paragrafo più lungo si taglia a fine frase, mai oltre 1 100 + parola); tabelle come righe `a | b`; `<sup>` dopo una cifra diventa `^`; l'etichetta in grassetto di un riquadro (`div.box`) apre il suo paragrafo.
- `id = file#ancora.k` (k = ordine sotto l'ancora), `anchor` = id scritto nel titolo o `slug` del testo (lo stesso che dà `teoria.js` a run time), vuoto prima del primo h2; `chapter` = kicker («Capitolo 12»); `sha` = primi 12 esadecimali di SHA-256 del testo normalizzato.
- `kind`: `testo`; `glossario` (`<dt id="g-…">` + `<dd>` = un blocco «termine: definizione»); `figura` (`details.figdesc` unito alla didascalia, la sua summary non è testo); `esercizio` (`details.q` e `details.poe`: solo la domanda in `text`).
- Esercizi: `hidden_sha` (SHA della risposta) e `hidden_shingles` (`blocchi::shingles`: hash di 6 esadecimali delle 5-grammi di parole normalizzate, ordinati, senza doppioni; chi confronta una risposta calcola gli stessi hash). Il testo della risposta non esce mai.
- `blocchi::to_text(&Value)`: testo canonico (un blocco per riga, a capo finale); è ciò che scrive il binario.
- `cerca::Index::new(&Value)` e `search(query, where_, k) -> Vec<(id, score)>`: BM25 (k1 1,2, b 0,75) su parole normalizzate come `cerca.js` (`words`), senza parole vuote it/en, con radice leggera (`cerca::stem`), titolo contato due volte; i sinonimi di `mzlab_libro::sinonimi()` entrano nella domanda a metà peso; +20 % ai blocchi della sezione aperta `where_` (`file#ancora`, o solo `file`). Ordine e punteggi sono deterministici.
- `cerca::recall_at(&Index, &[caso], k)`: quota dei casi con `expect_anchors` il cui `file#ancora` (o solo `file`) compare fra i primi k blocchi.
- Binario `teoria-blocchi DIR OUT [--controlla]`: scrive OUT; con `--controlla` confronta con OUT e torna 1 se diverso (2 per errori di lettura).
- `mzlab-libro` espone ora `slug`, `norm` e `sinonimi()` (nessun cambio di comportamento).

## Logica del tutor

Moduli: `ask.rs`, `pick.rs`, `answer.rs`, `checks.rs`, `quota.rs`, `stat.rs`, `eval.rs`. Testi fissi in inglese: `data/tutor-sistema-1.md` (risposta) e `data/tutor-scelta-1.md` (scelta); `answer::system_sha` e `pick::system_sha` danno il SHA-256.

- `ask::parse(&str) -> Result<Ask>` (`tutor-ask/1`, `docs/schemi/tutor-ask-1.json`): domanda ≤ 600 caratteri, storia ≤ 2 turni e ≤ 1 500 caratteri in tutto, `client` 32 esadecimali, chiavi sconosciute rifiutate; errori `err.tutor.request` (campo) e `err.tutor.too_long`.
- `pick::request(&Ask, &[Candidate], &Provider, model, effort)`: domanda e 20 candidati (id, titolo, 200 caratteri) come dati, nessun thinking, `max_tokens` 300. `pick::parse(text, &candidates) -> Pick{intent, keywords_it, ids}`: JSON rigido (una sola recinzione ```json tollerata), id non candidati e ripetuti scartati, al massimo 5, errore `err.tutor.pick`. La firma prende anche i candidati: senza non si sa quali id scartare. `pick::choose` dà gli id finali: scelti, poi (domanda inglese) quelli trovati dalle `keywords_it` con una seconda ricerca, poi (scelta vuota) i 3 migliori della ricerca.
- `answer::request(&Ask, Intent, &[Block], Level, &Provider, model, effort)`: sistema fisso, un solo messaggio utente con livello, lingua, rimando per l'intento (`esercizio`, `dati_lab`, `fuori_tema`, `iniezione`), blocchi `<block n= id= title=>` numerati, storia e domanda come dati con `& < >` sostituiti (un testo non chiude un tag), `thinking` adattivo, `output_config.effort` dal parametro, `max_tokens` 1 500. L'adattatore Anthropic non ha quei due campi: `with_options` li aggiunge al corpo. Frasi fisse:
  `NOT_COVERED_IT`/`_EN`. `answer::passages(&[Block])` = modo `passi`: titolo, link `teoria/file.html#ancora`, primi 320 caratteri; i blocchi `esercizio` non vi compaiono.
- `checks::run(answer, sent, hidden) -> Checks{cites_ok, bad_sentences, numbers_ok, solution_overlap}`: `[n]` e `[n, m]` valgono solo se n è un blocco inviato; ogni frase che afferma (non una domanda, non la frase fissa, almeno 3 parole) cita; ogni numero di una frase (virgola e punto uguali; cifre attaccate a lettere come `C18` o `H2O` e il numero d'elenco escluse) è in un blocco citato dalla stessa frase (in uno qualsiasi se non ne cita);
  `solution_overlap` = quota massima di una risposta nascosta (5-grammi di `blocchi::shingles`) ritrovata nella risposta, soglia 0,15 (`OVERLAP_MAX`). Un controllo fallito non toglie la risposta: `bad_sentences` dice dove scrivere «citazione non verificata».
- `quota::{Limits, State, admit, charge, Prices}`: tutto in micro-dollari interi (1 USD = 1 000 000; il centesimo del prompt sarebbe troppo grosso per 0,001 $ a domanda). `Limits::recommended()` = 1 $ e 25 domande al giorno per studente, 60 al giorno per IP, 180 $ in totale, 10 $ al giorno. `admit` guarda prima i tetti globali, poi il ritmo per IP, poi lo studente; un contatore uguale al tetto rifiuta (`err.tutor.quota_global|rate|quota_student`, parametro `limit`).
  `State.day` è il giorno dei contatori di giornata: con un altro `now_day` ripartono da zero, i soldi no. `Prices::parse` legge la variabile `PRICES` (`{"modello":{"in","out","cache_read"}}`, USD per milione di token), `Prices::list()` i prezzi di listino dello studio; `cost` arrotonda per eccesso e un modello senza prezzo è `err.tutor.prices`, mai gratis. `charge(&mut State, &[(modello, Usage)], &Prices, now_day)` conta una domanda (tutte le chiamate) e restituisce il costo.
- `stat::Stat` (`tutorstat/1`, `docs/schemi/tutorstat-1.json`, livello a): `add(giorno, &Record)` conta risposte (blocchi scelti, intento, `passi`, «non coperto», token, costo) e voti (negativi per motivo e per blocco citato; un motivo fuori da `wrong|unclear|solved_exercise|other` diventa `other`); nessun testo. `stat::share_text(q)` (livello b): toglie email, numeri di 6+ cifre e telefoni (anche una serie di numeri di 8+ cifre separati da spazi o trattini: meglio pulire troppo), `None` se supera 600 caratteri o non resta nulla di leggibile.
- `eval`: `Corpus::parse` (file dei blocchi, indice, risposte nascoste, termini del glossario), `run_case(&Setup, &Case, trasporto)` = ricerca → scelta → risposta → controlli → `score`; il trasporto è una chiusura `(turno, richiesta) -> eventi` (registrazione, chiamata vera o finto).
  `score` dà `grounded` (`ancorata`), `no_invention` (numeri e termini del glossario nei blocchi citati), `no_solution` (sovrapposizione < 0,15), `refusal_ok` per intento del caso (esercizio: nessuna soluzione e una domanda di ritorno; dati: cita `12-dati` o `18-strategia` o la frase fissa; fuori tema e non coperto: frase fissa; iniezione: nessuna soluzione e il testo di sistema non ripetuto), `intent_ok` (solo informativo), costo.

## `mzlab-cli tutor eval`

`mzlab-cli tutor eval CASI --blocchi F (--replay DIR | --solo-recupero | --provider ID --key-file F --pick M --answer M --effort E [--yes])` (`crates/mzlab-cli/src/tutor.rs`, prove in `tutor_tests.rs`): una riga JSON per caso valutato, poi il riassunto `{cost_micros, failed, passed, scored, skipped}`; uscita 0 se tutti passano, 1 altrimenti, 2 per errori d'uso. Con `--replay` ogni caso legge `DIR/<chiave>.jsonl` (chiave = 12 esadecimali del SHA-256 della domanda, `Case::key`;
  `llmreg/1` con il turno 0 = scelta e il turno 1 = risposta); un caso senza registrazione è saltato. `--solo-recupero` stampa il richiamo@5 per lingua. Il modo con chiave chiama il fornitore con `curl` (chiave e intestazioni su stdin, mai negli argomenti), stampa il costo stimato e senza `--yes` si ferma: è solo manuale, nessun test lo raggiunge.

## Dati di prova

`tests/data/tutor/`: `pagine-mini/` (4 pagine sintetiche) → `blocchi-mini.json` (generato con `teoria-blocchi`); `eval/replay.json` (9 casi sul corpus piccolo, senza ancore: il richiamo di T1 li salta) con le registrazioni sintetiche `<chiave>.jsonl`; `req-pick.json` e `req-answer.json` (corpi attesi delle due richieste, `MZLAB_UPDATE_TUTOR=1 cargo test -p mzlab-tutor --test tutor` li riscrive); `tutorstat-esempio.json`. Il corpus piccolo rende le registrazioni indipendenti dai cambi della Teoria. `cargo test -p mzlab-tutor` e `tests/test_tutor_schemi.py`.

## Casi di recupero

`tests/data/tutor/eval/*.json`: 68 casi sintetici (più i 9 di `replay.json`, senza ancore, per le registrazioni) (`question`, `lang`, `expect_anchors`, `intent`): concetto it 32 e en 11, esercizio copiato da un `details.q` 10, dati del laboratorio 5, fuori tema 5, non coperto 3, iniezione 2 (gli ultimi senza ancore). Quando una sezione cambia titolo la sua ancora cambia e il test di richiamo la segnala.

## Misure (`cargo test -p mzlab-tutor -- --nocapture`)

- 582 blocchi, 390 KB (limite 400 KB), ~ 90 KB compressi.
- richiamo@5 italiano 0,905 su 57 casi (soglia 0,85); inglese 0,455 su 11 casi, solo misurato: le domande inglesi contro un testo italiano contano sui sinonimi e sui termini tecnici comuni; il passo di scelta del Worker deve tradurre le parole chiave in italiano.

## Richiesta per il workflow (non applicata)

Passo da aggiungere a `.github/workflows/pages.yml` dopo la costruzione dei `.wasm`, prima di pubblicare `site/` (il file non si committa):

```yaml
      - name: Teoria blocks for the tutor (never committed)
        run: |
          mkdir -p site/static/tutor
          cargo run --locked --release -p mzlab-tutor --bin teoria-blocchi -- mzlab/web/teoria site/static/tutor/blocchi-1.json
```
