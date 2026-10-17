# Modulo 1 — Navigazione e Funzionalità della Vista Dati

Questo modulo illustra in dettaglio come gestire, esplorare e analizzare
i tracciati cromatografici e spettrometrici all'interno della **Vista Dati**
di mzLab.

---

## 1. Caricamento dei File e Organizzazione dei Dati

Al primo accesso all'applicazione su <https://freeesta.github.io/mzlab/>,
viene presentata la schermata iniziale di caricamento.

[Screenshot: Schermata iniziale di caricamento di mzLab. Al centro campeggia
un'ampia zona di rilascio circondata da un bordo tratteggiato con la scritta
«Trascina qui i file oppure clicca per sceglierli dal computer» con l'elenco dei
formati supportati (.mzML · .raw · .wiff · .D). Accanto all'intestazione
«1. File dei dati» è presente il pulsante cartella per aprire un'intera directory.
Sotto il riquadro principale è presente il comando «Prova con i file di esempio ▾»
(Full Scan, MS2, MRM, HRMS), che permette di caricare set didattici preimpostati
senza scaricare file esterni.]

### 1.1 Formati Supportati e Modalità di Rilascio
mzLab include lettori nativi scritti in Rust e compilati in WebAssembly, che
permettono di leggere direttamente nel browser sia formati aperti che file
proprietari dei principali costruttori di spettrometri di massa:

- **Formati aperti standard (.mzML, .mzXML, NetCDF .cdf/.nc):** supportati sia
  in profilo continuo (*profile*) che già centroidati (*centroided*), sia per
  spettrometria a bassa risoluzione (LRMS, quadrupoli) che ad alta risoluzione
  (HRMS, Q-TOF, Orbitrap).
- **File Thermo Fisher `.raw`:** letti nativamente dal lettore Rust/WASM di mzLab
  (acquisizioni Orbitrap, Q-Exactive, TSQ, ecc.), senza necessità di Windows né
  di DLL proprietarie.
- **File SCIEX `.wiff`:** letti nativamente (3200 QTRAP, serie QTRAP, TripleTOF)
  trascinando contemporaneamente il file `.wiff` e il relativo file `.wiff.scan`.
- **Cartelle Agilent `.D`:** lette nativamente selezionando o trascinando la cartella
  della corsa analitica (tramite il pulsante «Apri cartella» o il drag-and-drop).
- **Cartelle Waters MassLynx `.raw`:** lette nativamente tramite selezione o rilascio
  della cartella di corsa Waters.
- **Bruker e Shimadzu:** cartelle Bruker `.d` (TDF, TSF, BAF) e file Shimadzu
  (`.qgd`, `.lcd`) gestiti dai rispettivi moduli WASM dedicati.
- **Cartelle e caricamento multiplo:** è possibile selezionare intere cartelle o
  trascinare contemporaneamente più corse (ad esempio serie cinetiche
  $t_0, t_5, \dots, t_{60}$ o standard MRM).
- **Apertura progressiva e indicizzazione locale:** mzLab non blocca l'interfaccia
  durante il caricamento. Il motore in WebAssembly analizza i file in background
  tramite Web Workers, memorizza i binari nel file system virtuale locale (OPFS)
  e genera un file indice leggero (`.mzidx`). Questo permette l'apertura
  immediata dei cromatogrammi anche per file da centinaia di megabyte.

---

## 2. Area di Lavoro: Il Blocco Cromatografico

Una volta completato il caricamento, la schermata si struttura in due grandi
aree orizzontali: il **cromatogramma** nella metà superiore e lo
**spettro di massa** nella metà inferiore.

[Screenshot: Vista Dati completa suddivisa orizzontalmente. Nella metà
superiore è visualizzato il tracciato del cromatogramma con il tempo di
ritenzione in minuti sull'asse orizzontale e l'intensità in conteggi al secondo
(cps) sull'asse verticale. Una linea verticale blu indica il cursore temporale
corrente. Nella metà inferiore è visibile lo spettro di massa associato a
quell'istante temporale, con il rapporto m/z sulle ascisse e l'abbondanza
relativa o assoluta sulle ordinate. In alto a destra compaiono le schede dei
file caricati.]

