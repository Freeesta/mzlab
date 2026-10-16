# Modulo 3 — Criticità e Punti di Attenzione dell'Interfaccia

Questo modulo raccoglie e chiarisce gli aspetti dell'interfaccia di mzLab
che possono generare dubbi negli studenti o che si discostano dai tradizionali
software spettrometrici desktop.

---

## 1. Gestione dei Formati Proprietari (.raw, .wiff) e Conversione

Uno dei primi ostacoli per gli studenti riguarda il caricamento diretto dei file
generati dagli spettrometri di laboratorio (Thermo, Sciex, Agilent, Waters).

### 1.1 Perché il Browser Non Legge Direttamente Tutti i File Proprietari?
Gli spettrometri commerciali salvano i dati in formati binari proprietari
(ad esempio i file `.raw` di Thermo Fisher o i file `.wiff` di Sciex).
I costruttori forniscono librerie di decodifica (*RAWFileReader*, *WiffReader*)
che sono librerie dinamiche Windows closed-source (DLL a 64 bit per ambiente
.NET).

Poiché mzLab viene eseguito interamente all'interno della sandbox di sicurezza
del browser web (su qualsiasi sistema operativo: macOS, Windows, Linux, tablet)
senza componenti nativi installati, **non è consentito al browser caricare DLL
di sistema**.

### 1.2 Eccezione Integrata: File .wiff del 3200 QTRAP
mzLab include un lettore WebAssembly specializzato per i file generati dallo
spettrometro Sciex 3200 QTRAP. Affinché il caricamento avvenga con successo,
è indispensabile trascinare **contemporaneamente entrambi i file** associati
alla corsa:
- Il file principale con estensione `.wiff`
- Il file gemello con estensione `.wiff.scan` (avente lo stesso identico nome base)

Se viene trascinato solo il file `.wiff` senza il relativo `.wiff.scan`, il
programma segnalerà che la scansione binaria è incompleta.

### 1.3 Conversione Tramite MSConvert (ProteoWizard)
Per tutti gli altri strumenti (es. Thermo Q-Exactive, Orbitrap, strumenti
Agilent o Waters), i dati devono essere preventivamente convertiti nel formato
standard aperto `.mzML` utilizzando il software libero **ProteoWizard MSConvert**
(disponibile per Windows).

**Parametri consigliati per MSConvert:**
1. *Output format:* **mzML**.
2. *Binary encoding precision:* **64-bit** per $m/z$ e **32-bit** per l'intensità.
3. *Compression:* spuntare **zlib compression** (riduce le dimensioni del file
   fino al 70%).
4. *Filters (Filtri):*
   - Per studi didattici sui profili continui e larghezza picco: lasciare in
     formato profilo (*profile mode*).
   - Per file molto pesanti o algoritmi di estrazione veloci: aggiungere il
     filtro `Peak Picking` (selezionando `Vendor` per `msLevel = 1-`).

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
Quando si carica un file `.mzML`:
1. Il file originale viene copiato nello storage privato del browser.
2. Un processo Web Worker in background crea un indice binario (`.mzidx`) per
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
