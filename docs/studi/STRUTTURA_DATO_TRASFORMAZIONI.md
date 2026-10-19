# Struttura come dato: le trasformazioni genitore → prodotto

Seguito di `STRUTTURA_DATO.md` §3. Come si scrive una trasformazione con la mappatura atomica, così che il sito di modifica si segua lungo una catena di reazioni e si leghi all'MS/MS. Tutto **[ipotesi]** di formato salvo dove è scritto **[verificato]**. Il tipo `transform` del Flow (`docs/studi/WORKFLOW_MODULARI.md`, ramo `claude/workflow-modulari`, `contratto.schema.json` → `transform1`) è un sottoinsieme di questo record: stessi nomi dei campi.

## 1. Oggi [verificato, letto nel codice]
- **Regole** (`TP_Mine/data/regole_tp_2.json`, `regole_tp/2`, `hypo/rules.rs`): tre profili (`fotocatalisi`, `ozono_cloro`, `metabolismo_I_II`), 49 regole nella fotocatalisi; ognuna è un **cambio di formula** (`delta_formula`), una classe, precondizioni sui gruppi funzionali del genitore (`richiede`), `passi_max`, `livello_previsione`, `direzione_rt`, `fonte`. Nessun modello di reazione, nessun atomo.
- **Siti** (`chem/posizioni/`, `posiz/1`): per un Δ, gli atomi del genitore che possono prenderlo (`aromatic_oh`, `aliphatic_oh`, `n_oxide`, `s_oxide`, `demethyl`, `carbonyl`, `aromatic_cl`, `aromatic_br`), con la classe di simmetria e il prior f(−) come «Prev.».
- **Riga di un TP** (`interp/1`): Δ dal genitore, alternative, controllo isotopico, `drawing` (`structure`, `multicenter`, `bracket`; `zones` con supporto), indici nell'ordine dello SMILES del genitore.
- **Frammenti** (`frag/1`): formule dei due lati di un legame, indici nell'ordine del molfile di Ketcher.
Quindi oggi il legame genitore → TP è «stessa formula + Δ»; dove sta il cambio lo dicono `posiz/1` e `interp/1`, ognuno con il suo ordine degli atomi.

