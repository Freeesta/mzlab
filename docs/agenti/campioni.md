# Campioni: ruoli dei file e tempi (`campioni/2`)

Un solo modello Rust dei ruoli e dei tempi dei file di una serie, per la vista Dati e per mzFinder: `crates/mzlab-core/src/campioni/` (`mod.rs` tipi e `route`, `nomi.rs` parser dei nomi, `valida.rs` avvisi e indici per il percorso TP, `json.rs` JSON canonico e migrazione). Schema: `docs/schemi/campioni-2.json`. Test: `crates/mzlab-core/tests/campioni.rs`.

## Modello
- Un `Campione` per file, nell'ordine di iniezione. Campi JSON in italiano, valori in inglese: `file`, `sha256` (la chiave; vuoto se non ancora noto), `tipo` ∈ `sample t0 blank dark infusion std calibration qc`, `tipo_bianco` ∈ `solvent procedure` o null (facoltativo: tutti i bianchi si sottraggono allo stesso modo, nessun avviso se manca), `tempo` + `unita_tempo` ∈ `s min h d` (come scritto o confermato),
  `tempo_min` (derivato, l'unico usato nei calcoli), `tempo_confermato`, `serie` (`""` = l'unica, o condivisa da tutte), `replica`, `concentrazione`, `unita`, `ordine_iniezione`, `istd`, `note`, `composto` (per `infusion`, lo riempie mzFinder).
- `Campione::role()`: un `sample` a 0 minuti conta come `t0`. `set_time` ricava `tempo_min` (`minutes_per`). La vista Dati non scrive mai `tipo: t0` né `tipo_bianco` (un campione a 0 minuti e un solo tipo Bianco) e il tempo del `dark` è negativo; il Rust continua a leggere i vecchi `t0` e `tipo_bianco` (`CDOC.fields` li converte).
- `to_json` è canonico (chiavi in ordine alfabetico, stessi byte) e `digest` ne è lo SHA-256; `from_json` rifiuta schema, tipi, unità e numeri non validi (`error.badCampioni`) e ricalcola `tempo_min`.

## `proponi(&[(nome, sha256, kind)])`
- Un solo parser dei nomi, tempi sempre con `tempo_confermato = false`. Ruolo e minuti seguono `project::guess_sample` (stessi minuti sugli oltre 30 000 nomi generati della tabella di `project_parity`, controllato dal test), con in più: secondi (`90 s`, `90s`), la replica scritta `(2)` in fondo al nome, l'infusione (`infusion`, `infusione`, `direct-infusion`, `FIA` o `kind == "msn"`), il buio (`dark`, `buio`, `adsorption`), `tipo_bianco` da `solv…`/`proc…`.
- Senza unità nel nome il tempo è proposto in minuti. Un numero davanti a `ppm`, `mg/L`… è una concentrazione, non un tempo (`X25ppm_t10` = 10 min). Concentrazione solo per standard e taratura. Infusione e QC non hanno tempo.
- Le parole dei ruoli si cercano nel nome con le cartelle, come `campioni/1`; il tempo nel nome senza cartelle né estensione.
- `serie`: il prefisso prima del tempo (`X_TiO2_t005min` → `X_TiO2`); se i file con tempo hanno un solo prefisso non vuoto, tutti `""`. Un file senza tempo prende la serie più lunga con cui comincia il suo nome, altrimenti `""`.
- Repliche numerate per serie, ruolo, minuti e concentrazione (`number_replicates`); la replica scritta nel nome vince.
- `tipo_v1(nome)`: il tipo di `campioni/1` (senza infusione), per `mzlab-feat/src/samples.rs`, che resta un involucro con la stessa uscita.

## `valida(&[Campione]) -> Vec<Avviso>`
- `Avviso {code, severity, file, series}`; codici stabili, le parole le mettono le pagine. JSON `{"avvisi":[{"codice","gravita","file","serie"}],"bloccante"}`; `bloccante` se c'è un errore (mzFinder non parte).
- Per file (errori): `timeMissing` (sample senza tempo), `timeUnconfirmed` (sample, anche a 0 minuti, con tempo non confermato), `t0WithTime` (`t0` con tempo diverso da 0).
- Per serie (i file della serie più quelli con `serie` vuota): `noControls` (errore: né t0 né bianco né buio), altrimenti `noT0` e `noBlank` (avvisi) e `noDark` (informazione); `onePoint` (avviso: un solo tempo trattato); `replicates` (informazione: due trattati con stesso tempo e concentrazione).
- Su tutto: `noTreated` (errore: nessun sample; con `noControls` se mancano anche i controlli), `noInfusion` (informazione).

## `per_tp(&[Campione]) -> PerTp`
- Indici 0-based per ruolo: `treated` (sample con tempo > 0), `t0`, `blank`, `dark`, `infusion`, `std` (con `calibration`), `qc`, `untimed` (sample senza tempo utile).
- `minuti[]` compatibile con `feature_pipeline/1` (`genitore.minuti`): il tempo dei trattati, 0 per i t0, null per tutti gli altri; l'infusione è null e sta solo nel suo indice, mai fra i controlli.

