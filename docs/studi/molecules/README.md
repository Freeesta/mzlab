# Database interno `molecules` (studio)

Decisione di Federico: la lista di nomi di Tabui smette di essere di Tabui e diventa un database interno dell'app, `molecules`, con più informazioni per molecola e usato da più strumenti. Questo studio fissa cosa c'è oggi, il contratto `molecules/1`, cosa si cancella e il piano a PR.

## 1. Cosa c'è oggi

| Elenco | File | Chi lo legge e come | Campi usati | Note |
|---|---|---|---|---|
| Lista di Tabui | `TP_Mine/data/composti.csv` (625 righe, 109 KB) | Tabui, campo «Name» del genitore: `csvRows`, `getList`, `suggest` in `TP_Mine/js/20_tpmine.js` (JS, nessun Rust) | `names` (sinonimi separati da `;`), `neutral_formula`, `smiles`; `mass` letto ma non usato; `inchikey` non usato | Sola lettura, nessun dato dell'utente. Arriva con mzFinder (`ctx().files`). Letto anche da test Rust e Python (sotto). |
| Sostanze per lo screening HR | `mzlab/web/sostanze_hrms.csv` (645 righe) | Vista Dati, screening target HR: `liste-target.js` (`fetch`) + `liste-target-core.js` (lettore CSV e calcoli in JS) | nomi inglese e italiano, sinonimi, formula, massa, m/z di tre addotti (derivati), SMILES, InChIKey, CAS, CID, quattro classi, `check` | 43 molecole in comune con la lista di Tabui per InChIKey intero, nessun conflitto di formula, massa o SMILES. Per InChIKey-14 le comuni sarebbero 44 e 8 «diverse»: sono stereoisomeri, quindi la chiave è l'InChIKey intero. |
| `liste/1` tipo `suspects` | `crates/mzlab-feat/src/liste/`, `docs/schemi/liste-1.json` | Nessuna interfaccia ancora: solo lettore CSV e JSON canonico | `nome`, `formula`, `massa` | Duplica, in piccolo, i campi di una molecola. |
| `liste/1` tipi `transformations`, `adducts` | idem | mzFinder (trasformazioni); addotti senza interfaccia | `nome`, `delta`, `carica`, `ripetizioni_max` | Non sono molecole: restano dove sono. |
| Liste di m/z di riferimento | `mzlab/web/contaminanti.json` + liste dell'utente in IndexedDB `mzlab_lab`, store `liste_utente` (`liste.js`) | Vista Dati, LR e HR | `mz`, `name`, `adduct`, `formula`, `pol` | Sono IONI osservati (spesso solo m/z), non molecole: restano liste di ioni. |
| Indice dei sospetti `.mzsus` | `tools/genera_sospetti.py`, scaricato da mzFinder | Ipotesi H1 di mzFinder | InChIKey-14, formula, massa, conteggi | Centinaia di migliaia di righe, fonte esterna fissata: resta separato. |
| Liste regolatorie | `crates/mzlab-feat/src/attributio/reglists/` | Attributio | catalogo delle liste, voci per InChIKey | Fonti esterne con versione: restano separate. |
| Parent di Tabui e della Pipeline | campi `tp-name`, `tp-mol`, `tp-smi` (`20_tpmine.js`), `pipe-pf`, `pipe-psmi` (`34_pipeline_run.js`) | Tabui, Pipeline | nome, formula, SMILES | Compilati a mano o dalla lista di Tabui; nulla è salvato fra le sessioni. |