### 2.1 Tipologie di Tracciati Cromatografici
Nella barra degli strumenti del cromatogramma è possibile selezionare il
tipo di traccia da visualizzare:
1. **TIC (*Total Ion Chromatogram*):** somma l'intensità di tutti gli ioni
   registrati in ciascuna scansione in funzione del tempo. Mostra il profilo
   complessivo della corsa analitica, compresi fondo ed eluenti.
2. **BPC (*Base Peak Chromatogram*):** riporta in ogni istante l'intensità
   dello ione più abbondante della scansione. Risulta molto più pulito del
   TIC per individuare analiti in matrici complesse.
3. **EIC / XIC (*Extracted Ion Chromatogram*):** visualizza l'andamento
   temporale dell'intensità di una specifica finestra di $m/z$ (es. un addotto
   $[M+H]^+$ o un frammento diagnostico).

### 2.2 Selezione Temporale: Scansione Singola vs Spettro Medio
- **Clic singolo sul cromatogramma:** posiziona il cursore verticale (linea blu).
  Il pannello inferiore mostra istantaneamente lo spettro della *singola
  scansione* acquisita a quel tempo di ritenzione.
- **Trascina orizzontale con il mouse (intervallo temporale):** evidenzia
  un intervallo colorato (ad esempio attraverso la larghezza di un picco
  cromatografico da 4.10 a 4.25 min). Lo spettro inferiore calcola
  automaticamente lo **spettro medio** di tutte le scansioni comprese
  nell'intervallo, migliorando significativamente il rapporto segnale/rumore
  ($S/N$) e riducendo le fluttuazioni istantanee.

[Screenshot: Cromatogramma con un intervallo temporale selezionato tramite
trascinamento del cursore del mouse attorno a un picco principale a 5.32 min.
L'area selezionata appare evidenziata in azzurro trasparente con due delimitatori
verticali. L'intestazione del pannello inferiore riporta l'indicazione:
«Spettro medio: 5.28 - 5.36 min (12 scansioni mediate)».]

### 2.3 Navigazione da Tastiera e Scorrimento Scansioni
La navigazione dinamica lungo il picco cromatografico consente di osservare
l'evoluzione dei singoli ioni (fondamentale per distinguere ioni coeluenti o
frammentazioni avvenute in sorgente):
- **Tasti ← e →:** avanzamento o arretramento di **una singola scansione**.
  Tenendo premuta la freccia, lo scorrimento avviene a circa 15 scansioni/s.
- **Maiusc + ← / →:** salti rapidi di **5 scansioni**.
- **Barra Spaziatrice (Spazio):** avvia o mette in pausa la riproduzione
  automatica continua lungo il cromatogramma. Se è attivo un intervallo
  selezionato, la riproduzione cicla avanti e indietro all'interno dell'intervallo.
- **Tasto Esc (oppure clic):** interrompe immediatamente lo scorrimento continuo.
- **Icona Lucchetto (blocco asse intensità):** posizionata in alto a sinistra
  nel pannello dello spettro, blocca l'altezza massima dell'asse delle intensità.
  In questo modo, scorrendo lungo un picco cromatografico, l'altezza dei picchi
  spettrometrici cresce e decresce proporzionalmente alla loro reale intensità,
  senza che la scala si riscali continuamente al 100%.

---

## 3. Il Pannello Spettro di Massa: Esplorazione e Zoom

Il pannello dello spettro visualizza la distribuzione dell'abbondanza degli
ioni in funzione del rapporto $m/z$.

[Screenshot: Pannello dello spettro di massa dettagliato. Sulla parte superiore
del grafico sono visibili le etichette numeriche con i valori esatti di m/z
sopra ogni picco significativo (ad esempio 152.0706 e 174.0526). Nella barra
degli strumenti del pannello si trovano le icone per il righello, la tabella
dei picchi, l'ingranaggio dei parametri di visualizzazione e il lucchetto di
blocco scala.]

### 3.1 Controlli di Zoom e Scala
- **Zoom rettangolare:** cliccare e trascinare orizzontalmente sull'area del
  grafico per ingrandire una specifica finestra di $m/z$.