## 2. Il record `trasf/1`
```
{ "format": "trasf/1",
  "parent": "<sha256 del nucleo di mol/1 del genitore>",
  "product": "<sha256 del prodotto>",          // assente se si conosce solo il cambio (livello 3)
  "delta": "+O",                               // come parz/1 e le ipotesi
  "rule": "hydroxylation", "rule_set": "regole_tp/2:fotocatalisi", "rule_version": "…",
  "map": [[0,0],[1,1],…,[12,null]],             // [atomo del genitore, atomo del prodotto o null se perso]
  "added": [21],                               // atomi del prodotto senza genitore (l'O nuovo)
  "centre": [9],                               // atomi del genitore dove legami o H cambiano (il centro di reazione)
  "sites": [{"atom":9,"p":0.5},{"atom":10,"p":0.5}],   // dove può stare il cambio, con supporto (come posiz/1)
  "region": {"atoms":[9,10,11],"kind":"multicenter"}, // solo per un prodotto incerto, come regions di mol/1
  "level": "3",                                // docs/agenti/livelli.md
  "evidence": [{"kind":"ms2_fragment","mz":188.07,"atoms":[0,1,2,3,4,21],"on":"product"}] }
```
- **Riferimenti, non copie**: genitore e prodotto sono record `mol/1` a parte, indicati dall'hash. L'hash è lo SHA-256 del **nucleo** canonico (atomi, legami, stereo, gruppi), non di coordinate, proprietà o regioni: ridisegnare una molecola non ne cambia l'identità.
- **Mappatura sugli indici canonici** dei due record. Se uno dei due record cambia versione del canonico, la mappa si ricalcola dalla permutazione (`can` porta la versione).
- **Prodotto certo**: `product` + `map` completa + `centre`. **Prodotto incerto** (livello 3, «OH da qualche parte sull'anello»): niente `product`; `delta` + `sites` + `region`; la mappa è l'identità sugli atomi non toccati (implicita). Così non si inventa mai una struttura unica.
- **Catene** (genitore → TP1 → TP2): si compongono le mappe. Un atomo del genitore si segue fino all'ultimo prodotto; se a un passo diventa `null`, è perso. I `sites` di un passo successivo si riportano sugli atomi del genitore attraverso la mappa composta.
- **Stereo**: la mappa dice anche se un centro del genitore resta tale. Un `centre` su uno stereocentro rende la stereo del prodotto `unknown` salvo regola che dica altro (ritenzione/inversione): mai copiata in silenzio.

## 3. Collegamenti
```
Tabui (genitore, mol/1) ──► regole (regole_tp/2 oggi; modelli di reazione domani) ──► trasf/1 candidati
        │                                                                              │
        ├─ props: Fukui f(−) (reatt/1), solo indizio su S/N ──► sites.p «Prev.» ◄───────┤
        │                                                                              ▼
        └─ posiz/1 (classi di simmetria sugli indici canonici)            interp/1 (riga MS: Δ, isotopi, MS2)
                                                                                       │
                     frag/1 → evidence (sottostruttura del prodotto) ──► map ──► atomi del genitore ──► TPMOL
```
- **Regole di biotrasformazione** (alla BioTransformer, CTS, PubChem Transformations): domani una regola porta anche un **modello di reazione con atomi mappati** (centro, atomi aggiunti, legami cambiati), scritto da noi in un formato nostro a partire dalle fonti originali (articoli, tabelle pubblicate, manuali). Una regola senza modello resta un Δ con precondizioni, come oggi. Licenze: patRoon GPL-3 (`patRoon_TP.md`, verificato lì); BioTransformer, CTS e PubChem: licenze **da verificare** prima di prendere anche solo i dati delle regole.
- **Fukui e proprietà per atomo**: `props` del genitore, agganciati al `can` del genitore; diventano `sites[].p` a priori solo dove `FUKUI_COSA_DICONO.md` lo ammette (eteroatomi S, N), mai per ordinare le posizioni C–H.
- **interp**: `interp/1` legge `trasf/1` invece di ricostruire il Δ; il suo `drawing` diventa una vista di `sites`/`region` sugli indici canonici (stesse forme di `TPMOL`).
- **MS/MS**: un frammento spiegato (`frag/1`, attribuzione dei frammenti del motore MS2) è una sottostruttura del **prodotto**; la `map` la riporta sul genitore. Se tutti i frammenti che contengono l'atomo aggiunto contengono anche l'atomo 9 e mai l'11, il supporto di 9 sale: è la stessa logica delle zone di `interp/1`, ma sugli atomi e lungo le catene.
- **Disegno**: «Use this structure» produce un `mol/1` (con `regions` se c'è una parentesi o un multicentro) e, accanto al genitore, un `trasf/1` con `delta` e `region`.

## 4. Frammenti e perdite neutre come sottostrutture
- Un frammento = insieme di atomi di un record (`atoms`), legami tagliati (`cut`), H spostati, formula e m/z. Una perdita neutra = il complemento. Si scrivono sugli indici canonici, non su quelli di Ketcher.
- Una classe diagnostica (`classi/1`, `data/classi-1.json`) resta una regola su formule e perdite; se un giorno porta sottostrutture, usa lo stesso formato [ipotesi].

## 5. Cosa NON fare
- Non scrivere la mappa sugli indici dello SMILES o del molfile; non copiare la struttura dentro la trasformazione.
- Non dare un `product` a un TP di livello 3; non scegliere un sito «più probabile» e disegnarlo come certo.
- Non mettere i Fukui dentro `p` per le posizioni C–H.
- Non tradurre codice di BioTransformer o patRoon né prendere i loro file di regole senza aver letto la licenza.