Lettori di `composti.csv` fuori da Tabui (solo prove e strumenti): `crates/mzlab-mol/tests/smiles.rs` e `stereo.rs`, `crates/mzlab-feat/tests/grafo_record.rs`, `crates/mzlab-chem/examples/precalc_reatt.rs`, `tools/struttura_dato/confronto.py` (dati d'oro RDKit riga per riga), `tools/genera_composti.py` (lo genera), `tests/test_composti.py`, `tests/test_guardie_regole.py`, `tests/test_tabui_inglese.py`.

Doppioni: nome, sinonimi, formula, massa, SMILES e InChIKey della stessa molecola stanno in due file (43 molecole) e in due lettori JS; `liste/1 suspects` ripete gli stessi campi; i m/z degli addotti in `sostanze_hrms.csv` sono numeri derivati salvati come dati.

Dati dell'utente da migrare: nessuno. La lista di Tabui non si modifica dalla pagina e nulla del genitore è salvato; le liste di ioni dell'utente (`liste_utente`) non cambiano.

## 2. Contratto `molecules/1`

Un record = una molecola. Campi in italiano, valori in inglese, come `liste/1`.

| Campo | Tipo | Regola |
|---|---|---|
| `id` | stringa | `ik:<InChIKey intero>` se c'è l'InChIKey; altrimenti `u:<primi 12 hex dello SHA-256>` del JSON canonico del record alla nascita. Non cambia mai, neanche se il record si modifica. |
| `nome` | stringa non vuota | Nome mostrato. |
| `sinonimi` | stringhe | Ordinate, senza doppioni (maiuscole ignorate), senza il `nome`. |
| `formula` | stringa | Formula della specie in ordine di Hill, senza segno di carica. Nelle interfacce di Tabui si chiama «Molecular formula» (mai «formula» da sola). |
| `carica` | intero | 0 per le molecole neutre; diverso da 0 per le specie con carica permanente. |
| `massa` | numero | Monoisotopica della specie (Da), CALCOLATA dal motore dalla formula e dalla carica (elettrone compreso, `elements.rs`), mai scritta a mano; nel CSV è una colonna di controllo: se c'è e differisce di più di 0,0005 Da la riga è sbagliata. |
| `smiles` | stringa o null | Come scritto (nessuna canonicalizzazione nella prima versione). |
| `inchikey` | stringa o null | 27 caratteri, maiuscolo. |
| `addotti` | stringhe | Nomi di addotti preferiti, in ordine (`[M+H]+`, ...), dalla tabella interna o da una lista `liste/1 adducts` dell'utente; vuoto = nessuna preferenza. |
| `raccolte` | stringhe | Gruppi dell'utente (per esempio una lista di sospetti, un corso): sostituiscono `liste/1 suspects`. |
| `classi` | stringhe | Etichette chiave=valore (`effect:stimulant`, `chemical:amphetamines`, `role:active_principle`, `psychoactive:yes`). |
| `esterni` | oggetto | `cas`, `pubchem_cid` (stringhe), solo se noti. |
| `proprieta` | array | `{chiave, valore, unita, fonte, condizioni}`: dati sperimentali o calcolati (`rt` in `min`, `logp` senza unità, ...). Unità e fonte obbligatorie. |
| `note` | stringa | Testo libero (qui va il `check` di `sostanze_hrms.csv`). |
| `origine` | oggetto | `{tipo: "catalogo"\|"utente"\|"importata", catalogo: ["tabui","hrms"] (solo per il catalogo), file (nome del file importato), creato, modificato (data ISO)}`. |

Documento: `{schema: "molecules/1", molecole: [record ordinati per id]}`. JSON canonico come `liste/1`: chiavi in ordine alfabetico, array ordinati dove la regola lo dice, numeri con la stessa scrittura; `digest()` = SHA-256 dei byte canonici, entra in `prov/1` quando un passo usa le molecole.

CSV (import ed export): separatore `;` (tabulazione o virgola se l'intestazione non ha `;`, come `liste/1`), righe vuote e `#` saltate, virgolette doppie. Colonne nell'ordine canonico: `id;name;synonyms;molecular_formula;charge;monoisotopic_mass;smiles;inchikey;adducts;collections;classes;cas;pubchem_cid;notes;origin;catalog`, poi una colonna per proprietà `prop:<chiave>[<unità>]` con la sua `prop:<chiave>:source`. Liste dentro una cella separate da `|`.

In import l'intestazione dà le colonne in qualsiasi ordine; obbligatori `name` e `molecular_formula`; `id` vuoto = assegnato. Ogni riga sbagliata dà un errore (riga, chiave `error.molecules.*`, testo) e il file è rifiutato se ce n'è uno, come `liste/1`.

Livelli: catalogo (sola lettura, nel sito) < importata < utente. Lo stesso `id` in un livello più alto sostituisce il record intero; «modifica» di una molecola del catalogo crea la copia dell'utente con lo stesso `id`; «elimina» di quella copia fa tornare quella del catalogo.

Ricerca (una sola funzione, Rust): `cerca(q, limite=8, ppm=10)`:
- `q` numerico: `|m − massa| ≤ massa·ppm·1e-6`, ordine per scarto, poi ordine del catalogo.
- `q` formula valida (la legge `elements.rs`, almeno un C o due elementi, per non scambiare un nome corto per formula): formula di Hill uguale; poi anche la ricerca per testo.
- testo (≥ 2 caratteri, minuscole): su `nome` e `sinonimi`, punteggio 0 uguale, 1 inizia con, 2 inizio di parola (prima un carattere fra spazio, `-`, `(`, `,`), 3 contiene; parità per lunghezza del `nome`, poi per ordine stabile del catalogo. È l'algoritmo di `suggest` di Tabui di oggi, identico.
- Risultato: `[{id, nome, formula, carica, massa, smiles, addotti, punteggio}]`; `get(id)` dà il record intero.

Dove sta: `crates/mzlab-core/src/molecules/`, sul modello di `campioni/` (`route(op, input)`, `crates/mzlab-wasm/src/molecules.rs`, caso `lr_molecules` in `rust-worker.js`, rotte `api/molecules/<op>`). Così lo servono il motore della vista Dati (Infusione MSn, screening HR) e Tabui (che chiama già `api/campioni/...`) senza toccare i `.wasm` di mzFinder. Il catalogo NON va dentro il `.wasm`: è un file statico (`mzlab/web/molecole/catalogo.csv`) che la pagina passa al motore una volta (`carica`).

Archiviazione delle molecole dell'utente: browser = IndexedDB `mzlab_lab`, store nuovo `molecole` (keyPath `id`, `DB_VER` 4 in `qqq-db.js`); app desktop = un file JSON canonico nella cartella dei dati dell'utente. Mai nel repository; nel repository solo schema, catalogo pubblico ed esempi INVENTATI.

## 3. Un solo posto per ogni informazione

- Molecola (nomi, formula, SMILES, InChIKey, classi, proprietà): solo `molecules`. Gli altri puntano per `id`.
- Addotti: definizioni solo nella tabella interna e in `liste/1 adducts`; una molecola dice solo quali preferisce, per nome. I m/z si calcolano, non si salvano.
- Sospetti dell'utente: una raccolta di `molecules`. `liste/1` perde il tipo `suspects` (nessuna interfaccia lo usa ancora); `from_csv("suspects", …)` diventa l'import di molecole con la raccolta uguale al nome del file.
- Restano separati, perché sono altro: trasformazioni (`liste/1`), liste di ioni di riferimento (`contaminanti.json`, `liste_utente`), indice `.mzsus`, liste regolatorie, librerie di spettri. Possono rimandare a una molecola per InChIKey in un secondo tempo.

Si cancella (ognuno nella PR che sposta il suo ultimo lettore):
- `TP_Mine/data/composti.csv` e `tools/genera_composti.py` (M3): il catalogo ne contiene le righe con `catalog = tabui`, nello stesso ordine.
- `csvRows`, `getList`, `suggest` di `20_tpmine.js` (M3).
- `mzlab/web/sostanze_hrms.csv`, `loadBuiltinList` di `liste-target.js` e il lettore CSV con i calcoli di `liste-target-core.js` (M5).
- tipo `suspects` di `liste/1` (schema, `from_csv`, test) (M6).

## 4. Piano a PR (in serie; M1-M3 nella stessa chat DEV perché toccano gli stessi file)

| PR | Cosa | File | Prove |
|---|---|---|---|
| M1 motore | struttura, CSV, JSON canonico, livelli, ricerca, rotte; catalogo generato una volta dai due CSV | `crates/mzlab-core/src/molecules/*`, `crates/mzlab-wasm/src/molecules.rs`, `rust-worker.js`, ponte delle rotte, `docs/schemi/molecules-1.json`, `mzlab/web/molecole/catalogo.csv`, `tools/genera_molecole.py` (una tantum), `docs/agenti/molecules.md` (riga in AGENTS §5) | sotto, punti 1-4 |
| M2 archivio e gestione | store `molecole`, file desktop, finestra «Molecole» (cerca, aggiungi, modifica, elimina, importa ed esporta CSV o JSON), componente di ricerca riutilizzabile che riceve i testi dal chiamante (Tabui gli passa l'inglese) | `qqq-db.js`, `molecole-ui.js` (modulo ES, `@ts-check`), `lang/it.js`, `lang/en.js`, `crates/mzlab-desktop` | migrazione IndexedDB 3→4 senza perdite, e2e della finestra, axe, tastiera |
| M3 Tabui | il campo «Name» chiama `api/molecules/cerca`; si cancellano lettore e lista; i test e gli strumenti che leggevano `composti.csv` leggono il catalogo filtrato su `catalog = tabui` | `20_tpmine.js`, indice dei file di mzFinder, i lettori elencati in §1 | e2e: stesse proposte di prima per le stesse parole |
| M4 Infusione MSn | campo di ricerca nella scheda «Fragmentation graph»: la scelta compila «Ion formula» = formula + primo addotto (`[M+H]+` se vuoto) | file della chat Infusione MSn | e2e con una molecola inventata dell'utente |
| M5 screening HR | `liste-target` prende le molecole (raccolta o catalogo `hrms`), m/z dal motore | `liste-target*.js` | stessi m/z di prima su tutte le righe (≤ 1e-5) |
| M6 sospetti | `liste/1` senza `suspects`; import come raccolta | `crates/mzlab-feat/src/liste/*`, schema, `liste.md` | giro CSV dei sospetti sintetici → molecole |
| M7 altri | Pipeline (`pipe-pf`, `pipe-psmi`), Attributio, Flow con lo stesso componente | — | — |

Soglie per ogni PR: `tools/controlla.py` (funzioni ≤ 80 righe, file ≤ 500, doc ≤ 25 KB, righe di doc ≤ 600 caratteri). Tetti: M1-M7 non toccano i `.wasm` di mzFinder (table 447 688 / 512 000, HR 1 118 979 / 1 212 416); il `.wasm` del motore principale cresce: M1 misura prima e dopo e lo scrive nella PR.

## 5. Prove

1. Parità con Tabui: prima di M3, uno script lancia con node le funzioni `csvRows`/`getList`/`suggest` di oggi (estratte da `20_tpmine.js` per segnaposto) su tutte le domande «prefisso di 2, 3 e 5 lettere di ogni nome e sinonimo» più 100 sottostringhe interne, e salva il risultato come indici di riga (`tests/dati/molecole_tabui_oro.json`, solo numeri). Il test Rust di `cerca` sul catalogo filtrato `tabui` dà gli stessi indici nello stesso ordine.
2. Migrazione senza perdite: per ogni riga dei due CSV di oggi, il catalogo ha un record con gli stessi nomi (insieme), formula, SMILES, InChIKey, CAS, CID, classi e note; massa calcolata entro 0,0005 Da da quella scritta; nessuna riga persa né fusa per sbaglio (fusione solo per InChIKey intero: 625 + 645 − 43 = 1227 record).
3. Giro CSV: CSV canonico → JSON → CSV identico al byte; CSV in ordine qualsiasi → JSON uguale al byte; righe sbagliate → un errore per riga con la sua chiave.
4. Ricerca: massa con ppm al bordo (dentro a 9,99 ppm, fuori a 10,01), formula, testo; tutto su molecole INVENTATE (nomi come «Esempina», formule plausibili).
5. Nessun nome reale nei file nuovi: `tests/test_esempi_anonimi.py` e il controllo dei cognomi; il catalogo pubblico esiste già e resta l'unico file con nomi di composti.
6. Solo sul Mac di Federico: l'app desktop (file nella cartella dell'utente, giro salva e riapri) e la migrazione IndexedDB 3→4 su un profilo vero con liste dell'utente già salvate.

## 6. Decisioni prese
- Le sostanze dello screening HR entrano nel catalogo: un solo database, filtrato per `catalog` dove serve.
- La finestra «Molecole» si apre da una voce del menu dell'intestazione e dal link «Gestisci» sotto ogni campo di ricerca.
