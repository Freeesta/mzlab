# Modulo 3 — Criticità e Punti di Attenzione dell'Interfaccia

Questo modulo raccoglie e chiarisce gli aspetti dell'interfaccia di mzLab
che possono generare dubbi negli studenti o che si discostano dai tradizionali
software spettrometrici desktop.

---

## 1. Gestione dei Formati Proprietari (.raw, .wiff, .D) e Lettori Nativi Rust

Uno degli aspetti più avanzati di mzLab è la capacità di elaborare direttamente
all'interno del browser i formati di dati prodotti dai principali strumenti di
laboratorio, senza richiedere l'installazione di software commerciale né la
preventiva conversione manuale su computer Windows.

### 1.1 Come mzLab Legge i Formati Proprietari nel Browser
I formati generati dai costruttori (ad es. i file `.raw` di Thermo Fisher, `.wiff`
di SCIEX, le cartelle `.D` di Agilent o `.raw` di Waters MassLynx) sono tradizionalmente
vincolati a librerie dinamiche Windows proprietarie (DLL closed-source).

mzLab supera questo vincolo grazie a lettori dedicati sviluppati in linguaggio
Rust e compilati in WebAssembly (WASM). Questi moduli vengono eseguiti direttamente
all'interno del browser dell'utente (su qualsiasi sistema operativo: macOS, Linux,
Windows, iPadOS) sfruttando Web Workers dedicati:
- **Nessuna installazione locale:** non occorre installare driver, runtime .NET o DLL.
- **Privacy e sicurezza totale:** l'elaborazione avviene interamente sul dispositivo
  dello studente, senza inviare dati o spettri a server remoti.
- **Caricamento modulare e leggero:** il modulo WASM di uno specifico costruttore
  viene scaricato e attivato in memoria solo quando lo studente trascina un file
  di quella famiglia.

### 1.2 Formati Supportati Nativamente e Modalità di Caricamento
1. **Thermo Fisher `.raw`:**
   - I file `.raw` (Thermo Orbitrap, Q-Exactive, TSQ, LTQ, ecc.) vengono letti
     nativamente dal lettore Rust di mzLab. È sufficiente trascinare il file
     `.raw` nella zona di caricamento.
2. **SCIEX `.wiff` (3200 QTRAP, serie QTRAP, TripleTOF):**
   - mzLab legge nativamente le acquisizioni SCIEX. Affinché il caricamento avvenga
     con successo, è indispensabile selezionare o trascinare **contemporaneamente
     entrambi i file** della corsa:
     * Il file descrittore principale con estensione `.wiff`
     * Il file binario delle scansioni con estensione `.wiff.scan` (con il medesimo nome)
3. **Agilent `.D` (LC/MS e GC/MS):**
   - Le acquisizioni Agilent sono organizzate come cartelle con estensione `.D`
     (contenenti file di parametri e dati binari).
   - Possono essere caricate trascinando l'intera cartella `.D` oppure facendo clic
     sul pulsante **«Apri cartella»** (icona cartella accanto a «1. File dei dati»).
4. **Waters MassLynx `.raw`:**
   - I dati Waters MassLynx sono salvati in cartelle con estensione `.raw`.
   - Vengono letti nativamente trascinando o selezionando l'intera cartella di corsa.
5. **Bruker e Shimadzu:**
   - mzLab supporta inoltre le cartelle Bruker `.d` (formati TDF, TSF, BAF) e i file
     Shimadzu (`.qgd` e `.lcd`) tramite i rispettivi lettori WebAssembly.

### 1.3 Quando Può Servire MSConvert (ProteoWizard)?
I lettori Rust/WASM integrati in mzLab gestiscono direttamente la stragrande
maggioranza delle corse analitiche di laboratorio.
Il ricorso al programma esterno gratuito **ProteoWizard MSConvert** (disponibile
per Windows) resta utile solo come soluzione alternativa nei seguenti casi specifici:
- File generati da strumenti molto datati o con opzioni di acquisizione non standard;
- File proprietari con indici interni danneggiati o corruzioni binarie;
- Necessità di applicare filtri di pre-elaborazione particolari prima dell'apertura
  (es. algoritmi di centroidazione proprietari non simulabili in ambiente standard).