## `da_legacy(json)`
- Ingresso `{"session":[NB.session v1], "qaqc":{"roles":{file: ruolo}}, "campioni1":{campioni/1}, "mzflow":{"files":[...]}}`, ogni parte facoltativa.
- Ordine dei file: sessione, poi `.mzflow`, poi `campioni/1`. Ogni campo viene dalla sessione, se no dal `.mzflow`, se no da `campioni/1`; il ruolo di `qqq.qaqc` vince (tiene QC e buio, che la sessione salva come `sample`). Vocabolari della sessione: `standard` → `std`, `control` → `dark`.
- I tempi della sessione e del `.mzflow` sono confermati (scritti dall'utente), quelli di `campioni/1` no. Un `sample` a 0 minuti diventa `t0`; bianchi, standard, QC e infusioni perdono il tempo; la replica scritta resta, le altre si numerano.

## Binding
- Una funzione per motore: `campioni(op, json)` in `mzlab-wasm/src/campioni.rs` (vista Dati) e in `mzlab-feat-table-wasm` (mzFinder), con `op` ∈ `proponi` (`{"files":[{"name","sha256","kind"}]}`), `valida`, `per_tp`, `canonico` e `digest` (un `campioni/2`: JSON canonico e SHA-256), `da_legacy`. Errori JSON `{error_key, params}` (`error.badCampioni`).
- `mzlab-core/src/project.rs` resta per la parità col Python del programma principale.

## Pannello Pipeline di mzFinder (`TP_Mine/files/finder-ruoli.js`, `TP_Mine/js/34_pipeline_run.js`)
- Le funzioni di `finder-ruoli.js` che chiamano il Rust (`build`, `check`, `forTp`, `canonical`) sono async: il `call` è il binding `.wasm` della tabella nel sito e la rotta nativa `finder/campioni` (`pool.campioni`) nell'app; `stemOf` toglie anche le estensioni doppie (`x.mzML.mzidx` dell'app).
- Il taccuino tiene i ruoli in `NB.campioni` (`campioni/2` canonico, una voce per file, chiave = nome `.mzML`). `build` parte da lì; per i file senza voce migra `NB.session` (v1) e i ruoli di `qqq.qaqc` con `da_legacy` (tempi confermati) e propone dai nomi gli altri con `proponi` (tempi non confermati).
- Il pannello mostra una tabella file/ruolo/tempo (`select` e `input` nativi, `aria-label`, fuoco ripristinato dopo ogni modifica) con un solo selettore d'unità; il tempo proposto è giallo finché non lo confermi (Invio o «Confirm times»). Gli avvisi brevi in `role=status` vengono dai codici di `valida`; «Run» è spento, con il motivo nel `title`, se `valida` è bloccante.
- La pipeline riceve `minuti[]` di `per_tp` e il `campioni/2` canonico (`campioni` del genitore → `build_evidence_to`, parametro facoltativo in coda: il cancello 1 dei TP legge i ruoli, l'infusione non è mai un controllo); il registro dell'esecuzione porta anche l'operazione `campioni` (JSON canonico e SHA-256 del Rust); il Feature QC usa ruoli e minuti del modello.

## Tabella dei file di Tabui LR (`TP_Mine/lazy/gruppi-campioni.js`)
- Dopo `TPLAZY.load("gruppi-campioni.js")`, `TPGRUPPI_FILES.mount(el, nomi, {R, call, nb, save, onChange})` disegna File | Role | Condition | Time | Replicate (`select`/`input` nativi, `aria-label`, fuoco ripristinato dopo ogni modifica); `20_tpmine.js` non lo usa ancora (l'innesto con il Dig per condizione è di GR-5). Ruoli, tempi, condizioni (`serie`), repliche e avvisi sono quelli di `finder-ruoli.js` (`build`, `check`, `canonical`) sopra `api/campioni/<op>`: nessuna regola sui nomi in JS.
- «Condition» ha un `datalist` delle etichette usate; «Set condition…» la assegna ai file spuntati; le condizioni e i tempi proposti sono gialli finché non si preme «Confirm conditions» (stato della pagina: le voci già in `NB.campioni` al montaggio contano come confermate).
- Il `select` «Reference condition» (con ≥ 2 condizioni; default: la prima chiamata ref…/reference/control/no gas) sta in `NB.gruppi_opts.reference`; gli altri campi di `NB.gruppi_opts` (`min_rep_frac`, `min_area`, `factors`) si conservano così come sono. `onChange` dà alla pagina, al montaggio e a ogni modifica, `{campioni, reference, conditions}` con il `campioni/2` canonico (`tempo_min` del Rust). Un file senza voce in `NB.campioni` prende le proposte dai nomi (anche se la sessione v1 della vista Dati ha una voce senza condizione), salvo un tempo già confermato.
