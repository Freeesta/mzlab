# Lista di pesticidi e contaminanti ambientali, classi di struttura del catalogo

Studio e piano per due aggiunte ai dati di riferimento: (1) una lista integrata di pesticidi e contaminanti ambientali accanto a Keller 2008; (2) classi certe per i record `tabui` del catalogo delle molecole che non ne hanno.

## Dati di partenza
- `mzlab/web/contaminanti.json`: una lista (`keller`, 286 voci di fondo LC-MS, Keller 2008), espansa in ioni da `mzlab/chem/contaminants.py`; la usano `liste.js` (Calcolatrice, finestra della formula, annotazione dei picchi), `ricerca.js` (palette) e il controllo di massa in Rust (`massacc::refs::background`, ioni di fondo sempre presenti).
- `mzlab/web/molecole/catalogo.csv`: 1228 record; le classi `tipo:valore` (`chemical`, `effect`, `psychoactive`, `role`) vengono solo dal CSV `hrms`; 583 record `tabui` ne sono senza, nessun pesticida classificato.

## Fonti e licenze
| Fonte | Cosa offre | Licenza | Uso qui |
|---|---|---|---|
| PubChem (NCBI) | formula, SMILES, InChIKey calcolati da PubChem per CID | NCBI non pone restrizioni ai dati; i dati depositati possono averne, i descrittori calcolati no | **usata**: solo i descrittori calcolati |
| Nomi comuni ISO dei pesticidi, classi chimiche | nomi e famiglie chimiche | dati di fatto | **usati** (classi assegnate dalla struttura, non copiate da un compendio) |
| NORMAN SusDat | liste di sospetti con classi | licenza diversa per lista (molte su Zenodo con licenza aperta), da verificare una per una | **non usata**: da decidere per lista |
| US EPA CompTox | liste, classi, proprietà | dati federali di pubblico dominio, ma alcune liste includono dati di terzi | **non usata**: da verificare per lista |
| EU Pesticides Database | stato di approvazione, LMR | riuso con citazione (Decisione 2011/833/UE) | **non usata**: niente stato normativo nella lista |
| BCPC / Compendium of Pesticide Common Names | classi e nomi | diritti riservati | **non usata** |

## Scelte
- **Dove**: seconda lista integrata in `contaminanti.json` (stessa forma canonica di Keller: ioni con `name`/`name_en`, `cls`/`cls_en`, `formula`, `adduct`, `mz`, `pol`), più `tags`, `smiles`, `inchikey`. Così Calcolatrice, palette, finestra della formula e pannello Liste la vedono senza codice nuovo; con due liste integrate il pannello mostra le spunte «Liste attive».
- **Non è fondo**: la lista ha `"background": false` e il controllo di massa (Rust) la salta: un pesticida non è un ione che c'è in ogni corsa. Parità sito/app: lo stesso file e la stessa funzione Rust.
- **Classi**: `chemical:<famiglia>` e `use:<uso>` da un vocabolario chiuso con nomi IT/EN (`pesticides_meta.json`); la famiglia chimica è quella della struttura (es. spiroxamina = spirochetalammina, non morfolina).
- **Addotti**: ESI+ `[M+H]+`, `[M+Na]+`, `[M+NH4]+`; acidi (fenossiacidi, PFAS, bisfenoli…) `[M-H]-`; anfoteri entrambi.
- **Insegnamento**: una coincidenza di m/z con la lista è un candidato compatibile, mai un'identificazione (stesso testo di Keller).

## Classi di struttura del catalogo
Le classi d'uso o d'effetto dei 583 record `tabui` (es. «FANS», «antiepilettico») non si deducono dalla struttura e non vengono scritte. Si aggiungono solo classi `structure:` che il grafo SMILES mostra con certezza, con regole fisse (`tools/classi_struttura.py`): acido carbossilico, estere, solfonammide C-SO2-N, nitro su C o N, ammonio quaternario, presenza di F/Cl/Br/I (utile per il profilo isotopico). Il generatore del catalogo le aggiunge a ogni record con SMILES; 353 record `tabui` su 583 ne ricevono almeno una, gli altri restano senza.

## Da decidere (proprietario del prodotto)
- Allargare la lista con NORMAN SusDat o CompTox (centinaia di composti): serve la verifica della licenza lista per lista.
- Stato di approvazione UE e limiti massimi di residuo: fuori da questa lista (dati normativi, `LISTE_NORMATIVE.md`).
