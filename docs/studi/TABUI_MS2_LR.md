# Tabui: le MS2 a bassa risoluzione abbinate da sé ai TP

Studio dell'Architetto per la richiesta di Federico: in bassa risoluzione (SCIEX 3200 QTRAP, per ora solo `.wiff` + `.wiff.scan`) i file MS2 sono iniezioni separate dal full scan, ognuna con 2-3 esperimenti di ione prodotto; Tabui deve capire da sé a quale riga (TP, genitore, ione inatteso) appartiene ciascuno, mostrare l'abbinamento, farlo correggere e riportare ambiguità e MS2 non abbinate. Prompt delle chat DEV: `docs/studi/PROMPT_TABUI_MS2_LR/`. Numeri veri (m/z, tempi, nomi dei campioni): solo nel repository privato, `scratchpad/consegne/TABUI_MS2_LR_DATI.md` e `scratchpad/tabui_ms2_lr_truth.json`.

## 1. Cosa c'è già
- **Pool delle MS2 di una riga** (`crates/mzlab-feat/src/legacy/lr/ms2.rs`): `pool` prende, in ogni file `ms2`, le scansioni di livello 2 con precursore entro `tol_da` da `mz_x` o `mz` e tempo dentro `window(e)` (ms2.rs:62: dal `lo` più basso all'`hi` più alto delle celle full scan della riga, ±0,05 min); classi di 0,3 Da, al più 6 frammenti; `ms2` annota «shared with the parent» / «parent fragment + Δ». L'abbinamento quindi esiste già, ma è implicito, riga per riga, solo nel `detail`, e nessuno lo vede.
- **Piano** (`lr/plan.rs:190`, `plan_json`): i file `ms2` sono elencati a parte; `finder-lr.js` ne apre l'indice dopo le colonne (`cols = [...p.lc, ...p.ms2]`). La fase «ms2» del worker (`finder-lr.js:100`) oggi è solo un messaggio.
- **Tipo del file** (`legacy/sample.rs:143`, `kinds`): `ms2` se ha almeno 5 scansioni di livello 2 e meno di 20 di livello 1. Un campione MS2 di un `.wiff` lo è già.
- **Avvio automatico** (`TP_Mine/js/20_tpmine.js:273`): con `only` (apertura di Tabui) tiene solo `full` e `hr`: le MS2 aperte nella scheda Dati restano fuori finché l'utente non preme «Reload from Data».
- **`.wiff` nativo**: `wiff::Experiment` ha livello e precursore (`wiff/mod.rs:80`), la polarità viene dal segno di `IS`; l'energia di collisione NON entra nell'indice (`wiff/lettore.rs:266`, `wiff/run.rs:60`; `mzmlwrite.rs:281` scrive il precursore senza `<activation>`), mentre `metodo/1` la ha per esperimento (`ce_eV`). Gli mzML di msconvert degli stessi file la portano.
- **HR** (`legacy/ms2.rs`, `select`): DDA nella stessa iniezione, precursore entro 10 ppm e tempo dentro il picco dello stesso file: un caso più semplice, da non copiare. Si riusano invece `read_scans` (lettura dei picchi dall'indice) e `similarity::pair_components` (frammenti uguali / spostati / perdite comuni, tolleranza fissa `TOL_HR` = 0,005 a similarity.rs:9).
- **Livello** (`lr/summary.rs:17`): in LR le MS2 non cambiano il livello (5/4).
- **Picchi di una traccia**: `lr/peak1d.rs::find_peak` (picco attorno a un tempo, S/N, bordi; già usato per l'MRM) e `mzlab_core::peaks::detect_peaks` (tutti i picchi).

## 2. Cosa dicono i file veri del 3200
Serie B del repository privato: `FullMass.wiff` (7 campioni, full scan), `MS2.wiff` (3 campioni), `MRM.wiff` (13). Ogni campione MS2 ha 2 esperimenti di ione prodotto (codice 11) per tutta la corsa (0-20 min), CE 10 eV, positivo; i metodi `.dam` del corso ne hanno fino a 3, e uno ha lo stesso precursore a tre CE.
- La traccia di un esperimento (TIC delle scansioni di prodotto contro il tempo, già nell'indice: nessun picco da leggere) mostra l'eluizione dello ione: l'apice cade entro 0,07 min dall'apice dello stesso m/z nel full scan dello stesso tempo di campionamento. La deriva tra iniezioni è piccola ma non nulla.
- Su 6 esperimenti: 3 puntano a ioni con un picco full scan forte che **non sono righe del Dig** (né candidati né inattesi); 1 cade 0,2 min dopo il genitore (più leggero: frammento in sorgente o TP che coeluisce); 1 ha due picchi (uno nel fronte); 1 ha lo stesso m/z di una riga inattesa ma **nessun segnale** al suo tempo.
- **Errore di oggi**: a quella riga `pool` attacca 407 scansioni di rumore, perché una sua cella ha un picco largo 11 min e `window` usa il minimo e il massimo di tutte le celle. Il genitore non ha MS2 in questi file (l'albero dice «no MS2 of the parent»).

## 3. Modello
- **Esperimento MS2** = (file, polarità, precursore arrotondato a 0,1, CE arrotondata a 0,1 o assente). Si costruisce dalle righe dell'indice (`Scan.level == 2`, `precursor`, `collision_energy`, `polarity`, `rt`, `tic`): vale per i file `ms2` e anche per le scansioni di livello 2 dentro un file `full` (stessa logica per full scan, MS2 e MRM). Due esperimenti uguali dello stesso file si uniscono.
- **Traccia** = (rt, tic) delle sue scansioni; **picchi della traccia** = `detect_peaks` sopra S/N 5 e dopo `rt_min` («Ignore RT before»).
- **Abbinamento** (`link`) = esperimento + picco della traccia + riga, con Δm/z, ΔRT, deriva, stato e motivo. Le scansioni di una riga sono quelle dei suoi abbinamenti, dentro i bordi del picco della TRACCIA (non più `window`).

## 4. Regole di abbinamento (in ordine)
1. **Polarità**: un esperimento di polarità diversa da quella dei file full della serie (0 = non detta, vale) non si abbina: motivo `polarity`.
2. **m/z**: righe compatibili = genitore, candidati, inattesi (e bersagli MS2, §6) con |P − `mz`| ≤ 0,5 o |P − `mz_x`| ≤ 0,5 Da (P = precursore impostato dallo studente, che può aver scritto il valore calcolato o quello letto sullo spettro, spostato di +0,14…+0,38). 0,5 Da = finestra di Q1 a risoluzione unitaria; costante `PREC_TOL`, mostrata nella tabella.
3. **Isotopo**: se P è a +1,003 o +2,007 (±0,5) da una riga più forte allo stesso tempo e nessuna riga compatibile è più vicina, stato `isotope` (mostrato, non messo nel pool).
4. **Tempo**: tempo atteso = apice della riga nel file full con lo stesso tempo di campionamento del file MS2 (se c'è ed è un picco), altrimenti `ref_rt`; più la deriva del file MS2. Si cerca il picco della traccia con `find_peak(rt, tic, atteso, rt_tol_min, 3, 3)`: abbinato se S/N ≥ 3 e |apice − atteso| ≤ `rt_tol_min`.
   Acquisizione a finestra (IDA, EPI programmato: meno di 10 scansioni dell'esperimento fuori da ±`rt_tol_min` dal tempo atteso): la traccia non ha un fondo da cui stimare il rumore, quindi il «picco» sono tutte le scansioni dentro ±`rt_tol_min`, con `why` «acquired only around this time».
5. **Deriva** di un file MS2 = mediana di (apice della traccia − tempo atteso) sugli abbinamenti con una sola riga compatibile e S/N ≥ 10; si applica con almeno 2 abbinamenti, limitata a ±`rt_tol_min`, e si riporta. Si calcola prima, poi si rifà il passo 4.
6. **Un picco, più righe**: la riga col |ΔRT| minore (a parità, |Δm/z| minore) prende il picco con stato `ambiguous` e l'elenco delle `alternatives` (D2).
7. **Frammento in sorgente**: un picco della traccia con P più leggero del genitore e apice entro 0,5·`rt_tol_min` dall'apice del genitore (la stessa soglia di `insource` delle righe) non va a un TP: motivo `parent_rt` (D4), salvo che la riga compatibile sia già marcata `insource`/`isf` (allora è abbinata con stato `isf`).
8. **CE**: ogni CE è un esperimento: una riga può avere più abbinamenti, uno per CE (D3). Il `detail.ms2` di oggi resta quello della CE con più scansioni; le altre CE stanno in `detail.ms2_links`.
9. **Genitore**: gli abbinamenti al genitore danno la sua MS2 (albero supposto, confronto dei prodotti).
10. **Correzioni**: l'utente sceglie per un picco una riga o «nessuna»; la chiave è (nome del file, P ±0,05, CE, apice ±`rt_tol_min`/2): una correzione che non trova più il suo picco resta elencata come `stale`. Le correzioni battono le regole 2-7, mai la 1.
11. **Non abbinati** (`unmatched`, uno per esperimento o per picco della traccia senza riga): `no_row` (nessuna riga a quel m/z), `no_row_at_rt` (righe a quel m/z ma ad altri tempi), `no_signal` (riga compatibile ma nessun picco della traccia al suo tempo: è il caso dell'errore di §2), `front` (prima di `rt_min`), `parent_rt`, `polarity`, `stale`.

## 5. Contratto `ms2_links/1`
Nel riassunto di `api.run` (chiave `ms2_links`, solo percorso Rust) e restituito da `relink`:
```
{"schema":"ms2_links/1","prec_tol":0.5,"rt_tol":0.25,
 "experiments":[{"file":"x.mzML","time":45,"polarity":1,"precursor":0.0,"ce":10,"scans":558,"rt":[0.01,20.0],"drift":0.03,
                 "peaks":[{"apex":0.0,"lo":0.0,"hi":0.0,"sn":0.0,"scans":0}]}],
 "links":[{"exp":0,"peak":0,"row":12,"state":"auto|ambiguous|isotope|isf|user","dmz":0.12,"drt":0.03,"alternatives":[34],"why":"..."}],
 "unmatched":[{"exp":1,"peak":null,"reason":"no_signal","rows":[100028],"why":"..."}],
 "parent":{"has_ms2":false}}
```
- `why` e i testi sono in inglese (Tabui non ha chiavi `lang/`). `rows[].ms2` nel riassunto = numero di abbinamenti della riga (0 se nessuno). `detail.ms2_links` = gli abbinamenti della riga con i loro frammenti; `detail.ms2` invariato nella forma (scansioni, precursore, CE, frammenti).
- Correzioni in ingresso a `relink`: `[{"file":"x.mzML","precursor":0.0,"ce":10,"apex":0.0,"row":12|null}]`.

## 6. Bersagli MS2 (D1)
Lo studente acquisisce una MS2 perché ha già visto quello ione: se un picco della traccia ha motivo `no_row` o `no_row_at_rt`, si cerca nella tabella allineata la riga con |m/z − P| ≤ 0,5 e |RT − apice| ≤ `rt_tol_min` (area massima) e diventa una riga «MS2 target m/z P @ RT» (`kind` `unexpected` con `ms2_target: true`, id da 200001), con i criteri e il punteggio degli inattesi ma senza il loro filtro (`max_unexpected`, criteri). Senza riga nella tabella resta `unmatched` con motivo `no_feature`. A parte va capito perché il Dig non elenca ioni così forti (indizio: `lr/rows.rs:274` scarta come inatteso ogni ione entro `tol_da` da un m/z di ipotesi, anche quando l'ipotesi è a un altro tempo): diagnosi nel pacchetto M3, correzione solo con il sì del Coordinatore.

## 7. Interfaccia (Tabui, solo inglese)
- All'avvio automatico entrano anche i file MS2 della scheda Dati, se la serie non è HR.
- Risultati LR: una scheda «MS2 matches» con una vera `<table>` (file, tempo, precursore, CE, apice della traccia, riga abbinata, Δm/z, ΔRT, stato come testo e badge, motivo) e per ogni picco un `<select>` (righe entro ±1,5 Da ordinate per |ΔRT|, più «None: not this ion»). Le non abbinate sono nella stessa tabella, con il motivo. Un cambio chiama `relink` (nessun nuovo Dig) e aggiorna la tabella, i conteggi e il dettaglio aperto.
- Dettaglio di una riga: «From F, precursor P, CE c, MS2 peak at t min (Δ from the full scan); matched automatically» con il pulsante «Change» che porta al `<select>`; con più CE, uno spettro per CE.
- Correzioni ricordate nel browser (`localStorage` `qqq.tabui.ms2links`, chiave = file ordinati + formula del genitore) e scritte nelle esportazioni (JSON: `ms2_links`; Excel: foglio «MS2 matches»).

## 8. Casi ambigui
| Caso | Esito |
|---|---|
| Due righe allo stesso m/z e tempi vicini (isomeri non separati, candidato + inatteso) | regola 6: la più vicina, `ambiguous`, alternative mostrate |
| Due picchi nella traccia, due isomeri | uno per riga (regola 4) |
| Due picchi, una riga | il più vicino è abbinato, l'altro `no_row_at_rt` (o bersaglio) |
| Picco al tempo del genitore, P più leggero | `parent_rt` |
| P = isotopo di una riga più forte | `isotope` |
| Stesso P, CE diverse | un abbinamento per CE |
| Riga con P giusto ma traccia piatta | `no_signal`, nessuna scansione nel pool |
| File MS2 senza tempo di campionamento | tempo atteso = `ref_rt` |
| Polarità diversa | `polarity` |
| MS2 del genitore assente | `parent.has_ms2: false`, l'albero lo dice già |

## 9. Domande di prodotto (mandate al Coordinatore; i prompt usano la scelta (a))
- D1 MS2 su uno ione che Tabui non elenca: (a) riga «MS2 target» con i suoi criteri; (b) solo elenco dei non abbinati.
- D2 Due righe compatibili: (a) alla più vicina, marcata ambigua; (b) a nessuna finché l'utente sceglie.
- D3 Stesso precursore, CE diverse: (a) spettri separati; (b) sommati.
- D4 Picco al tempo del genitore: (a) segnalato, non attaccato a un TP; (b) attaccato.
- D5 Correzioni: (a) ricordate nel browser per quell'insieme di file ed esportate; (b) solo per la sessione.
- D6 Una MS2 abbinata con almeno 2 frammenti spiegati dal genitore (uguali o spostati) alza la riga al livello 3: (a) no, resta 4/5 con la prova scritta; (b) sì.

## 10. Pacchetti
| Pacchetto | Cosa | Dipende da |
|---|---|---|
| M0 | Correzione del difetto esistente: la finestra di `ms2.rs::window` attacca rumore a una riga (scansioni MS2 solo vicino a `ref_rt` e solo con segnale nella traccia) | fatto: finestra attorno a `ref_rt` e controllo del segnale della traccia in `ms2.rs`; prove in `lr_dig.rs` |
| M1 | CE dei `.wiff` nell'indice e nell'mzML (`<activation>`) | nulla (parallelo a M2) |
| M2 | Motore `lr/ms2link.rs`, pool dagli abbinamenti, `ms2_links/1`, `relink`, worker | fatto: `ms2link.rs` e `ms2link/`, `ms2_link`/`relink` in `finder-lr`, `ms2_relink` nel worker; prove `lr_ms2link.rs` e `lr_ms2link_dati.rs`; `finder-lr` 610 748 B (prima 587 098) |
| M3 | Bersagli MS2 (§6) e diagnosi delle righe mancanti | M2 unita |
| M4 | Interfaccia: avvio con le MS2, scheda «MS2 matches», correzioni, esportazioni, e2e | M2 unita (parallelo a M3) |
| M5 | Confronto col genitore come in HR (`pair_components` con tolleranza in Da) e D6 | M4 unita |

## 11. Trappole
- Il nome del composto del corso, i suoi TP e i loro m/z non vanno nel repository pubblico: i test con dati veri leggono la verità da `scratchpad/tabui_ms2_lr_truth.json` del repository privato e saltano senza `MZLAB_DATI`.
- `finder-lr` sta sotto 600 000 B (`tools/build_feat_wasm.sh`), il motore sotto 1 703 936 B con il margine per DEV-09/10/11 dello studio formati (M1 aggiunge poche centinaia di byte: misurarli).
- `tests/golden/finder_lr_contratto.json` confronta le chiavi del `detail`: le chiavi nuove (`ms2_links`) si tolgono nel test come `specificity`.
- Funzioni ≤ 80 righe, un modulo di test in un file a sé (`#[path]`), mai `cargo fmt` sull'intero repository.
