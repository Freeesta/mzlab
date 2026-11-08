# Verifica dei lotti Gemini 16 (crate `mzlab-tutor`) e 17 (crate `mzlab-libro`)

Fase 1: solo report, nessuna modifica al codice committata. Base: `origin/main` 73db2dd. Ogni punto è stato letto sul codice vero; dove possibile riprodotto con un test e misurato in release (`cargo test --release`, medie su 20-50 giri; il rumore di misura è ±10 %, quindi sotto il 10 % scrivo «nel rumore»). Le prove sperimentali (patch + test) sono state applicate in locale e poi annullate.

Corpus delle misure: i 23 capitoli `mzlab/web/teoria/src/*.md` (410 KB), le 46 pagine `mzlab/web/teoria/*.html` (513 KB), i 77 casi di `tests/data/tutor/eval/`.

## 1. Risultati misurati (le tre leve vere)

| Cosa | Prima | Dopo | Output |
|---|---|---|---|
| `mzlab_libro::norm` su tutti i .md (`index.rs:39` `base_letter`, ramo ASCII) | 135,7 ms | 9,0 ms (15×) | identico |
| `blocchi::build` (46 pagine) | 114 ms | 37 ms | stessi byte (sha256 `dbc1e7ec…`) |
| `cerca::Index::new` | 112 ms | 29 ms | identico (recall@5 0,811 invariato) |
| `search` per domanda | 234 µs | 185 µs | stessi risultati (7560 hit) |
| `convert` libro, 23 capitoli (`scan.rs:220` `at` in `unsafe_match`) | 53,7 ms | 32,9 ms | identico (hash) |
| `convert` + prefiltro per carattere in `replace_all` (`inline.rs:23`) | 32,9 ms | 22,2 ms | identico (hash, annotate sì/no) |
| `convert` totale dopo le due correzioni | 53,7 ms | 22,2 ms (2,4×) | identico; i test di parità col Python (HTML committato) passano |

Tutti i test di `mzlab-libro` e `mzlab-tutor` restano verdi con queste patch. Callgrind del convert iniziale: `unsafe_match::{closure#1}` ≈ 37 % delle istruzioni di `convert`, `replace_all` ≈ 30 %.

## 2. Lotto 16: `mzlab-tutor`