In tali casi, la conversione consigliata è in formato standard aperto `.mzML` a
64-bit per $m/z$, 32-bit per l'intensità e compressione zlib abilitata.

---

## 2. Comportamento del Righello Spettrometrico: Disattivazione Automatica

Un comportamento frequente che disorienta lo studente è la scomparsa improvvisa
delle linee di misura tracciate con il righello.

[Screenshot: Spettro di massa con una misura attiva (etichetta «Na⁺/H⁺») che
scompare istantaneamente non appena lo studente preme la freccia direzionale
sulla tastiera per avanzare alla scansione successiva. Lo spettro si aggiorna
e torna privo di segmenti di misura.]

### 2.1 Perché il Righello si Disattiva da Solo?
Nel codice di mzLab, una misurazione appartiene in modo univoco a una specifica
istanza visiva definita da:
$$\text{measKey} = [\text{zoom}, \text{r0}, \text{r1}, \text{file}, \text{livello}, \text{indice scansione}]$$

Se lo studente compie una delle seguenti azioni:
- Esegue un nuovo zoom o sposta l'asse delle masse;
- Cambia scansione (con le frecce **←** o **→**);
- Passa da una scansione singola a uno spettro medio di un intervallo;
- Seleziona un altro file dalla lista;

la chiave della misurazione non corrisponde più ai dati visualizzati e mzLab
cancella automaticamente le linee del righello.

Questa scelta progettuale evita che differenze di massa calcolate su uno spettro
rimangano sovraimposte su un altro spettro, generando false interpretazioni o
sovrapposizioni visive caotiche.

### 2.2 Buona Prassi di Misura
1. Navigare prima sul cromatogramma e selezionare la scansione o l'intervallo
   temporale ottimale (apice del picco o media).
2. Se necessario, ingrandire preventivamente la regione di $m/z$ di interesse.
3. Attivare il righello e compiere la misurazione da picco a picco.
4. Per rimuovere deliberatamente il righello, premere semplicemente il tasto
   **Esc**.

---

## 3. Modalità Amministratore (5 Clic Rapidi sul Logo)

In mzLab è presente una modalità ad accesso speciale denominata "Modalità
Amministratore".

### 3.1 Come si Attiva e Come si Riconosce
Facendo **cinque clic rapidi e consecutivi** (entro un intervallo di 600 ms
l'uno dall'altro) sul logo o sulla scritta "mzLab" in alto a sinistra,
l'interfaccia esegue un'animazione di ingrandimento e mostra una notifica
in basso a centro schermo: **«Sbloccato»**. La pagina si ricarica abilitando
la chiave `qqq.admin` nel localStorage.
Ripetendo la sequenza di cinque clic, la notifica mostra **«Modalità
amministratore disattivata»** e l'applicazione torna allo stato standard.

### 3.2 Perché Non È Visibile per gli Studenti?
La modalità amministratore è riservata ai docenti e agli sviluppatori della
piattaforma:
- Sblocca l'editor integrato per la scrittura e modifica dei capitoli di teoria.
- Abilita capitoli estesi e schemi metodologici avanzati non previsti nel
  programma didattico di base.
- Mostra strumenti diagnostici interni di benchmark e test dei worker.

Per uno studente universitario impegnato nell'analisi dei dati di laboratorio,
attivare per errore la modalità amministratore introduce voci di menu e comandi
di modifica che non hanno attinenza con l'esperienza didattica.
Se attivata accidentalmente, è consigliabile ripetere i cinque clic sul logo
per disattivarla.

---