- **Zoom asse Y (intensità):** cliccare e trascinare verticalmente sui numeri
  dell'asse verticale delle ordinate.
- **Zoom asse X (m/z):** cliccare e trascinare orizzontalmente sui numeri
  dell'asse delle ascisse.
- **Doppio clic sul grafico (oppure tasto Backspace):** ripristina la visuale
  completa dello spettro intero (*Whole View*).
- **Annullamento zoom (Cmd + Z su Mac, Ctrl + Z su Windows):** torna
  all'inquadratura precedente (mzLab memorizza gli ultimi 15 livelli di zoom).

---

## 4. Il Righello Spettrometrico ($\Delta m/z$) e Riconoscimento Addotti

Il righello è uno degli strumenti analitici più potenti di mzLab. Permette di
misurare con precisione la distanza di massa tra due picchi e interrogare
immediatamente le tabelle chimiche.

[Screenshot: Spettro di massa con il righello attivo. Sopra due picchi a m/z
152.0706 e m/z 174.0526 compare un segmento orizzontale a linea continua
terminante con due frecce verticali sui rispettivi apici. A metà del segmento
è visibile un'etichetta colorata con la scritta: «Na⁺/H⁺ · Δm/z 21.9820 (21.98
Da)». Cliccando con il tasto destro sull'etichetta si apre il menu contestuale
con le opzioni di ricerca tra le perdite neutre e gli scambi di addotto.]

### 4.1 Utilizzo del Righello Passo-Passo
1. Fare clic sull'icona del **Righello** nella barra superiore dello spettro
   (oppure fare clic destro sul picco di partenza e scegliere
   **«Misura da questo picco»**).
2. Il primo picco selezionato viene marcato come riferimento con una linea
   verticale tratteggiata e l'etichetta «rif.».
3. Fare clic su un secondo picco dello spettro: compare istantaneamente una
   barra orizzontale con la differenza esatta $\Delta m/z$.
4. **Riconoscimento automatico degli scambi di addotto:**
   Se la differenza riscontrata corrisponde a uno scambio ionico noto entro
   la tolleranza dello strumento, mzLab etichetta direttamente la transizione:
   - In modalità positiva:
     * $Na^+ / H^+$ ($\Delta m/z \approx 21.9819$)
     * $NH_4^+ / H^+$ ($\Delta m/z \approx 17.0265$)
     * $K^+ / H^+$ ($\Delta m/z \approx 37.9559$)
   - In modalità negativa:
     * $[M+Cl]^- / [M-H]^-$ ($\Delta m/z \approx 35.9767$)
     * $[M+HCOO]^- / [M-H]^-$ ($\Delta m/z \approx 45.9929$)
5. **Ricerca nelle perdite neutre:**
   Facendo clic destro sulla misura del righello e selezionando
   **«Cerca nelle perdite neutre»**, si apre direttamente la scheda laterale
   delle perdite con il valore preimpostato (es. perdita di $CO$ $-28\text{ Da}$,
   $H_2O$ $-18\text{ Da}$, $CO_2$ $-44\text{ Da}$).
6. Per rimuovere le misure attive, premere il tasto **Esc** oppure selezionare
   «Togli le misure» dal menu contestuale.

---

## 5. Simulazione del Pattern Isotopico Teorico da Formula

Per verificare se un picco osservato corrisponde effettivamente a una specifica
formula chimica, mzLab include un generatore di pattern isotopici integrato.

[Screenshot: Spettro ad alta risoluzione con sovrapposto il pattern isotopico
teorico. Sui picchi sperimentali a m/z 152.0706 (M), 153.0739 (M+1) e 154.0755
(M+2) sono sovrapposti dei cerchietti rossi che indicano l'intensità e la
posizione isotopica teorica calcolata per la formula C8H10NO2+. In alto a destra
nello spettro compare la dicitura: «C8H9NO2 [M+H]+ (R=auto)», confermando la
perfetta coincidenza tra profilo teorico e sperimentale.]

