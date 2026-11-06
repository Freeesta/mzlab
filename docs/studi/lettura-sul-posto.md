# Lettura sul posto dei file grandi (Chromium): nota di progetto, senza codice

Scopo: un .raw da 1,6 GB o un lotto di mzML da decine di GB non deve più essere copiato nello spazio del browser (OPFS). Si legge dal disco dell'utente, a fette, come fa già l'app desktop. Riferimento: PR #2802 (quota), punto 5 del piano.

## 1. Cosa fa oggi la copia e perché c'è
- Ogni file finisce in `mzlab/files/` (OPFS), copiato a blocchi da 8 MiB (`archivio.writeNow`). Accanto stanno l'indice (`idx/`, ~0,7× per gli mzML), più un file di appoggio durante la costruzione. Il picco è ≥1,8× il file.
- La copia serve a tre cose:
  - (a) riaprire la sessione dopo un ricaricamento senza chiedere di nuovo il file;
  - (b) byte stabili: il file sul disco può cambiare o sparire;
  - (c) letture sincrone veloci dai worker (`lettura-sync.js`, SyncAccessHandle read-only).
- Costo: lo spazio. Chrome può rifiutare ben sotto la quota che dichiara (caso reale: rifiuto a ~800 MB con 10,7 GB dichiarati).

## 2. Proposta: il `File` dell'utente come sorgente, l'indice in OPFS
- **Sorgente.** Il `File` (o `FileSystemFileHandle.getFile()`) viene passato ai worker per structured clone, e i worker lo leggono con `FileReaderSync.readAsArrayBuffer(file.slice(a, b))`.
  - Questa via esiste già come ripiego in `openSource` (`viaBlob`) e in `lettori-worker.js` / `libreria-cartella.js`.
  - Il cambiamento è quindi: «non copiare, e consegnare il Blob come Fonte».
