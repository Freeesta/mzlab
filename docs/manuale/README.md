# Manuale per gli Studenti — mzLab

**Piattaforma didattica per la Spettrometria di Massa e l'Analisi LC-MS**  
*Versione applicazione:* mzLab 1.0.1  
*Data di rilascio:* 16 ottobre 2026  
*Commit di riferimento:* `b79e8cb`  
*URL pubblico dell'applicazione:* <https://freeesta.github.io/mzlab/>  
*Ambito didattico:* Corsi di Metodi Spettroscopici e Spettrometria di Massa, Lauree in Chimica (UniTO).

---

## 1. Introduzione e Obiettivi Pedagogici

mzLab è un'applicazione web scientifica interamente client-side progettata
per l'esplorazione, l'elaborazione e l'interpretazione di dati cromatografici
e spettrometrici ad alta e a bassa risoluzione (LC-MS, MS/MS tandem, DDA, MRM).

Il principio cardine del laboratorio è che il programma fornisce la
visualizzazione e gli strumenti matematici di calcolo, ma **l'interpretazione
chimica e l'attribuzione spetta esclusivamente allo studente**. A risoluzione
unitaria, un valore di $m/z$ rappresenta un *candidato*, non un'identificazione
certa: formulare ipotesi strutturali corrette richiede l'integrazione di
tempo di ritenzione, profili di estrazione cromatografica (XIC), masse esatte,
pattern isotopici e frammentazioni tandem MS/MS.

---

## 2. Configurazione Iniziale del Browser e Gestione Cache

mzLab funziona direttamente nel browser sfruttando WebAssembly (WASM),
Web Workers e il file system virtuale locale OPFS (*Origin Private File System*).
Non richiede installazione su disco né invio di dati a server remoti: tutti i
file caricati rimangono rigorosamente all'interno del computer locale.

Poiché l'applicazione utilizza un Service Worker per consentire il
funzionamento offline e velocizzare i caricamenti successivi, quando viene
rilasciata una nuova versione o si riscontrano comportamenti anomali
è indispensabile pulire la cache locale.

### 2.1 Ricaricamento Forzato (Hard Reload)
Per forzare il browser a bypassare la cache HTTP e riscaricare il codice:
- **macOS (Safari, Chrome, Firefox, Edge):** premere `Cmd + Maiusc + R`
  (oppure tenere premuto `Shift` e cliccare sul pulsante di ricarica).
- **Windows / Linux:** premere `Ctrl + F5` oppure `Ctrl + Shift + R`.

### 2.2 Reset Completo dall'Interfaccia di mzLab (Consigliato)
Se il software continua a mostrare versioni precedenti dei file o indici OPFS
corrotti:
1. Nella barra superiore o nel menu principale dell'applicazione, fare clic
   su **«Nuova sessione»**.
2. Selezionare l'opzione **«Reset completo»** (oppure avviare l'URL con
   `https://freeesta.github.io/mzlab/?reset=all`).
3. L'applicazione eliminerà in modo sicuro gli indici OPFS, i database
   IndexedDB temporanei e registrerà nuovamente il Service Worker.

### 2.3 Pulizia Manuale Tramite Strumenti per Sviluppatori (DevTools)
In alternativa, è possibile cancellare direttamente lo stato locale dal browser:
1. Aprire gli Strumenti per Sviluppatori:
   - Su macOS: `Cmd + Alt + I` (o `F12`).
   - Su Windows/Linux: `Ctrl + Shift + I` (o `F12`).
2. Recarsi nella scheda **«Application»** (o **«Applicazione»**).
3. Nel menu laterale sinistro:
   - Fare clic su **«Service Workers»** e cliccare su **«Unregister»**
     accanto al worker di mzLab.
   - Fare clic su **«Storage»** (Archiviazione) e premere il pulsante
     **«Clear site data»** (Cancella dati del sito). Questa operazione svuota
     immediatamente IndexedDB, Cache Storage e lo spazio OPFS.
4. Ricaricare la pagina per ottenere una sessione completamente pulita.

---

## 3. Struttura del Manuale

Il presente manuale è suddiviso in moduli didattici progressivi:

1. **[01. Navigazione e Funzionalità](01_navigazione_e_funzionalita.md)**  
   Guida dettagliata alla Vista Dati: caricamento dei file `.mzML`,
   gestione del cromatogramma (TIC, EIC/XIC, BPC), selezione degli intervalli
   temporali, navigazione da tastiera scansione per scansione, pannello dello
   spettro di massa, righello di misura $\Delta m/z$ con riconoscimento
   scambi di addotto, confronto con pattern isotopici teorici ed estrazione
   e integrazione dei picchi.

2. **[02. Esercizi Guidati](02_esercizi_guidati.md)**  
   Tre esercitazioni pratiche complete svolte passo-passo su composti noti:
   - *Esercizio 1:* Determinazione di massa esatta, addotti e pattern
     isotopico del Paracetamolo ($C_8H_9NO_2$).
   - *Esercizio 2:* Ricerca e discriminazione di fotoprodotti di
     trasformazione della Carbamazepina ($C_{15}H_{12}N_2O$).
   - *Esercizio 3:* Analisi di spettri tandem MS/MS di derivati fenolici,
     identificazione delle perdite neutre diagnostiche ($-CO$, $-H_2O$)
     con il righello $\Delta m/z$.

3. **[03. Criticità e Punti di Attenzione dell'Interfaccia](03_criticita_interfaccia.md)**  
   Approfondimento sugli aspetti dell'interfaccia che richiedono particolare
   cura: conversione dei formati proprietari (.raw, .wiff), comportamento del
   righello di misura, sblocco della modalità amministratore, differenze
   di puntamento tra trackpad e mouse, e archiviazione OPFS nel browser.

---

## 4. Convenzioni Grafiche e Pedagogiche Adottate

Nel testo vengono utilizzate le seguenti convenzioni:

- `[Screenshot: ...]`  
  Descrizioni testuali dettagliate della vista a schermo, inserite per
  guidare visivamente lo studente anche senza dover consultare immagini esterne.
- **Tasti e combinazioni:** evidenziati in grassetto (es. **←**, **→**,
  **Maiusc + ←**, **Spazio**, **Esc**, **Cmd/Ctrl + Z**).
- *Formule e parametri:* in corsivo o notazione matematica (es. *m/z*, RT,
  $C_8H_9NO_2$, $[M+H]^+$, $[M+Na]^+$).
- **Riquadri di Attenzione Didattica:** evidenziano regole analitiche
  fondamentali (ad es. la parità dell'azoto o la distinzione tra isomeri
  e isobari).
