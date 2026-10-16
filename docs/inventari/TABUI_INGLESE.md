# Audit Read-Only: Migrazione «Tabui Solo Inglese» (T11)
## 1. Introduzione e Obiettivi
Il presente documento costituisce l'audit read-only esaustivo per la completa internazionalizzazione del modulo **Tabui** (noto internamente anche come *TP Mine* o *mzFinder*) e dei componenti collegati nel repository mzLab (`TP_Mine/`, `mzlab/`, `crates/mzlab-feat*`, `crates/mzlab-cli*`, file di lingua).

Storicamente, Tabui è stato concepito con etichette sintetiche in lingua inglese, mentre i testi esplicativi e di supporto (tooltip, popover `TPHELP`, messaggi contestuali) sono stati parzialmente bilingui (IT/EN tramite `TP_Mine/js/40_tabui_testi.js`), con alcune stringhe residue in italiano nella logica di calcolo, nelle regole di trasformazione, nei modelli di esportazione e in componenti specialistici (es. *Banco 2b* in Attributio).

**Obiettivi dell'audit:**
- Rilevare e censire sistematicamente ogni stringa italiana presente nell'interfaccia utente (UI), nelle tabelle, nei dialoghi modali, nei tooltip e negli output generati (CSV, Excel, report, CLI).
- Verificare rigorosamente la regola aurea **«Formula mai da sola»**, assicurando che ogni occorrenza di formula sia qualificata in modo non ambiguo come *Neutral formula*, *Ion formula*, *[M+H]+ formula*, ecc.
- Mappare integralmente le chiavi di localizzazione (`lang/` e `40_tabui_testi.js`) utilizzate da Tabui.
- Definire un piano d'azione operativo dettagliato per consentire al team frontend di completare la transizione a **«Tabui solo inglese»** senza regressioni funzionali o di test.
## 2. Statistiche e Riepilogo Generale
Il censimento ha identificato un totale di **132 occorrenze** rilevanti per la migrazione, suddivise come segue:

### Ripartizione per Ambito
| Ambito / Ambito applicativo | Conteggio occorrenze | Descrizione sintetica |
|---|:---:|---|
| **UI / Tabelle** | 14 | Intestazioni tabelle, titoli sezioni/modali, etichette bottoni, valori select e dati mostrati a video |
| **Tooltip / Messaggi** | 1 | Testi di avviso, messaggi di notifica/errore e prefissi di stato interattivi |
| **Esportazioni / CLI** | 5 | Colonne esportate (CSV, Excel), formati di output e metadati scambiati |
| **Chiavi i18n (Cataloghi)** | 112 | Dizionari italiani in `40_tabui_testi.js` (84 chiavi), `44_tp_riga.js` (1 chiave), e catalogo `lang/it.js` (27 chiavi) |
| **Totale complessivo** | **132** | **Perimetro completo Tabui & feature table** |

### Ripartizione per Priorità di Intervento
| Priorità | Conteggio | Criterio di classificazione |
|---|:---:|---|
| **Alta** | 14 | Elementi direttamente visibili all'utente nell'interfaccia primaria o scritti nelle colonne esportate |
| **Media** | 115 | Tooltip, testi di aiuto secondari nei popover `TPHELP`, messaggi contestuali e chiavi i18n |
| **Bassa** | 3 | Nomi interni di campi JSON / strutture dati non esposti direttamente a video |

## 3. Tabella Principale dell'Audit
La seguente tabella documenta ogni elemento censito, indicando file e riga, contesto, testo attuale, proposta in lingua inglese, livello di priorità e note tecniche per l'implementazione.

