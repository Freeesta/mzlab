# Interpretazione della feature table

## 1. Divisione del lavoro

- Rust calcola i controlli (`tabcheck/1`, nuovo modulo `mzlab_llm::tabcheck` sopra `mzlab-feat`: tabella, `table.stats`, `table.pca`, `table.kmd`, `classi/1`, `interp/1` dove ci sono).
- Il subagente interprete legge `tabcheck/1` e ≤ 50 righe scelte da Rust, scrive una `lettura/1` di tipo `table` con risultati, domande e azioni proposte (modifiche di piano o parametri, grafici).
- Nessuna identità come verità: in LR solo «m/z candidato a RT»; in HR candidati con livello (scala in `livelli.md`) e prove già presenti nella tabella.

## 2. Controlli (`tabcheck/1`)

| Controllo | Cosa misura Rust | Soglia di segnalazione | Azione tipica |
|---|---|---|---|
| dimensione | righe, colonne, file, quota di celle vuote per file | un file con < 50 % delle righe della mediana | chiedere se il file è un bianco o è fallito |
| bianchi | per riga rapporto mediana campioni / massimo bianchi | quota di righe con rapporto < 3 | `table.filter.blank_ratio` |
| contaminanti di sistema | righe presenti in tutti i file, bianchi compresi, con area quasi costante | ≥ 1 riga così tra le 20 più intense | escluderle o segnarle |
| QC: ripetibilità | CV delle aree nei QC per riga, distribuzione | mediana CV > 20 % o quota CV > 30 % sopra 25 % | `table.drift` o controllo dello strumento |
| QC: deriva di intensità | pendenza dell'area mediana dei QC contro l'ordine | variazione > 20 % fra primo e ultimo QC | `table.drift` |
| deriva RT | scarto RT delle righe comuni contro l'ordine | > 2 × FWHM mediana | `features.align.rt_warp`, `rt_gap` |
| duplicati | coppie con Δm/z < tolleranza e ΔRT < FWHM mediana | qualunque | errore di allineamento: `features.align.ppm` o `mode` |
| addotti | coppie coeluenti con differenze note: ESI+ [M+Na]+ − [M+H]+ 21,9819; [M+NH4]+ − [M+H]+ 17,0265; [M+K]+ − [M+H]+ 37,9559; ESI− [M+Cl]− − [M−H]− 35,9767; [M+HCOO]− − [M−H]− 46,0055 | correlazione delle aree ≥ 0,9 | raggruppare in famiglia (`mode=family`) |
| isotopi | coppie Δ 1,00336 (HR) o 1 Da (LR) coeluenti, rapporto compatibile con il numero di C | rapporto fuori dal 50–200 % dell'atteso | segnalare, nessuna correzione |
| frammenti in sorgente | coeluenti con rapporto di aree costante fra i file e m/z minore | correlazione ≥ 0,95 | raggruppare, chiedere il genitore |
| tracce lunghe | righe con larghezza > 5 × mediana | quota > 5 % | sanguinamento della colonna o contaminanti: filtro |
| serie omologhe | KMD (`table.kmd`) con ≥ 5 membri | qualunque | segnalare (polimeri, tensioattivi) |
| valori mancanti | struttura dei vuoti: casuali o per gruppo | vuoti concentrati in un gruppo | `features.gapfill` |
| struttura | PCA dei QC e dei campioni (`table.pca`): QC raggruppati, ordine di iniezione su un componente | QC sparsi o PC1 correlato con l'ordine (|r| > 0,7) | deriva, effetto di lotto |
| polarità e unità | colonne coerenti con l'`ispezione/1` | qualunque incoerenza | errore di piano |

Ogni controllo dà: `id`, `value`, `threshold`, `flag` (`ok|note|warn`), `rows` (≤ 20 id d'esempio), `proof` (numeri usati). Tetto del blocco: 6 000 token stimati.

## 3. Domande che l'interprete fa

- Che cosa cercava l'utente (prodotti di un genitore, differenze fra gruppi, contaminazione, qualità della serie)?
- Quali file sono bianchi, QC, standard, se i ruoli mancano.
- Se un gruppo di righe segnalate è atteso (per esempio un additivo della fase mobile).
- Se rifare la corsa con i parametri proposti o tenere la tabella com'è.

## 4. Uscita (`lettura/1`, tipo `table`)

```json
{"schema":"lettura/1","kind":"table","findings":[
 {"id":"F1","topic":"blank","statement":"23 of 412 rows are at least as intense in the blank as in the samples","evidence":["tabcheck.blank.value"],
  "confidence":"high","action":{"type":"set_param","step":"table.filter@1","name":"blank_ratio","value":3}},
 {"id":"F2","topic":"adduct","statement":"r12 (m/z 386.3) co-elutes with r4 (m/z 364.4); the difference fits [M+Na]+ − [M+H]+","evidence":["tabcheck.adducts.rows"],
  "confidence":"medium","action":{"type":"none"}}],
 "questions":[{"id":"Q1","text":"Is the 391.3 signal present in every file a known contaminant of your system?","options":["yes","no","not sure"]}],
 "charts":[{"ref":"qc:cv","why":"distribution of the QC CV"}],
 "numbers":["23","412","386.3","364.4","391.3"]}
```
- Al più 8 risultati, ordinati per effetto sul risultato dell'utente; ogni risultato ha prova e azione (anche `none`).
- `action.set_param` passa dalla politica e da `flow::check` come ogni altra modifica.
- I numeri in `numbers` devono comparire nell'ingresso; un numero assente fa fallire il controllo e la frase si segna «not verified» (come `bad_sentences` del tutor).

## 5. Riferimenti per i casi d'oro

I file sintetici (`tools/dati_sintetici.py`) contengono già ciò che i controlli devono trovare: un contaminante in ogni file con RT fisso, l'addotto sodico del genitore, i suoi isotopi, tre frammenti in sorgente a rapporto costante, prodotti che salgono e scendono nel tempo, uno spostamento costante di m/z dello strumento. `06-valutazione.md` li usa come risposta nota.