Legenda: **V** vero, **F** falso, **P** parziale. Gruppo: **a** sicuro (output identico), **b** bug reale (cambia l'output), **c** da non fare.

| # | Punto Gemini | Verdetto | Prova (file:riga) | Gruppo / guadagno / rischio |
|---|---|---|---|---|
| 1 | `stem()` va in panic su UTF-8 | **F** | `cerca.rs:28-37`: i tagli `len()-1` cadono dopo una vocale/`s` ASCII, quindi su confine di char; l'unico chiamante passa parole di `words()` (`lib.rs:21`), che spezza su tutto ciò che non è ASCII alfanumerico (+ `+/.`), quindi gli accentati non arrivano mai. `take(7)` su ASCII = `[..7]`. | nessuna correzione. Al più un `debug_assert!(w.is_ascii())`. |
| 2a | `STOP.contains` lineare (82 voci) | **V** | `cerca.rs:34`. Callgrind: `slice::contains` 14 % di `Index::new` (≈3-4 ms su 29). | a, marginale. `binary_search` su array ordinato va bene; `phf` = nuova dipendenza: c. |
| 2b | `format!(" {k}")` nel ciclo sinonimi | **V** | `cerca.rs:68`. Misurato `query_terms` 10,1 → 9,0 µs per domanda (su 185 µs). L'equivalenza con `match_indices` + byte precedente `' '` è esatta. | a, trivial (~1 µs/domanda). |
| 2c | stem come `[u8;8]`/u64 | **V** nell'idea | `stem` è `pub` e restituisce `String` (usato da `eval.rs:221`): cambia l'API pubblica. Guadagno: `stem` 7 % di `Index::new`; la `search` costa 185 µs a domanda contro secondi di chiamata al modello. | c (complessità e API per ~2 ms). |
| 3 | `section_of` lineare in `recall_at` | **V** | `cerca.rs:153`; ma solo valutazione (recall_at ≈ 9 ms su 77 casi). | a, nessun beneficio pratico; c se si cambia la firma di `search`. |
| 4 | `cut()` con `Vec<char>` e copie | **P** | `blocchi.rs:84`. Il codice proposto compila ed è **equivalente**: confronto con l'originale su 4000 stringhe casuali (accentati, giapponese, `\u{a0}`, tab, punteggiatura) = 0 differenze. Guadagno: build 36 → 31 ms (13 %, al limite del rumore). L'«O(N²)» c'è ma i paragrafi sono di 1-3 KB. | a. |
| 5a | `shingles`/`sha12` allocano | **V** | `blocchi.rs:101-117`. Il costo vero è SHA-256 (`sha12` 10 % del profilo), non le `String`. | a, trascurabile. |
| 5b | shingle come `u32` invece di stringa a 6 esadecimali | **V** nell'idea | Cambia il formato di `teoria-blocchi/1` (`hidden_shingles` nei blocchi committati/Worker) e le soglie in `checks.rs`/`eval.rs`. | c (decide Federico: formato). |
| 5c | `blocks.clone()` in `build` | **V** | `blocchi.rs:160`; sostituibile con `let n = blocks.len()`. Output identico (sha verificato). | a. |
| 5d | `cur.chars().count()` in `emit_section` | **V** | `blocchi.rs:64`, O(pezzi). | a, trascurabile. |
| 6.1 | `numbers()`: `Vec<char>` | **V** | `checks.rs:96`. 2,4 µs a chiamata. | a, trascurabile. |
| 6.2 | «bug» `13C`/`15N`/`1H` estratti come numeri | **P** | Il comportamento c'è (provato: «13C e 15N» → `["13","15"]`), ma non è un bug di correttezza: lo stesso estrattore gira su blocco e su risposta, quindi «13C» in entrambi dà 13 in entrambi. Saltare i numeri seguiti da lettera indebolirebbe il controllo: «100 mL», «50V» (quantità vere) non verrebbero più verificati. | c (cambia la severità del controllo anti-invenzione: decide Federico). |
| 6.3 | canonicalizzazione via `f64` | **P** | `checks.rs:116`. Imprecisa solo oltre 2^53 (cifre lunghe), e uguale sui due lati. Equivalente a trim di zeri su numeri normali (proposta non provata). | a, valore basso. |
| 6.4 | `sentences()` con `peekable`, `run()` con 2 `String` per blocco | **V** | `checks.rs:67`, `:150`. `checks::run` completo 217 µs. | a, trascurabile. |
| 7a | `invented_terms` ricalcola gli stem del glossario | **V** | `eval.rs:222-231`. `score()` = 126 µs/caso (86 termini). | a, ma solo CI/eval: ~0,1 ms per caso. |
| 7b | `shingles(answer::SYSTEM)` a ogni valutazione | **V** | `eval.rs:~262`, solo caso «iniezione»: `score` passa da 126 µs a 1157 µs. `OnceLock` ok (nessuna dipendenza). | a, ~1 ms per caso «iniezione». |
| 7c | `Case::ask()` fa JSON → stringa → parse | **V** | `eval.rs:44-47`. | a, trascurabile. |
| 8.1 | `tag_lower` allocato 3 volte in `tokens()` | **V** | `html.rs:~118-125`. | a, trascurabile. |
| 8.2 | `</SCRIPT>` maiuscolo non trovato / `</script` in una stringa | **P** | Il `find` è case-sensitive (`html.rs`, ricerca di `</script`), ma in `mzlab/web/teoria/*.html` ci sono 0 occorrenze di `</SCRIPT`; le pagine sono generate in minuscolo. «`</script` dentro una stringa JS termina lo script» è il comportamento dell'HTML stesso, non un bug. | a (hardening, 1 riga), nessun cambio sul corpus attuale. |
| 8.3 | `<script src="…" />` auto-chiuso mangia l'HTML fino al prossimo `</script>` | **F** | In HTML lo script non è auto-chiudente: anche il browser lo tratta come aperto. 0 occorrenze nel corpus. Il parser fa ciò che fa il browser. | nessuna. |
| 9 | allocazioni in `attr`, `squash`, `id.clone()`, `term_anchor.clone()` | **V** | `html.rs:62,~180`; `pagina/lettore.rs:82,275`. Il codice proposto per `attr` e `squash` ha la stessa semantica (letto, non provato `attr`); `squash` provato nel giro di build. | a, nel rumore. |
| 10 | `format!` in `push_str`, `passages` doppia scansione, `find` per ID | **V** | `answer.rs:113-142,179-180,50`; `pick.rs:82,99-108`. Una domanda costruisce 2 prompt: microsecondi. `passages` proposto equivalente. | a, solo estetica. |
| 11 | `State`/`Limits` senza `Copy` | **V** | `quota.rs:12,36`: solo interi (40 B). Aggiungere `Copy` obbliga a togliere i `.clone()` di `State::on` (`clippy::clone_on_copy`). | a, estetica, zero guadagno. |
| 12 | `share_text` / `mask_emails` / `mask_digits` | **V** | `stat.rs:138,147,173`: `chars().count()` e `Vec<char>` sono evitabili; `mask_digits` su byte è corretto (`+`, ` `, `-`, cifre sono ASCII). Eseguiti una volta per domanda. | a, estetica. |

## 3. Lotto 17: `mzlab-libro`

### Correttezza

| # | Punto Gemini | Verdetto | Prova | Gruppo / cambio di output |
|---|---|---|---|---|
| C1 | `replace_text`: `&src[..1]` panic su UTF-8 | **F** | `edit.rs:~84` instrada su `replace_text` solo se `src.starts_with(ascii alfabetico)`, quindi `src[..1]` è ASCII. Provati `è1`, `é`, `x1`, `q`: `Err("Unknown block.")`, nessun panic. | nessuna correzione. |
| C2 | `splice` con inizio > fine | **V** | `edit.rs:107` e `:143`. Riprodotto: `edit_block(…, "5-2", …)` e `"qa5-2"` su documento di 13 righe → **panic** `slice index starts at 4 but ends at 2` (nel `.wasm` è un abort). `3-2` non va in panic (range vuoto `2..2`, inserisce, poi la verifica di rilettura risponde `CANT`). | **b**: panic → `Err("Unknown block.")`. Attenzione al codice proposto: aggiunge anche `b > lines.len()` → errore, mentre oggi `b.min(lines.len())` accetta (tronca): quel pezzo **cambia comportamento** senza motivo; basta `a > b`. |
| C3 | `headings()` conta le righe `#` dentro ```` ```html ```` e `<!-- privato -->` | **V (latente)** | Riprodotto: con un blocco `html` che contiene `# x`, togliere quella riga fa rispondere a `retitled` «A section was added, removed or moved». Nei 23 capitoli: 0 righe `#` dentro blocchi html. Il codice proposto copre solo ```` ```html ```` (uguale a `blocks.rs:96`), non i commenti privati né altre recinzioni. | **b** (editor): rifiuta meno a torto. Nessun effetto sull'HTML prodotto. Priorità bassa. |
| C4 | `find_inner` conta `<b` anche in `<br>` | **P (non raggiungibile)** | `edit.rs:195`. In teoria vero. In pratica i tag con `data-src` sono `p`, `li`, `td`/`th`, `span`, `h*`, `summary`, e dentro non c'è mai un tag inline il cui nome inizi con il loro (gli inline prodotti sono `b i em sub sup code span a`). | a (hardening a costo zero, nessun cambio di output). |
| C5 | figura con `{#fig:id}` duplicato | **V** | `special.rs:179`. Riprodotto: due figure `x` e una `y` danno «Figura 01-p.1», «01-p.2», «01-p.2» (la terza ha lo stesso numero della seconda) e `id="fig-x"` duplicato. Le equazioni invece rifiutano il duplicato (`special.rs:48`). Nei 23 capitoli: 0 duplicati. | **b**: HTML silenziosamente sbagliato → `LibroError` «duplicate figure id». Dice cosa fa il converter Python? Non verificato: se il riferimento Python accetta, è un cambio di comportamento visibile all'autore. Decide Federico. |
| C6 | `\0` nel sorgente collide col segnaposto | **V** | `inline.rs:131-160`. Riprodotto: «`\x000\x00`» nel testo diventa `<code>codice</code>` (il primo frammento trattenuto). 0 NUL nei capitoli. | **b**: solo per input con NUL; meglio un errore (`LibroError`) che l'Unicode privato proposto (il Python usa lo stesso `\0`). Decide Federico. |

### Velocità e peso

| # | Punto Gemini | Verdetto | Prova | Gruppo / guadagno / rischio |
|---|---|---|---|---|
| P1 | `inline::render`/`replace_all`: 9 `Vec<char>` + 9 `String` per paragrafo; «3-4×» | **P** | `inline.rs:23-37`. Vero che ogni riga scandisce 10 volte. Ma la riscrittura su `&str` delle 7 funzioni (`cite`, `link`, `reference`, `bold`, `emph`, `tight`, `run`) è grossa. Basta un prefiltro: ogni matcher richiede `c[i] == trigger` (letto uno per uno), quindi `if !text.contains(trigger) { return text.to_string() }` dà un output identico. Misurato: `convert` 32,9 → 22,2 ms (−33 %, hash identico). Il resto (riscrittura completa) vale altri ~3 ms stimati da callgrind (`replace_all` + `Vec<char>` ≈ 14 % di quello che resta). | a con prefiltro (rischio quasi nullo); riscrittura completa: **c**. |
| P2 | `src_attr`: `format!` anche con `annotate == false` | **V** | `blocks.rs:57`. Callgrind: 4320 istruzioni in tutto (180 chiamate) con `annotate == false`. | a, ma inutile (il guadagno non c'è). c. |
| P3 | `subst_refs`/`placeholder`: `Vec<char>` sull'intero HTML | **P** | `page.rs:155-205`; ~6 % di `convert` (≈1,4 ms). **Il codice proposto cambia l'output**: su `{{{eq:a}}}` l'originale produce `{eq. 01-p.1}` (riprovato), il proposto salta 2 byte dopo il primo `{{` che non combacia e lascia `{{{eq:a}}}`. Si corregge avanzando di 1 carattere invece di 2. | a solo con quella correzione; guadagno basso. |
| P4 | `base_letter`: scansione lineare per ogni carattere | **V**, **la leva più grossa** | `index.rs:39`. Con il ramo `c.is_ascii() → c`: `norm` 135,7 → 9,0 ms (15×, meglio dei «5-10×» stimati), `Index::new` 112 → 29 ms, `build` 114 → 37 ms. `FROM` e `TO` hanno entrambi **80** caratteri (non 83: contati), quindi `[u8;80]` va; più semplice tenere `TO.as_bytes()[p] as char`. Output identico. | **a**. |
| P5 | `unsafe_match`: `Vec<char>` e `count()` ripetuti | **V** (hotspot reale) | `scan.rs:220`: `at()` fa `zip + filter + count` su ogni posizione per `javascript:`. ~37 % del `convert`. **Il codice proposto va in panic su qualsiasi testo non ASCII** (riprodotto con una copia: `"<p>è</p>"` → «byte index 4 is not a char boundary»; inoltre `to_lowercase()` cambia le lunghezze). Correzione sicura e equivalente: `let mut it = c[p..].iter(); w.chars().all(\|x\| it.next() == Some(&x))` (zip-count == len ⇔ prefisso). Misurato: 53,7 → 32,9 ms (−39 %), hash identico. | **a** (con la correzione mia), la versione di Gemini: **c**. |
| P6 | `frag::decode`/`escape` a catena | **P** | `frag.rs:16-30`. `escape(s)` = `crate::escape(s, false)`: stesso output. **Il `decode` proposto è sbagliato**: usa `&apos;`, mentre il vero decode gestisce `&#x27;` e `&#39;` (che è ciò che `escape(…, true)` scrive): perderebbe gli apostrofi nel round-trip dell'editor. Ok solo l'uscita rapida `if !s.contains('&')`. Percorso usato solo con `annotate`. | a, valore basso. |
| P7 | `convert`/`teoria_page`/`inject_tema`: catene di `.replace` sull'intera pagina | **V**, trascurabile | `page.rs:86-90,147-151,208`: ≈500 KB di memcpy a pagina (~50 µs). Spostare `@BODY@` per ultimo cambia l'output se il corpo contiene `@SCRIPTS@` letterale. | c (rischio > guadagno). |
| P8 | `first2` alloca | **V** | `scan.rs:15`, `blocks.rs:175`. `matches!(l.as_bytes().get(..2), Some(b"$$"\|…))` equivalente (prefissi ASCII). | a, trascurabile. |
| P9 | `has_word_start` alloca | **V** | `index.rs:173`; equivalente esatto (`starts_with(k)` o `k` preceduto da spazio). Stesso schema in `cerca.rs:68`. | a, ~1 µs per domanda. |
| P10 | `equation`: `Vec<char>` per riga | **V** | `special.rs:21`. Il prefiltro `lines[j].contains("$$")` è valido perché `eq_end` (`scan.rs:161`) cerca solo `$$`. | a, trascurabile. |

## 4. Proposte per la fase 2

**(a) Sicuri, output identico** (consigliati, in 2 PR separate):
- PR libro: `base_letter` (P4), `at` in `unsafe_match` (P5, versione corretta), prefiltro `replace_all` (P1), uscita rapida in `frag::decode`, `has_word_start`, `first2` (P9, P8). Guadagno misurato: `convert` 53,7 → 22 ms, `norm` 15×.
- PR tutor: `build` senza clone, `cut` su `&str` (4000 casi senza differenze), `squash`, `OnceLock` per gli shingle di `SYSTEM`, `query_terms` senza `format!`. Il guadagno tutor viene soprattutto da P4 (build 114 → 37 ms), il resto sta nel rumore.

**(b) Bug reali, cambiano l'output «in meglio»** (decide Federico dove tocca comportamento visibile all'autore):
1. C2 panic su `a > b` (solo `a > b`, non `b > lines.len()`): non cambia nessuna risposta valida.
2. C5 figura con id duplicato → errore (in linea con le equazioni). Visibile all'autore del capitolo.
3. C6 NUL nel sorgente → errore. Visibile solo a chi mette un NUL.
4. C3 `headings` ignora le righe dentro i blocchi `html`: l'editor smette di rifiutare a torto; l'HTML non cambia.

**(c) Da non fare**:
- Riscrittura completa dell'inline su `&str` (P1): ~3 ms in più per un rischio medio su 7 funzioni.
- Codice proposto di `unsafe_match` (panic su non-ASCII), `subst_refs` (cambia `{{{eq:}}}`) e `decode` (perde `&#x27;`).
- Stem come `[u8;8]`/u64 e `phf` (API pubblica e dipendenza per ~2 ms); shingle `u32` (cambia il formato).
- Ignorare i numeri incollati a lettere in `checks::numbers` (indebolisce il controllo anti-invenzione).
- `src_attr` con `Display`, catene di `replace` in `page.rs` (guadagno nullo).

## 5. Come riprodurre

Test e bench usati (non committati): `crates/mzlab-libro/tests/zz_verify.rs` (casi C1-C6), `crates/mzlab-libro/tests/zz_bench.rs`, `crates/mzlab-tutor/tests/zz_bench.rs`; `cargo test --release -p <crate> --test zz_bench -- --nocapture`. Profili con `valgrind --tool=callgrind` su un binario con `CARGO_PROFILE_RELEASE_DEBUG=1`. In fase 2 i bug (b) avranno un test che fallisce prima e passa dopo. Il WASM non è stato misurato: la dimensione del `.wasm` e il limite di memoria vanno controllati in fase 2 con `tools/pre_pr.py`.