| File:Riga | Ambito / Contesto | Testo attuale (italiano) | Proposta in lingua inglese | Priorità | Note tecniche |
|---|---|---|---|:---:|---|
| `TP_Mine/js/20_tpmine.js:784` | Esportazioni/CLI | r.label ('forte', 'possibile', 'debole', 'progenitore') | LAB[r.label] \|\| r.label ('strong', 'possible', 'weak', 'parent') | Alta | Colonna esportata 'Verdict' in CSV ed Excel Ranking sheet: emette stringhe italiane grezze… |
| `crates/mzlab-feat/src/attributio/readers/feature_decisions.rs:21` | Esportazioni/CLI | livello, nome, identita, motivazione (colonne CSV) | level, name, identity, justification | Alta | Intestazioni di colonna in feature_decisions.csv usate per scambiare decisioni con il labo… |
| `TP_Mine/data/composti.csv:1` | UI/Tabelle | formula (intestazione di colonna) | neutral_formula (oppure 'Neutral formula') | Alta | Violazione della regola 'Formula mai da sola': intestazione nuda nel CSV dei composti prog… |
| `TP_Mine/data/regole_tp_2.json:6` | UI/Tabelle | "nome": "TiO2/UV: radicali ·OH, lacune h+, O2·−" | "name": "TiO2/UV: ·OH radicals, h+ holes, O2·−" | Alta | Descrizione leggibile del profilo di trasformazione mostrato in interfaccia. |
| `TP_Mine/js/52_attributio_banco.js:244` | UI/Tabelle | Banco 2b (aria-label) | Bench 2b | Alta | Accessibilità: attributo aria-label della sezione revisione. |
| `TP_Mine/js/52_attributio_banco.js:252` | UI/Tabelle | Banco 2b (intestazione modale/sezione) | Bench 2b | Alta | Titolo principale del pannello di revisione feature. |
| `TP_Mine/js/52_attributio_banco.js:257` | UI/Tabelle | Banco 2b (aria-label) | Bench 2b | Alta | Accessibilità: attributo aria-label della sezione revisione. |
| `TP_Mine/js/52_attributio_banco.js:257` | UI/Tabelle | Banco 2b (intestazione modale/sezione) | Bench 2b | Alta | Titolo principale del pannello di revisione feature. |
| `TP_Mine/js/52_attributio_banco.js:257` | UI/Tabelle | Close the Banco (aria-label) | Close Bench 2b | Alta | Accessibilità: aria-label del pulsante chiusura revisione. |
| `TP_Mine/js/52_attributio_banco.js:262` | UI/Tabelle | Close the Banco (aria-label) | Close Bench 2b | Alta | Accessibilità: aria-label del pulsante chiusura revisione. |
| `TP_Mine/py/tpmine/engine.py:205` | UI/Tabelle | "progenitore" | "parent" | Alta | Nome del progenitore emesso dal motore Python nella lista candidati. |
| `TP_Mine/py/tpmine/engine.py:235` | UI/Tabelle | "progenitore" | "parent" | Alta | Etichetta di riga emessa dal motore Python per la riga del progenitore. |
| `TP_Mine/py/tpmine/engine.py:270` | UI/Tabelle | "debole" | "weak" | Alta | Etichetta di verdetto emessa dal motore Python per candidati sotto soglia. |
| `TP_Mine/py/tpmine/score.py:97` | UI/Tabelle | "forte", "possibile", "debole" | "strong", "possible", "weak" | Alta | Verdetti di ranking calcolati dal motore di scoring Python. |
| `TP_Mine/js/40_tabui_testi.js:10` | Chiavi i18n | Livello di identificazione (scala di Schymanski): L5 compare col trattamento, L4 form… | Rimozione dizionario IT; la chiave 'ft.col.level' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.col.level' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:11` | Chiavi i18n | Previsione (Löffler et al. 2025): quanto la trasformazione è attesa, da 4 regola gene… | Rimozione dizionario IT; la chiave 'ft.col.pred' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.col.pred' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:12` | Chiavi i18n | Che cosa potrebbe essere: formula e trasformazione dal progenitore, o «m/z … (unexpec… | Rimozione dizionario IT; la chiave 'ft.col.identity' usa la corrispondente versione E… | Media | Chiave TT 'ft.col.identity' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:13` | Chiavi i18n | m/z misurato (apice). In bassa risoluzione è un candidato, non un'identificazione. | Rimozione dizionario IT; la chiave 'ft.col.mz' usa la corrispondente versione EN di T… | Media | Chiave TT 'ft.col.mz' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:14` | Chiavi i18n | Tempo di ritenzione dell'apice, in minuti. | Rimozione dizionario IT; la chiave 'ft.col.rt' usa la corrispondente versione EN di T… | Media | Chiave TT 'ft.col.rt' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:15` | Chiavi i18n | Area nei campioni trattati in ordine di tempo; i simboli sotto dicono com'è stata ott… | Rimozione dizionario IT; la chiave 'ft.col.trend' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.col.trend' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:16` | Chiavi i18n | Un simbolo per campione: com'è stata ottenuta l'area. Apri la riga, scheda Samples, p… | Rimozione dizionario IT; la chiave 'ft.col.cells' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.col.cells' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:17` | Chiavi i18n | Area più alta fra i campioni (unità del motore). Il valore intero è nell'aiuto della … | Rimozione dizionario IT; la chiave 'ft.col.maxarea' usa la corrispondente versione EN… | Media | Chiave TT 'ft.col.maxarea' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:18` | Chiavi i18n | Punteggio di ordinamento (0-100): quota dei criteri superati. Non è una probabilità. | Rimozione dizionario IT; la chiave 'ft.col.score' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.col.score' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:19` | Chiavi i18n | Errore di massa della formula proposta, in ppm. | Rimozione dizionario IT; la chiave 'ft.col.ppm' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.col.ppm' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:20` | Chiavi i18n | Differenza di RT dal progenitore, in minuti: un prodotto più polare esce prima. | Rimozione dizionario IT; la chiave 'ft.col.drt' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.col.drt' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:21` | Chiavi i18n | Somiglianza della MS2 con quella del progenitore: frammenti uguali, spostati, perdite… | Rimozione dizionario IT; la chiave 'ft.col.sim' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.col.sim' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:22` | Chiavi i18n | Numero di scansioni MS2 dentro il picco. | Rimozione dizionario IT; la chiave 'ft.col.ms2' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.col.ms2' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:23` | Chiavi i18n | Tempo di trattamento a cui l'area è più alta, in minuti. | Rimozione dizionario IT; la chiave 'ft.col.tmax' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.col.tmax' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:24` | Chiavi i18n | Forma della cinetica: cresce, passa per un massimo, scende. | Rimozione dizionario IT; la chiave 'ft.col.class' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.col.class' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:25` | Chiavi i18n | Parte della molecola dove la MS2 colloca la modifica. | Rimozione dizionario IT; la chiave 'ft.col.region' usa la corrispondente versione EN … | Media | Chiave TT 'ft.col.region' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:26` | Chiavi i18n | Posto nell'ordine del motore: livello, poi punteggio. | Rimozione dizionario IT; la chiave 'ft.col.rank' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.col.rank' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:27` | Chiavi i18n | ● rilevata: il picco è stato trovato dal rilevamento in questo campione. | Rimozione dizionario IT; la chiave 'ft.state.detected' usa la corrispondente versione… | Media | Chiave TT 'ft.state.detected' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:28` | Chiavi i18n | ◐ riempita: nessun picco trovato; l'area è stata riletta dove la feature era attesa. | Rimozione dizionario IT; la chiave 'ft.state.filled' usa la corrispondente versione E… | Media | Chiave TT 'ft.state.filled' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:29` | Chiavi i18n | ◌ mirata: cercata apposta (ipotesi o lista), non trovata dal rilevamento libero. | Rimozione dizionario IT; la chiave 'ft.state.targeted' usa la corrispondente versione… | Media | Chiave TT 'ft.state.targeted' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:30` | Chiavi i18n | ○ assente: nessun segnale. | Rimozione dizionario IT; la chiave 'ft.state.absent' usa la corrispondente versione E… | Media | Chiave TT 'ft.state.absent' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:31` | Chiavi i18n | Solo le righe che compaiono o crescono col trattamento. | Rimozione dizionario IT; la chiave 'ft.chip.changed' usa la corrispondente versione E… | Media | Chiave TT 'ft.chip.changed' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:32` | Chiavi i18n | Togli le righe trovate nel bianco o nel buio. | Rimozione dizionario IT; la chiave 'ft.chip.noblank' usa la corrispondente versione E… | Media | Chiave TT 'ft.chip.noblank' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:33` | Chiavi i18n | Togli le righe già presenti al t0. | Rimozione dizionario IT; la chiave 'ft.chip.not0' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.chip.not0' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:34` | Chiavi i18n | Solo le righe con almeno una MS2. | Rimozione dizionario IT; la chiave 'ft.chip.ms2' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.chip.ms2' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:35` | Chiavi i18n | Solo le righe su cui hai deciso un livello. | Rimozione dizionario IT; la chiave 'ft.chip.decided' usa la corrispondente versione E… | Media | Chiave TT 'ft.chip.decided' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:36` | Chiavi i18n | Nasconde le righe che hai segnato come non TP. | Rimozione dizionario IT; la chiave 'ft.chip.hidenottp' usa la corrispondente versione… | Media | Chiave TT 'ft.chip.hidenottp' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:37` | Chiavi i18n | Una nota tua su questa riga: resta nel browser con la decisione ed entra nelle esport… | Rimozione dizionario IT; la chiave 'ft.decided.note' usa la corrispondente versione E… | Media | Chiave TT 'ft.decided.note' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:38` | Chiavi i18n | Solo le righe a questo livello o migliore. | Rimozione dizionario IT; la chiave 'ft.chip.level' usa la corrispondente versione EN … | Media | Chiave TT 'ft.chip.level' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:39` | Chiavi i18n | Prove diagnostiche: sito determinato con margine, RT e cinetica coerenti. Solo il chi… | Rimozione dizionario IT; la chiave 'ft.chip.diagnostic' usa la corrispondente version… | Media | Chiave TT 'ft.chip.diagnostic' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:40` | Chiavi i18n | Ioni nuovi che non vengono da una regola di trasformazione. | Rimozione dizionario IT; la chiave 'ft.chip.unexpected' usa la corrispondente version… | Media | Chiave TT 'ft.chip.unexpected' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:41` | Chiavi i18n | Candidati con punteggio forte (75 o più): i criteri sono quasi tutti superati. Resta … | Rimozione dizionario IT; la chiave 'ft.chip.strong' usa la corrispondente versione EN… | Media | Chiave TT 'ft.chip.strong' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:42` | Chiavi i18n | Candidati con punteggio possibile (da 50 a 74): alcuni criteri non sono superati. | Rimozione dizionario IT; la chiave 'ft.chip.possible' usa la corrispondente versione … | Media | Chiave TT 'ft.chip.possible' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:43` | Chiavi i18n | Candidati con punteggio debole (sotto 50): di solito servono solo per controllo. | Rimozione dizionario IT; la chiave 'ft.chip.weak' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.chip.weak' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:44` | Chiavi i18n | Altre identità con la stessa massa; l'elenco è nella scheda Identity della riga. | Rimozione dizionario IT; la chiave 'ft.alt' usa la corrispondente versione EN di T.en | Media | Chiave TT 'ft.alt' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:45` | Chiavi i18n | Formula molecolare proposta per lo ione, se c'è. | Rimozione dizionario IT; la chiave 'ft.col.formula' usa la corrispondente versione EN… | Media | Chiave TT 'ft.col.formula' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:46` | Chiavi i18n | Numero di esperimenti MS2 abbinati a questa riga. L'elenco è nella sezione «MS2 match… | Rimozione dizionario IT; la chiave 'ft.col.ms2match' usa la corrispondente versione E… | Media | Chiave TT 'ft.col.ms2match' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:47` | Chiavi i18n | Apre sotto la tabella la sezione «MS2 matches», dove si possono correggere gli abbina… | Rimozione dizionario IT; la chiave 'ft.lr.ms2link' usa la corrispondente versione EN … | Media | Chiave TT 'ft.lr.ms2link' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:48` | Chiavi i18n | Solo le righe di cui la MS2 colloca la modifica in una parte della molecola. | Rimozione dizionario IT; la chiave 'ft.chip.region' usa la corrispondente versione EN… | Media | Chiave TT 'ft.chip.region' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:49` | Chiavi i18n | Si attiva quando il motore ha calcolato t0 e bianco per questa serie; senza, la colon… | Rimozione dizionario IT; la chiave 'ft.chip.wait' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.chip.wait' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:50` | Chiavi i18n | Esporta le righe e le colonne che vedi, nell'ordine in cui le vedi, con i filtri scri… | Rimozione dizionario IT; la chiave 'ft.export.view' usa la corrispondente versione EN… | Media | Chiave TT 'ft.export.view' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:51` | Chiavi i18n | Il progenitore resta in cima come riferimento: filtri e ordinamento non lo spostano. | Rimozione dizionario IT; la chiave 'ft.pin.parent' usa la corrispondente versione EN … | Media | Chiave TT 'ft.pin.parent' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:52` | Chiavi i18n | Solo le righe con un picco rilevato (non riempito) in almeno due file. | Rimozione dizionario IT; la chiave 'ft.chip.det2' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.chip.det2' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:53` | Chiavi i18n | Apre i filtri numerici: file, area, rapporto col bianco, m/z e RT. | Rimozione dizionario IT; la chiave 'ft.more.filters' usa la corrispondente versione E… | Media | Chiave TT 'ft.more.filters' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:54` | Chiavi i18n | Come si confrontano le aree fra i file: nessuna, somma o mediana delle aree. | Rimozione dizionario IT; la chiave 'ft.norm' usa la corrispondente versione EN di T.e… | Media | Chiave TT 'ft.norm' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:55` | Chiavi i18n | Numero della riga nella tabella del motore. | Rimozione dizionario IT; la chiave 'ft.col.id' usa la corrispondente versione EN di T… | Media | Chiave TT 'ft.col.id' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:56` | Chiavi i18n | Carica dello ione, se il motore l'ha determinata. | Rimozione dizionario IT; la chiave 'ft.col.charge' usa la corrispondente versione EN … | Media | Chiave TT 'ft.col.charge' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:57` | Chiavi i18n | In quanti file il picco è stato rilevato (non riempito né mirato). | Rimozione dizionario IT; la chiave 'ft.col.detn' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.col.detn' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:58` | Chiavi i18n | Aggiungi o togli colonne facoltative; le prime due restano sempre. | Rimozione dizionario IT; la chiave 'ft.cols' usa la corrispondente versione EN di T.e… | Media | Chiave TT 'ft.cols' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:59` | Chiavi i18n | Clic o Invio: la riga si espande qui sotto con tutto ciò che si sa. Esc la chiude. | Rimozione dizionario IT; la chiave 'ft.row.open' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.row.open' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:60` | Chiavi i18n | XIC di tutti i campioni con i bordi del picco, e lo spettro (MS1 con gli isotopi, MS2… | Rimozione dizionario IT; la chiave 'ft.tab.overview' usa la corrispondente versione E… | Media | Chiave TT 'ft.tab.overview' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:61` | Chiavi i18n | Per ogni campione: tipo e tempo, area, stato, RT dell'apice e bordi del picco. | Rimozione dizionario IT; la chiave 'ft.tab.samples' usa la corrispondente versione EN… | Media | Chiave TT 'ft.tab.samples' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:62` | Chiavi i18n | Massa neutra, addotto, candidati con le loro prove, livello proposto e livello deciso… | Rimozione dizionario IT; la chiave 'ft.tab.identity' usa la corrispondente versione E… | Media | Chiave TT 'ft.tab.identity' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:63` | Chiavi i18n | Forma del picco, segnale/rumore, larghezza, isotopo M+1, presenza nel bianco e nel t0… | Rimozione dizionario IT; la chiave 'ft.tab.quality' usa la corrispondente versione EN… | Media | Chiave TT 'ft.tab.quality' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:64` | Chiavi i18n | Inizio e fine del picco (mediana dei file che lo hanno rilevato). | Rimozione dizionario IT; la chiave 'ft.peak.borders' usa la corrispondente versione E… | Media | Chiave TT 'ft.peak.borders' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:65` | Chiavi i18n | Avvisi sul picco: forma irregolare, segnale/rumore basso, pochi punti. | Rimozione dizionario IT; la chiave 'ft.q.flags' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.q.flags' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:66` | Chiavi i18n | Ione che il motore ha scelto per questa riga ([M+H]+, [M+Na]+…) e massa neutra che ne… | Rimozione dizionario IT; la chiave 'ft.adduct' usa la corrispondente versione EN di T… | Media | Chiave TT 'ft.adduct' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:67` | Chiavi i18n | Apre il file nella vista Dati con l'XIC di questa m/z. | Rimozione dizionario IT; la chiave 'ft.showdata' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.showdata' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:68` | Chiavi i18n | Il livello che decidi tu. Resta nel browser per questa serie ed entra nelle esportazi… | Rimozione dizionario IT; la chiave 'ft.decided' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.decided' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:69` | Chiavi i18n | Altri risultati della serie, uno aperto alla volta. | Rimozione dizionario IT; la chiave 'ft.more' usa la corrispondente versione EN di T.e… | Media | Chiave TT 'ft.more' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:70` | Chiavi i18n | Nome del composto progenitore: scrivi 2 lettere per cercarlo nell'elenco, oppure lasc… | Rimozione dizionario IT; la chiave 'ft.f.name' usa la corrispondente versione EN di T… | Media | Chiave TT 'ft.f.name' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:71` | Chiavi i18n | Formula molecolare neutra del progenitore, per esempio C10H12N2O3S. Se manca, il moto… | Rimozione dizionario IT; la chiave 'ft.f.formula' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.f.formula' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:72` | Chiavi i18n | Struttura del progenitore (facoltativa se c'è la formula): serve ai frammenti e ai gr… | Rimozione dizionario IT; la chiave 'ft.f.smiles' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.f.smiles' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:73` | Chiavi i18n | Ione con cui il progenitore compare nello spettro. | Rimozione dizionario IT; la chiave 'ft.f.adduct' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.f.adduct' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:74` | Chiavi i18n | Tempo di ritenzione del progenitore, in minuti. Vuoto = automatico. | Rimozione dizionario IT; la chiave 'ft.f.prt' usa la corrispondente versione EN di T.… | Media | Chiave TT 'ft.f.prt' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:75` | Chiavi i18n | Finestra dell'XIC attorno a ogni m/z, in Da (o in ppm in alta risoluzione). | Rimozione dizionario IT; la chiave 'ft.f.tol' usa la corrispondente versione EN di T.… | Media | Chiave TT 'ft.f.tol' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:76` | Chiavi i18n | Quanto può spostarsi l'RT di un picco fra i file, in minuti. | Rimozione dizionario IT; la chiave 'ft.f.rtt' usa la corrispondente versione EN di T.… | Media | Chiave TT 'ft.f.rtt' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:77` | Chiavi i18n | Quante trasformazioni si possono combinare in un solo prodotto (1-3). | Rimozione dizionario IT; la chiave 'ft.f.steps' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.f.steps' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:78` | Chiavi i18n | Bassa risoluzione: il picco deve comparire in almeno questi campioni trattati, o in d… | Rimozione dizionario IT; la chiave 'ft.f.minf' usa la corrispondente versione EN di T… | Media | Chiave TT 'ft.f.minf' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:79` | Chiavi i18n | Ignora ciò che esce prima di questo RT (fronte non trattenuto), in minuti. | Rimozione dizionario IT; la chiave 'ft.f.rtmin' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.f.rtmin' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:80` | Chiavi i18n | Cerca anche ioni che crescono nel tempo senza venire da una regola di trasformazione.… | Rimozione dizionario IT; la chiave 'ft.f.disc' usa la corrispondente versione EN di T… | Media | Chiave TT 'ft.f.disc' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:81` | Chiavi i18n | Variazione di formula della regola, per esempio +O-H2. | Rimozione dizionario IT; la chiave 'ft.f.chg' usa la corrispondente versione EN di T.… | Media | Chiave TT 'ft.f.chg' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:82` | Chiavi i18n | Solo le righe a livello 5: solo massa esatta, nessuna MS2 o libreria che le spieghi. | Rimozione dizionario IT; la chiave 'ft.chip.l5' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.chip.l5' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:83` | Chiavi i18n | Solo le righe di questa polarità (POS = ioni positivi, NEG = negativi). | Rimozione dizionario IT; la chiave 'ft.chip.pol' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.chip.pol' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:84` | Chiavi i18n | Miglior candidato di libreria per la MS2 della feature: un'ipotesi con le sue prove, … | Rimozione dizionario IT; la chiave 'ft.col.cand' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.col.cand' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:85` | Chiavi i18n | Entropia spettrale: somiglianza della MS2 con il record di libreria, da 0 a 1. | Rimozione dizionario IT; la chiave 'ft.col.entropy' usa la corrispondente versione EN… | Media | Chiave TT 'ft.col.entropy' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:86` | Chiavi i18n | Nomi che la tabella d'origine (mzMine, GNPS2, MS-DIAL) dà alla feature: sono prove co… | Rimozione dizionario IT; la chiave 'ft.col.ext' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.col.ext' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:87` | Chiavi i18n | Gruppo di feature dello stesso composto (addotti, isotopi, frammenti in sorgente) e q… | Rimozione dizionario IT; la chiave 'ft.col.members' usa la corrispondente versione EN… | Media | Chiave TT 'ft.col.members' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:88` | Chiavi i18n | Massa neutra che spiega più feature del gruppo: è un'ipotesi, aprila per le prove di … | Rimozione dizionario IT; la chiave 'ft.col.nmass' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.col.nmass' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:89` | Chiavi i18n | Polarità della tabella: POS = ioni positivi, NEG = negativi. | Rimozione dizionario IT; la chiave 'ft.col.pol' usa la corrispondente versione EN di … | Media | Chiave TT 'ft.col.pol' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:90` | Chiavi i18n | Su che cosa poggia il livello: libreria spettrale, standard, struttura prevista, giud… | Rimozione dizionario IT; la chiave 'ft.col.basis' usa la corrispondente versione EN d… | Media | Chiave TT 'ft.col.basis' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:91` | Chiavi i18n | Perché il livello è questo: i controlli superati o falliti, in parole. | Rimozione dizionario IT; la chiave 'ft.col.reasons' usa la corrispondente versione EN… | Media | Chiave TT 'ft.col.reasons' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:92` | Chiavi i18n | Le feature del gruppo, con il ruolo di ciascuna nella massa neutra; un pulsante apre … | Rimozione dizionario IT; la chiave 'ft.tab.members' usa la corrispondente versione EN… | Media | Chiave TT 'ft.tab.members' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/40_tabui_testi.js:93` | Chiavi i18n | Apre il Banco 2b su questa feature, per rivederla e decidere il livello. | Rimozione dizionario IT; la chiave 'ft.go.banco' usa la corrispondente versione EN di… | Media | Chiave TT 'ft.go.banco' usata nei tooltip/popover di aiuto di Tabui. |
| `TP_Mine/js/44_tp_riga.js:57` | Chiavi i18n | Copia negli appunti i valori di questa riga. | Copies the values of this row to the clipboard. | Media | Tooltip help per l'azione di copia riga. Rimuovere blocco it e registrare solo en. |
| `mzlab/web/lang/it.js:3221` | Chiavi i18n | Tabella delle feature | Rimozione dizionario IT o allineamento EN; la chiave 'feat.title' esiste già in en.js | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3222` | Chiavi i18n | Il programma non cerca le feature: la tabella la porti tu. Importa l'esportazione CSV… | Rimozione dizionario IT o allineamento EN; la chiave 'feat.intro' esiste già in en.js | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3223` | Chiavi i18n | Importa tabella (CSV/TSV) | Rimozione dizionario IT o allineamento EN; la chiave 'feat.import' esiste già in en.j… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3224` | Chiavi i18n | Usa la tabella caricata in Tabui | Rimozione dizionario IT o allineamento EN; la chiave 'feat.mzf' esiste già in en.js | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3225` | Chiavi i18n | Svuota la tabella | Rimozione dizionario IT o allineamento EN; la chiave 'feat.clear' esiste già in en.js | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3226` | Chiavi i18n | Lista di ioni (m/z; RT in minuti) | Rimozione dizionario IT o allineamento EN; la chiave 'feat.list' esiste già in en.js | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3227` | Chiavi i18n | Misura nei file aperti | Rimozione dizionario IT o allineamento EN; la chiave 'feat.go' esiste già in en.js | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3228` | Chiavi i18n | {n} righe non valide (servono m/z e RT): {list}. | Rimozione dizionario IT o allineamento EN; la chiave 'feat.err.lines' esiste già in e… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3229` | Chiavi i18n | Nessuna feature nella tabella. | Rimozione dizionario IT o allineamento EN; la chiave 'feat.err.empty' esiste già in e… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3230` | Chiavi i18n | Troppi ioni: al massimo {max} per volta. | Rimozione dizionario IT o allineamento EN; la chiave 'feat.err.many' esiste già in en… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3231` | Chiavi i18n | Servono le colonne dell'm/z e dell'RT (in mzmine: «row m/z» e «row retention time»). | Rimozione dizionario IT o allineamento EN; la chiave 'feat.err.columns' esiste già in… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3232` | Chiavi i18n | Nessuna colonna della tabella corrisponde ai file aperti (il nome del file deve compa… | Rimozione dizionario IT o allineamento EN; la chiave 'feat.err.nofiles' esiste già in… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3233` | Chiavi i18n | In Tabui non c'è nessuna tabella caricata. | Rimozione dizionario IT o allineamento EN; la chiave 'feat.err.nomzf' esiste già in e… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3234` | Chiavi i18n | Nessuna tabella caricata. | Rimozione dizionario IT o allineamento EN; la chiave 'feat.none' esiste già in en.js | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3235` | Chiavi i18n | {n} feature × {m} file ({src}). | Rimozione dizionario IT o allineamento EN; la chiave 'feat.sum' esiste già in en.js | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3236` | Chiavi i18n | misurate dalla lista | Rimozione dizionario IT o allineamento EN; la chiave 'feat.src.list' esiste già in en… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3237` | Chiavi i18n | tabella importata | Rimozione dizionario IT o allineamento EN; la chiave 'feat.src.file' esiste già in en… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3238` | Chiavi i18n | tabella di Tabui | Rimozione dizionario IT o allineamento EN; la chiave 'feat.src.mzfinder' esiste già i… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3239` | Chiavi i18n | File senza colonna nella tabella (area 0): {list}. | Rimozione dizionario IT o allineamento EN; la chiave 'feat.unmatched' esiste già in e… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3240` | Chiavi i18n | Con area | Rimozione dizionario IT o allineamento EN; la chiave 'feat.col.det' esiste già in en.… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3241` | Chiavi i18n | …e altre {n} feature. | Rimozione dizionario IT o allineamento EN; la chiave 'feat.more' esiste già in en.js | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3242` | Chiavi i18n | Mostra il profilo nel tempo dello ione m/z {mz}, RT {rt} | Rimozione dizionario IT o allineamento EN; la chiave 'feat.row.aria' esiste già in en… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3243` | Chiavi i18n | Misuro lo ione {i} di {n}… | Rimozione dizionario IT o allineamento EN; la chiave 'feat.progress' esiste già in en… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3469` | Chiavi i18n | Un processo della pipeline si è fermato ed è stato riavviato: il passo riparte. | Rimozione dizionario IT o allineamento EN; la chiave 'tpmine.feat.restarted' esiste g… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3470` | Chiavi i18n | La pipeline è stata fermata. | Rimozione dizionario IT o allineamento EN; la chiave 'tpmine.feat.cancelled' esiste g… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3471` | Chiavi i18n | {name} era in profilo: Tabui lavora sui centroidi e ne ha usato una copia centroidata… | Rimozione dizionario IT o allineamento EN; la chiave 'tpmine.centroided' esiste già i… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `mzlab/web/lang/it.js:3472` | Chiavi i18n | Tabella senza prove: memoria insufficiente per le prove su {files} file (fase {phase}… | Rimozione dizionario IT o allineamento EN; la chiave 'tpmine.feat.degraded' esiste gi… | Media | Chiave di traduzione usata dall'integrazione di Tabui con mzlab/web. |
| `TP_Mine/js/50_attributio.js:67` | Tooltip/Messaggi | Banco 2b: | Bench 2b: | Media | Prefisso messaggio di errore in caso di fallimento caricamento banco. |
| `TP_Mine/js/34_pipeline_run.js:182` | UI/Tabelle | p.segno ('contro', 'pro') | Normalizzare p.sign ('against', 'supports') | Media | Segno delle prove diagnostiche della pipeline; gestito parzialmente con fallback esc(p.seg… |
| `TP_Mine/js/37_feature_table.js:34` | UI/Tabelle | valori select: 'nessuna', 'somma', 'mediana' | 'none', 'sum', 'median' | Media | Valori interni passati alla normalizzazione Rust; etichette UI già in inglese. |
| `TP_Mine/data/regole_tp_2.json:3` | Esportazioni/CLI | "predefinito": "fotocatalisi" | "default": "photocatalysis" | Bassa | Chiave profilo predefinito e nome identificativo del set di regole. |
| `TP_Mine/js/20_tpmine.js:792` | Esportazioni/CLI | Chiavi interne t.Q3_osservato, t.Q3_consigliato, t.RT_attesa_min, t.finestra_RT_min, … | t.Q3_observed, t.Q3_suggested, t.expected_RT_min, t.RT_window_min, t.rel_intensity_pc… | Bassa | Modello dati transizioni MRM (le intestazioni del foglio Excel sono già in inglese). |
| `crates/mzlab-feat/src/export.rs:186` | Esportazioni/CLI | {"hojas":[{"nombre":"Features","filas":[...]} | {"sheets":[{"name":"Features","rows":[...]} | Bassa | Chiavi JSON serializzate verso xlsx.js; non direttamente visibili se non ispezionando il p… |

## 4. Verifica Regola «Formula mai da sola»
La regola di progetto stabilisce che nei titoli, nelle intestazioni di tabella, nelle colonne esportate e nelle etichette UI, la parola «formula» non debba mai comparire da sola (nuda). Deve essere sempre esplicitamente qualificata per chiarire se si tratti della formula neutra della molecola (*Neutral formula*), della formula dello ione osservato (*Ion formula* o *[M+H]+ formula*), o di un frammento (*Fragment ion formula*).

### Censimento Sistematico delle Occorrenze di «Formula»

| File:Riga | Componente / Contesto | Testo rilevato | Stato conformità | Correzione consigliata |
|---|---|---|:---:|---|
| `TP_Mine/data/composti.csv:1` | Intestazione colonna CSV composti progenitori | `names,formula,mass,smiles,inchikey` | **Non conforme (Nuda)** | Rinominare colonna in `neutral_formula` o `formula_neutra` |
| `crates/mzlab-feat/src/attributio/review/feature.rs:103` | Etichetta candidato singolo senza struttura | `label: "formula"` | **Non conforme (Nuda)** | Sostituire con `label: "neutral_formula"` |
| `TP_Mine/js/20_tpmine.js:674` | Colonna facoltativa tabella candidati LR | `label: "Neutral formula", help: "ft.col.formula"` | Conforme (Qualificata) | Mantenere `Neutral formula` |
| `TP_Mine/js/20_tpmine.js:783` | Intestazione colonna export Ranking (CSV/Excel) | `"Neutral formula"` | Conforme (Qualificata) | Mantenere `Neutral formula` |
| `TP_Mine/js/20_tpmine.js:792` | Intestazione colonna export MRM (Excel) | `"Neutral formula"` | Conforme (Qualificata) | Mantenere `Neutral formula` |
| `TP_Mine/js/21_tpmine_hr.js:285` | Intestazione tabella HTML frammenti localizzazione | `<th>fragment ion formula</th>` | Conforme (Qualificata) | Mantenere `fragment ion formula` |
| `TP_Mine/js/21_tpmine_hr.js:297` | Intestazioni export HR Ranking (CSV/Excel) | `"Neutral formula", "Possible neutral formulas"` | Conforme (Qualificata) | Mantenere qualificazioni chiare |
| `TP_Mine/js/21_tpmine_hr.js:304` | Intestazione export HR Criteria (Excel) | `"Neutral formula"` | Conforme (Qualificata) | Mantenere `Neutral formula` |
| `TP_Mine/js/21_tpmine_hr.js:305` | Intestazioni export HR Localisation (Excel) | `"Neutral formula", "Fragment ion formula"` | Conforme (Qualificata) | Mantenere qualificazioni differenziate |
| `TP_Mine/js/21_tpmine_hr.js:307` | Intestazione export HR MSn tree (Excel) | `"Ion formula"` | Conforme (Qualificata) | Mantenere `Ion formula` |
| `TP_Mine/js/21_tpmine_hr.js:308` | Intestazione export HR Inclusion list (Excel) | `"Neutral formula"` | Conforme (Qualificata) | Mantenere `Neutral formula` |
| `crates/mzlab-feat/src/attributio/export/schymanski_rows.rs:202` | Colonna esportazione Schymanski | `kv("neutral_formula", ...)` | Conforme (Qualificata) | Mantenere `neutral_formula` |
| `crates/mzlab-feat/src/attributio/export/publication_text.rs:18` | Dizionario colonne pubblicazione | `("Neutral molecular formula", ...)` | Conforme (Qualificata) | Mantenere `Neutral molecular formula` |
| `TP_Mine/js/40_tabui_testi.js:45 / 131` | Aiuto colonna formula (`ft.col.formula`) | `"Molecular formula proposed for the ion..."` | Da perfezionare | Aggiornare in "Neutral molecular formula proposed for the candidate ion..." |
| `TP_Mine/js/40_tabui_testi.js:71 / 157` | Aiuto campo input formula progenitore (`ft.f.formula`) | `"Neutral molecular formula of the parent..."` | Conforme (Qualificata) | Mantenere qualifica neutra esplicita |
| `TP_Mine/js/40_tabui_testi.js:81 / 167` | Aiuto variazione formula regola (`ft.f.chg`) | `"Formula change of the rule..."` | Conforme (Qualificata) | Mantenere `Formula change` |
| `crates/mzlab-cli/src/tests.rs:142` | Test di non regressione CLI feature table | `!lines[0].to_lowercase().split(",").any(|c| c == "formula")` | Conforme (Test guardiano) | Verifica attiva che previene colonne 'formula' nude nell'export CLI |
| `crates/mzlab-feat/tests/attributio_export.rs:460` | Test di non regressione Attributio export | `assert_ne!(h, "formula", "{kind}: a column called just «formula»")` | Conforme (Test guardiano) | Verifica attiva che impedisce intestazioni 'formula' nude negli export |

## 5. Mappatura Chiavi di Lingua (`lang/` e `40_tabui_testi.js`)
Tabui adotta un'architettura di localizzazione a due livelli:
1. **Catalogo dedicato `TABUI_T` (`TP_Mine/js/40_tabui_testi.js`)**: contiene i testi esplicativi e di aiuto richiamati dal componente popover `TPHELP` (attributi `data-tt`). Tutte le 84 chiavi esistono già sia in `it` che in `en`.
2. **Catalogo globale `mzlab/web/lang/`**: contiene le chiavi usate dall'integrazione di Tabui con la finestra principale e con la tabella feature (`feat.*` e `tpmine.*`). Tutte le chiavi censite in `it.js` dispongono già della corrispondente traduzione in `en.js`.

### Mappatura delle Chiavi Globali (`feat.*` e `tpmine.*`)

| Chiave i18n | Testo Italiano (`lang/it.js`) | Testo Inglese (`lang/en.js`) | Stato & Raccomandazione |
|---|---|---|---|
| `feat.title` | Tabella delle feature | Feature table | Allineato. Nessuna discrepanza. |
| `feat.intro` | Il programma non cerca le feature: la tabella la porti tu... | The program does not look for features: you bring the table... | Allineato. |
| `feat.import` | Importa tabella (CSV/TSV) | Import table (CSV/TSV) | Allineato. |
| `feat.mzf` | Usa la tabella caricata in Tabui | Use the table loaded in Tabui | Allineato. Riferimento corretto a Tabui. |
| `feat.clear` | Svuota la tabella | Clear the table | Allineato. |
| `feat.list` | Lista di ioni (m/z; RT in minuti) | List of ions (m/z; RT in minutes) | Allineato. |
| `feat.go` | Misura nei file aperti | Measure in the open files | Allineato. |
| `feat.err.lines` | {n} righe non valide (servono m/z e RT): {list}. | {n} invalid lines (m/z and RT are needed): {list}. | Allineato. |
| `feat.err.empty` | Nessuna feature nella tabella. | No feature in the table. | Allineato. |
| `feat.err.many` | Troppi ioni: al massimo {max} per volta. | Too many ions: at most {max} at a time. | Allineato. |
| `feat.err.columns` | Servono le colonne dell'm/z e dell'RT... | The m/z and RT columns are needed... | Allineato. |
| `feat.err.nofiles` | Nessuna colonna della tabella corrisponde ai file aperti... | No column of the table matches an open file... | Allineato. |
| `feat.err.nomzf` | In Tabui non c'è nessuna tabella caricata. | There is no table loaded in Tabui. | Allineato. |
| `feat.none` | Nessuna tabella caricata. | No table loaded. | Allineato. |
| `feat.sum` | {n} feature × {m} file ({src}). | {n} features × {m} files ({src}). | Allineato. |
| `feat.src.list` | misurate dalla lista | measured from the list | Allineato. |
| `feat.src.file` | tabella importata | imported table | Allineato. |
| `feat.src.mzfinder` | tabella di Tabui | Tabui table | Allineato. |
| `feat.unmatched` | File senza colonna nella tabella (area 0): {list}. | Files without a column in the table (area 0): {list}. | Allineato. |
| `feat.col.det` | Con area | With area | Allineato. |
| `feat.more` | …e altre {n} feature. | …and {n} more features. | Allineato. |
| `feat.row.aria` | Mostra il profilo nel tempo dello ione m/z {mz}, RT {rt} | Show the time profile of the ion m/z {mz}, RT {rt} | Allineato. |
| `feat.progress` | Misuro lo ione {i} di {n}… | Measuring ion {i} of {n}… | Allineato. |
| `tpmine.feat.restarted` | Un processo della pipeline si è fermato ed è stato riavviato... | A worker of the pipeline stopped and was restarted... | Allineato. |
| `tpmine.feat.cancelled` | La pipeline è stata fermata. | The pipeline was stopped. | Allineato. |
| `tpmine.centroided` | {name} era in profilo: Tabui lavora sui centroidi... | {name} was in profile: Tabui works on centroids... | Allineato. |
| `tpmine.feat.degraded` | Tabella senza prove: memoria insufficiente... | Table without proofs: not enough memory... | Allineato. |

### Strategia per `40_tabui_testi.js`
Il file `TP_Mine/js/40_tabui_testi.js` definisce la funzione `TT(key, params)`:
```javascript
const TT = (key, params) => {
  let lang = 'en';
  // fallback e lookup su T[lang]
  let s = (T[lang] && T[lang][key]) || T.en[key] || '';
  ...
};
```
**Raccomandazione tecnica:**
1. Rimuovere interamente l'oggetto `T.it` (84 chiavi) da `40_tabui_testi.js`.
2. Semplificare `TT` affinché utilizzi un unico dizionario `T` contenente direttamente le stringhe inglesi, eliminando qualsiasi logica di commutazione basata su `I18N.lang` o `?lang=it`.
3. Rimuovere in `44_tp_riga.js` il ramo `it` di `TXT` mantenendo unicamente `en`.
4. Correzione della chiave `ft.go.banco`: sostituire `Banco 2b` con `Bench 2b` nel testo inglese.
## 6. Piano di Azione e Conclusioni
Per portare a compimento la migrazione «Tabui solo inglese», si consiglia al team frontend di procedere secondo le seguenti fasi ordinate e a basso rischio:

### Fase 1: Correzioni ad Alta Priorità (UI ed Esportazioni)
1. **Attributio Bench 2b (`52_attributio_banco.js` e `50_attributio.js`)**:
   - Sostituire le occorrenze di `Banco 2b` con `Bench 2b` nel titolo della modale (`<h3>`), nei messaggi di errore e negli attributi `aria-label` (`Close Bench 2b`).
2. **Verdetto esportazioni (`20_tpmine.js:784`)**:
   - Nel metodo `exportAll`, mappare `r.label` con la tabella inglese `LAB[r.label]` per garantire che nei file CSV ed Excel la colonna `Verdict` contenga `strong`, `possible`, `weak`, `parent` invece di `forte`, `possibile`, `debole`, `progenitore`.
3. **Regola «Formula mai da sola» in `composti.csv`**:
   - Rinominare l'intestazione di colonna `formula` in `neutral_formula` nel dataset curato dei composti progenitori.
4. **Profilo reazioni (`regole_tp_2.json`)**:
   - Aggiornare la descrizione del profilo fotocatalisi: da `TiO2/UV: radicali ·OH, lacune h+, O2·−` a `TiO2/UV: ·OH radicals, h+ holes, O2·−`.

### Fase 2: Consolidamento e Semplificazione i18n
1. **Disattivazione catalogo IT in Tabui**:
   - Rimuovere la sezione `T.it` da `40_tabui_testi.js` e impostare la risoluzione diretta su stringhe inglesi.
   - Rimuovere il ramo `it` da `44_tp_riga.js`.
2. **Allineamento chiavi `feat.*` in `mzlab/web/lang/`**:
   - Mantenere le definizioni inglesi in `en.js` come riferimento primario.
   - Verificare che le etichette delle opzioni di normalizzazione in `37_feature_table.js` non dipendano da stringhe localizzate.

### Fase 3: Verifica Automatica e Guardiani
1. **Esecuzione test e2e dedicati a Tabui**:
   `python3 tests_e2e/e2e_tpmine1.py`
   `python3 tests_e2e/e2e_tpmine2.py`
   `python3 tests_e2e/e2e_tpmine_lr_tabella.py`
   `python3 tests_e2e/e2e_untarget.py`
2. **Verifica non-regressione esportazioni**:
   - Eseguire i test in `crates/mzlab-cli/src/tests.rs` e `crates/mzlab-feat/tests/attributio_export.rs` per confermare che nessuna colonna esportata presenti la dicitura nuda `formula`.

### Conclusioni
L'architettura di Tabui è già ampiamente predisposta per la modalità «solo inglese»: le tabelle principali e i flussi di calcolo operano già con testi in lingua inglese. L'intervento di completamento richiede la correzione mirata di poche sacche residue ad alta priorità (le stringhe esportate nei verdetti di `20_tpmine.js`, la denominazione di Bench 2b e l'intestazione di `composti.csv`), seguita dalla pulizia del dizionario `T.it` non più necessario.