## 4. Gestione dello Zoom: Trackpad vs Mouse Tradizionale

Il controllo della visualizzazione grafica presenta sfumature diverse a seconda
della periferica di puntamento utilizzata.

### 4.1 Utilizzo con Trackpad (Computer Portatili)
- **Pizzico a due dita (Pinch-to-zoom):** allargando o stringendo due dita sul
  trackpad, il browser genera un evento zoom centrato sulla posizione del
  puntatore.
- **Scorrimento laterale a due dita:** esegue la traslazione (*pan*) orizzontale
  dell'asse temporale o dell'asse delle masse senza modificare la scala.
- **Scorrimento verticale a due dita:** attenzione! Se il cursore non si trova
  sopra uno specifico asse, il movimento verticale scorre l'intera pagina web.

### 4.2 Utilizzo con Mouse a Rotella
- **Rotella semplice:** muove la pagina web verticalmente.
- **Ctrl + Rotella (oppure Cmd + Rotella su Mac):** effettua lo zoom
  bidirezionale centrato sul puntatore.
- **Trascina sui numeri degli assi (Metodo Consigliato):**
  * Trascinando il mouse verticalmente sui numeri dell'asse Y (a sinistra del
    grafico), si scala **esclusivamente l'altezza dei picchi** (intensità).
  * Trascinando orizzontalmente sui numeri dell'asse X (sotto il grafico),
    si allarga o restringe **esclusivamente l'intervallo temporale o di m/z**.
- **Maiusc + Trascina sul grafico:** trasla la finestra visualizzata.
- **Doppio clic o tasto Backspace:** ripristina la visualizzazione completa
  dell'intero asse (*Whole View*).
- **Cmd/Ctrl + Z:** annulla l'ultimo zoom tornando al precedente (fino a 15 livelli).

---

## 5. Archiviazione OPFS nel Browser, Messaggio di Quota e Persistenza

A differenza dei comuni siti web che caricano i file in memoria volatile RAM,
mzLab gestisce moli di dati considerevoli mediante l'**OPFS** (*Origin Private
File System*).

### 5.1 Come Funziona l'OPFS
L'OPFS è un file system isolato e sandboxato all'interno del browser dell'utente.
Quando si carica una corsa analitica (in formato aperto o proprietario):
1. I dati originali o indicizzati vengono memorizzati nello storage privato del browser.
2. Un processo Web Worker in background crea o gestisce l'indice binario (`.mzidx`) per
   accesso rapido casuale.
3. Ricaricando la pagina web o riaprendola il giorno successivo, **i file e la
   sessione di lavoro rimangono memorizzati nel browser** e non devono essere
   ricaricati da capo.

### 5.2 Messaggio di Quota e Permesso di Persistenza
I browser moderni (Chrome, Safari, Edge, Firefox) assegnano a ciascun sito
una quota di memoria locale (generalmente fino al 60% dello spazio libero su disco).
Tuttavia, se lo spazio su disco del computer è ridotto o se vengono caricati
molti file di grandi dimensioni, il browser può visualizzare una barra di notifica:
«Consentire a mzLab di archiviare dati in modo persistente?».
- **Cosa fare:** confermare e autorizzare la persistenza, per evitare che il
  browser elimini automaticamente i file e gli indici nei momenti di pulizia
  automatica del disco.

### 5.3 Come Liberare Spazio su Disco
Se si desidera cancellare i dati caricati in precedenza per liberare spazio:
- Dalla barra superiore di mzLab, aprire il menu di sessione e scegliere
  **«Nuova sessione» $\rightarrow$ «Reset completo»**.
- In alternativa, dai DevTools del browser: scheda *Application* $\rightarrow$
  *Storage* $\rightarrow$ pulsante **«Clear site data»**.
- Tutti i dati memorizzati in OPFS, IndexedDB e Cache Storage verranno rimossi
  senza lasciare residui nel computer.