### 5.1 Procedura di Confronto Isotopico
1. Posizionare il puntatore sul picco principale (monoisotopico) da analizzare.
2. Fare **clic con il tasto destro del mouse** sul picco.
3. Selezionare la voce di menu **«Profilo isotopico di una formula…»**.
4. Nella finestra di dialogo che compare, digitare la formula neutra proposta
   con l'addotto (es. `C8H9NO2 [M+H]+`), oppure la formula già ionizzata
   (es. `C8H10NO2+`).
5. Premere Invio: mzLab calcola la distribuzione probabilistica di tutte le
   combinazioni isotopiche ($^{13}C, ^{15}N, ^{18}O, ^{34}S, ^{37}Cl, \dots$),
   sovrappone i cerchietti rossi teorici sui centroidi sperimentali e adatta
   automaticamente lo zoom alla finestra del cluster isotopico ($M, M+1, M+2$).
6. Per chiudere la simulazione, fare clic destro e scegliere
   **«Togli il profilo isotopico»** (oppure fare doppio clic per ripristinare
   la vista intera dello spettro).

---

## 6. Estrazione XIC e Integrazione dei Picchi

L'estrazione del cromatogramma a ione estratto (XIC) consente di isolare
il segnale di un analita dal fondo chimico.

[Screenshot: Finestra con due tracciati cromatografici impilati. Il tracciato
superiore mostra il TIC generale grezzo, ricco di picchi e fondo; il tracciato
inferiore mostra l'XIC estratto a m/z 152.0706 con finestra di tolleranza di 5 ppm.
Sul picco dell'XIC a tempo di ritenzione 4.18 min è visibile l'area integrata
colorata in verde chiaro, delimitata da due marcatori di inizio e fine picco
con l'indicazione del valore dell'area: «Area: 1.48e6, S/N: 142».]

### 6.1 Finestre di Estrazione: Alta Risoluzione (HR) vs Bassa Risoluzione (LR)
- **Alta Risoluzione (HRMS):** l'estrazione deve essere effettuata impostando
  una tolleranza in **parti per milione (ppm)**.
  Il valore predefinito è $\pm 5\text{ ppm}$ (ad esempio per $m/z\ 152.0706$,
  una finestra di $5\text{ ppm}$ corrisponde a un intervallo strettissimo di
  soli $\pm 0.00076\text{ Da}$). Questo elimina ogni interferenza isobara.
- **Bassa Risoluzione (LRMS, Quadrupolo singolo o triplo):** la tolleranza
  va impostata in unità di massa atomica (Da nominali), tipicamente
  $\pm 0.5\text{ Da}$ o $\pm 0.7\text{ Da}$ attorno al valore nominale
  (es. $m/z\ 152.0 \pm 0.5$).

### 6.2 Integrazione Manuale e Automatica dei Picchi
1. **Integrazione Manuale e Regolazione Bordi:**
   - Fare clic destro sul picco cromatografico nell'XIC e selezionare
     **«Integra»** (oppure posizionare il cursore sul picco e premere il tasto **I**).
   - È possibile correggere i bordi dell'integrazione trascinando direttamente
     le barre verticali dei delimitatori nel pannello.
   - mzLab calcola l'area sottesa con la regola dei trapezi e baseline lineare,
     l'altezza del picco, la larghezza a metà altezza (FWHM) e la stima del
     rumore di fondo locale ($S/N$).
2. **Integrazione Automatica:**
   - Nella barra degli strumenti del cromatogramma/XIC, premere l'icona
     dell'**integrazione automatica** (o clic destro $\rightarrow$ **«Integra»**).
   - Per visualizzare le etichette degli apici con il rispettivo tempo di
     ritenzione, attivare l'opzione **«picchi»** nell'intestazione del pannello.
   - mzLab propone automaticamente i bordi del picco calcolando la linea di
     base (con algoritmo SNIP per la sottrazione del fondo).
3. **Tabella ed Esportazione:**
   - I picchi integrati vengono inseriti nella tabella integrata della sessione.
   - Possono essere esportati direttamente in formato compatibile con fogli di
     calcolo (TSV, HTML o file `.xlsx`).
