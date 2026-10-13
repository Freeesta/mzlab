# mzLab

Programma didattico per esplorare dati LC-MS e cercare i **prodotti di trasformazione (TP)** di un inquinante degradato nel tempo (esperienza al triplo quadrupolo del laboratorio di analisi degli inquinanti, UniTO). Lo studente sceglie cosa estrarre, integra, annota e disegna: il programma mostra i dati, **non dà le risposte**. Comprende la Teoria (spettrometria di massa a bassa e alta risoluzione, GC, LC) e la Pratica per preparare l'orale.

*(English: a light teaching tool that runs in the browser to explore LC-MS data and look for transformation products, with theory chapters and practice games. It shows the data; the student does the reasoning.)*

## Uso

Apri **https://freeesta.github.io/mzlab/** in un browser recente (Chrome, Edge, Firefox, Safari) su computer o tablet. Non si installa nulla: il calcolo gira **dentro il browser**. La prima volta si scaricano circa 10-15 MB; dopo funziona anche senza internet e si può installare come app (Chrome/Edge: «Installa» nella barra degli indirizzi; Safari: Aggiungi al Dock o alla schermata Home).

- **I tuoi file non lasciano il computer**: la pagina non li invia a nessun server e non raccoglie dati sui visitatori. Sessione, disegni e progressi restano nello spazio del browser e si ripristinano alla riapertura (stesso computer, stesso browser).
- Trascina i file `.mzML` (e, se vuoi, il `.dam` del metodo), controlla tempi e tipi, premi **Carica dati**. Senza file: **«Prova con i file di esempio»**.
- Si aprono direttamente anche i `.raw` Thermo, i `.wiff` SCIEX (con il loro `.wiff.scan`) e le cartelle `.D` Agilent. Per gli altri formati si converte in mzML (guida nella schermata di caricamento).
- **Da smartphone** il sito mostra solo la Teoria; Dati, Disegno e giochi vanno usati da computer o tablet.

## Le schede

- **Dati**: cromatogrammi (TIC, BPC, XIC, MRM), spettri, mappa RT×m/z, schede separate Full Scan / MS2 / MRM, file ad alta risoluzione (Orbitrap, Q-TOF) e DDA; integrazione dei picchi con tabella esportabile in Excel (.xlsx), correzione del fondo, calcolatrice m/z, tavola periodica, addotti, perdite neutre, confronto delle MS2 con librerie di spettri. Le tabelle e i grafici della relazione si costruiscono in Excel.
- **Disegno**: editor chimico Ketcher per molecole, frammenti e vie di trasformazione, con formula e massa sotto ogni struttura e Δm sopra ogni freccia; gli strumenti visibili si scelgono in «Strumenti dell'editor»; esportazione PNG, JPEG, SVG e .ket.
- **Teoria**: 22 capitoli in sei parti (dal problema dei TP alla cromatografia, alla ionizzazione, agli analizzatori, alla lettura degli spettri EI e MS2), glossario, bibliografia e formulario per l'orale, con figure interattive. Si apre anche da sola, con doppio clic su `mzlab/web/teoria/index.html`.
- **Pratica**: dallo spettro EI alla struttura (con Modalità orale), domande dell'orale con simulazione d'esame, perdite neutre, isotopi, formula esatta, scelta dello strumento, quadrupolo. Correzione immediata e ripasso distanziato.
- La **«i»** in alto apre le informazioni; la guida completa è il capitolo A della Teoria («Come si usa mzLab»).

Con risoluzione unitaria **un m/z è un candidato, non un'identificazione**.

## Sviluppo

Tutto ciò che serve a chi modifica il programma (regole, architettura, test, decisioni prese) è in [`AGENTS.md`](AGENTS.md) (nucleo) e in [`docs/agenti/`](docs/agenti/) (un file per area). In breve: nessuna compilazione; `python3 tools/verifica.py` esegue tutti i controlli (sintassi JS, pytest, prove nel browser con Playwright); il sito si costruisce con `python3 tools/build_site.py` e si pubblica da solo a ogni push su `main`. Per modificare i testi della Teoria: [`mzlab/web/teoria/LEGGIMI.md`](mzlab/web/teoria/LEGGIMI.md).

## Uso dell'intelligenza artificiale

Il codice, i test e la documentazione tecnica sono sviluppati con l'assistenza di strumenti di IA generativa (Claude Code, Anthropic), diretti dall'autore; i commit scritti con quell'assistenza lo riportano come coautore. Cosa entra in `main` lo decide il manutentore, e solo dopo i test automatici obbligatori. Il programma, quando lo si usa, **non contiene e non chiama modelli di IA**: tutto il calcolo avviene nel browser (o nell'app desktop) con le regole scritte nel codice.

*English: the code, tests and technical documentation are developed with the assistance of generative AI tools (Claude Code, by Anthropic) directed by the author; such commits list it as co-author. What enters `main` is decided by the maintainer after the required automated tests. The program itself contains and calls no AI model.*

## Licenze e crediti

Il codice è sotto licenza **MIT** (`LICENSE`, Federico Cristaudo). Materiale di terzi, ciascuno con la sua licenza (testi in [`LICENZE-TERZI.md`](LICENZE-TERZI.md), copiato anche nel sito):

- **Ketcher** 3.18.0 (EPAM Systems), Apache-2.0: `mzlab/web/vendor/ketcher/`, https://github.com/epam/ketcher
- **OpenChemLib JS** 9.25.1 (Zakodium / Actelion), BSD-3-Clause: `mzlab/web/vendor/openchemlib.js`, https://github.com/cheminfo/openchemlib-js
- **Pyodide** 314.0.7, MPL-2.0 (Python: licenza PSF), usato solo da mzFinder (nascosto), scaricato dalla build del sito: https://github.com/pyodide/pyodide
- **NumPy** (incluso in Pyodide, solo mzFinder), BSD-3-Clause: https://numpy.org
- **Spettri EI** della Teoria e della Pratica: MassBank Europe, CC BY-NC-SA (fonte sotto ogni spettro).
- **Masse e abbondanze degli elementi** (`elements.js`): masse da OpenChemLib, pesi atomici da Ketcher, abbondanze IUPAC.

## Come citare

Se usi mzLab nel corso o in una pubblicazione, cita il software con i dati di [`CITATION.cff`](CITATION.cff) (il pulsante «Cite this repository» di GitHub li formatta):

> Cristaudo, F. *mzLab: a browser-based teaching tool for LC-MS data analysis and the search for transformation products*. Università degli Studi di Torino. https://freeesta.github.io/mzlab/ (codice: https://github.com/Freeesta/mzlab, licenza MIT). DOI Zenodo: da assegnare alla prima release.

*How to cite (English): cite the software with the data of [`CITATION.cff`](CITATION.cff) (GitHub's “Cite this repository” button formats it). Cristaudo, F. mzLab: a browser-based teaching tool for LC-MS data analysis and the search for transformation products. University of Turin. https://freeesta.github.io/mzlab/ (code: https://github.com/Freeesta/mzlab, MIT licence). Zenodo DOI: to be assigned at the first release; cite the concept DOI for the software in general and the version DOI for a given version.*

## Limiti noti

- Non ancora validato contro il software dello strumento su più composti.
- Numpress non supportato (convertire con zlib).
- Provato a fondo con Chromium; Firefox e Safari solo con le prove automatiche essenziali.