- **Indice.** L'indice resta in OPFS: è piccolo rispetto al file (~0,7× negli mzML, molto meno nei .raw) ed è ciò che rende veloce la navigazione. Il picco scende da ≥1,8× a ~0,7×. Per un .raw da 1,6 GB si passa da ~2,9 GB a poche centinaia di MB.
- **Quando si applica.** Solo su Chromium (dove esiste `FileSystemFileHandle` dall'ingresso, vedi sotto), e solo per i file sopra una soglia (per esempio ≥ `BIG` = 50 MB) o quando il controllo preventivo di #2802 dice che non entrano. I file piccoli continuano a essere copiati: nessuna richiesta di permesso per il caso comune.
- **Safari e Firefox.** Tengono la copia OPFS. Firefox non ha `getAsFileSystemHandle`/`showOpenFilePicker` (si potrebbe leggere il `File` della sessione, ma senza handle non si riapre dopo un ricaricamento). Safari 16.4 non ha `showOpenFilePicker`. Per loro restano l'avviso di #2802 e la copia di sempre.

## 3. Da dove arriva l'handle
- **Trascinamento:** `DataTransferItem.getAsFileSystemHandle()` (Chromium 86+). Va chiamato in modo sincrono nel `drop`, prima di qualsiasi `await`.
- **Pulsante «Apri»:** `showOpenFilePicker({ multiple: true })` al posto di `<input type=file>` su Chromium. L'input resta il ripiego.
- **Cartella (lotto grande):** `showDirectoryPicker()` dà un solo permesso per tutti i file. Per 82 file è il modo giusto: una richiesta, non 82.

## 4. Permessi e ricaricamento (il vincolo principale)
- Nella sessione in corso il `File` si legge senza chiedere nulla.
- Dopo un ricaricamento serve l'handle: va salvato in IndexedDB (gli handle sono clonabili) nella riga del manifesto (`qqq-db.js`, store `manifest`).
  - All'apertura, `handle.queryPermission({mode:"read"})` dà di solito «prompt».
  - Per leggere bisogna chiamare `requestPermission` dentro un gesto dell'utente.
  - Quindi una sessione ripristinata mostra le righe «da riaprire» con un pulsante «Consenti la lettura dei file» (un solo clic per cartella o per gruppo di handle).
  - Chrome 122+ offre «Consenti sempre» (permessi persistenti) per i siti installati o già usati; non ci si può contare.
- **Stato della riga.** Una riga di un file non copiato ha uno stato in più: «letto dal disco, permesso da rinnovare». Non è un errore e va detto con calma, come chiede la regola di Federico.
- **File cambiato o spostato.** `getFile()` dà un nuovo snapshot; un `File` vecchio letto dopo una modifica lancia `NotReadableError`.
  - Al ripristino si confrontano dimensione e `lastModified` con il manifesto, e l'impronta (impronta.js) sui primi blocchi.
  - Se non coincidono: «il file è cambiato sul disco, ricaricalo», e l'indice vecchio si scarta.

## 5. Cosa cambia nei worker e nei lettori
- **`rust-worker.js` / `rust-nativo.js` / `idx-worker.js`.** La Fonte arriva come Blob invece che come nome in `files/`.
  - `openSource(kind, name, blob)` ha già il ramo Blob; serve un'entrata «solo Blob» che non cerchi la copia.
  - Le letture del motore (`leaseRead` → -1 → Blob) seguono già quel percorso.
- **Prestazioni da misurare.** `FileReaderSync` su fette passa dal processo del browser (IPC per ogni fetta), più lento della SyncAccessHandle.
  - Fette da 8 MiB o più per l'indice; la navigazione legge dall'indice in OPFS, quindi l'effetto si sente soprattutto in costruzione.
  - Misura da fare prima della PR: tempo di indice di un mzML sintetico da 1 GB, copia+indice contro indice da Blob.
- **Lettori di formato** (`lettori-worker.js`, .raw Thermo e altri): leggono già un Blob con `FileReaderSync`. Il .raw oggi è copiato «per stabilità»; con l'handle si salta la copia.
- **Più schede.** Oggi la seconda scheda legge la copia OPFS. Con la lettura sul posto non ha il `File`: deve ricevere l'handle da IndexedDB e chiedere il permesso nella propria scheda (lo stato di `BroadcastChannel` resta valido). È un caso da prevedere nel test.
- **Esportazioni e mzFlow** che rileggono la sorgente: stessa Fonte. Nessuna scrive nella sorgente: si apre solo in lettura (`mode: "read"`), mai `createWritable`.
- **Pulizia** (`pulizia.js`): un indice il cui file non è in `files/` oggi è «orfano dopo una settimana». Con la lettura sul posto la regola va estesa: si guarda anche il manifesto (riga con handle), altrimenti la pulizia cancellerebbe gli indici validi.

## 6. Parità con l'app desktop
- L'app desktop e la CLI leggono già sul posto: è il loro modo normale, nessuna copia. Non c'è una metà nativa da consegnare.
- Il test di parità resta quello dei byte di indice e risultati: stesso file, stesso `.mzidx` da Blob e da copia OPFS (e dal nativo). Va aggiunto: un indice costruito da Blob è byte per byte uguale a quello costruito dalla copia.

## 7. Tetti
- **Peso di avvio** (`test_peso_avvio`, 1.900.000 / 110.000 per file): niente di nuovo all'avvio. La scelta dell'handle va nel modulo già caricato su richiesta (`quota-lotto.js` o un nuovo modulo ES via `carica("m:…")`), e il ripristino dei permessi in un modulo caricato solo se il manifesto ha handle. `pannelli.js` non si tocca.
- **Wasm:** nessun cambiamento ai `.wasm` (le letture passano già da callback JS).
- **Riga e area dei documenti:** una voce in `dati-carico.md` (sorgente sul posto) e una in `memoria.md` (pulizia con handle).

## 8. Stima in PR (ognuna con test rosso prima)
1. **Fonte da Blob nei worker senza copia** (solo sessione corrente, nessun ripristino), attiva su Chromium per i file che non entrano. Test: e2e con la quota CDP di #2802; un lotto «non entra» si carica tutto, senza `.part` né copie in `files/`; parità dell'indice Blob contro copia. Misura: tempi di indice.
2. **Handle dall'ingresso** (drop con `getAsFileSystemHandle`, `showOpenFilePicker`, cartella) salvati nel manifesto. Test: righe con handle; Firefox/Safari invariati.
3. **Ripristino dopo ricaricamento** con «Consenti la lettura» (un clic) e controllo di dimensione, data e impronta. Test: ricaricamento, permesso negato (stato calmo), file cambiato.
4. **Pulizia e più schede** con handle. Test: un indice di un file sul posto non viene pulito; una seconda scheda chiede il permesso e apre.
5. **Testi e tour:** l'avviso di #2802 offre «Leggi dal disco senza copiare» su Chromium al posto di «Carica i primi k». Test: tour valido, IT+EN.

Ordine consigliato: 1 → 2 → 3 (il valore per Federico arriva già con 1 per la sessione corrente), poi 4 e 5. Rischio maggiore: la velocità di `FileReaderSync` in costruzione e l'esperienza del permesso al ripristino; per questo la misura del punto 1 decide se proseguire.

## 9. Alternativa minore: copia in streaming mentre si indicizza
- Oggi un .raw da 1,6 GB si copia tutto, poi parte l'indice. Si potrebbero fondere i due passi: ogni blocco da 8 MiB letto dal `File` va sia in OPFS sia al lettore e al costruttore dell'indice (come fa già `after(piece, offset)` per le impronte).
- Vantaggi: la prima vista arriva prima (circa un passaggio sul file in meno) e non servono permessi.
- Limiti: lo spazio resta 1,8×, quindi non risolve il rifiuto a ~800 MB. Inoltre il lettore .raw salta nel file (indici in coda), cosa che non si concilia con un flusso in ordine. Per gli mzML l'indice è già in ordine, per il .raw no.
- Conclusione: vale come miglioramento di velocità per gli mzML, non come risposta alla quota. Per il .raw la risposta è la lettura sul posto (sezioni 2-8).
