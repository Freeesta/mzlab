# Guida rapida per studenti

mzLab serve a guardare i dati LC-MS dell'esperienza 3 e a cercare i prodotti di trasformazione (TP). Mostra i dati: il ragionamento è tuo. A risoluzione unitaria **un m/z è un candidato, non un'identificazione**.

## Aprire il programma
1. Apri <https://freeesta.github.io/mzlab/> con un browser recente (Chrome, Edge, Firefox o Safari) su **computer o tablet**. Non si installa nulla.
2. La prima volta il browser scarica il motore di calcolo (qualche decina di secondi, serve la rete); poi resta in memoria e le volte successive è più rapido.
3. Da **smartphone** si apre solo la Teoria: per Dati, Disegno e Pratica usa un computer o un tablet.

## Caricare i dati
- Trascina i file `.mzML` (e, se ce l'hai, il `.dam` del metodo) nella schermata di caricamento, controlla tempi e tipi, premi **Carica dati**.
- Senza file: **«Prova con i file di esempio»**.
- I `.wiff` del 3200 QTRAP si aprono direttamente: trascina il `.wiff` insieme al suo `.wiff.scan` (stesso nome) e il sito crea un mzML per campione, scaricabile con «Scarica mzML». Gli altri `.wiff` (non 3200 QTRAP) si convertono con ProteoWizard MSConvert o con il convertitore SCIEX su Windows (la guida è nella schermata di caricamento; meglio il formato in profilo).
- I tuoi file **non lasciano il computer** (vedi [Privacy](privacy.md)).

## Cosa trovi
- **Dati**: cromatogrammi (TIC, XIC, MRM), spettri MS e MS2, mappa RT×m/z, integrazione dei picchi con tabella esportabile.
- **Disegno**: editor chimico per molecole e vie di trasformazione.
- **Teoria** e **Pratica**: capitoli, glossario, esercizi con correzione. La guida completa è il capitolo A della Teoria («Come si usa mzLab»).
- Il pulsante **«i»** in alto apre le informazioni; il **tour guidato** mostra i comandi principali.

## Se qualcosa non va
- Ricarica la pagina: sessione e disegni si ripristinano.
- Se il sito sembra «vecchio», ricarica forzando (Ctrl+Maiusc+R, oppure Cmd+Maiusc+R su Mac).
- Annota il passaggio che porta all'errore e il browser usato, poi segnalalo con una [issue](https://github.com/Freeesta/mzlab/issues/new/choose) **senza allegare dati di laboratorio**. Durante le lezioni chiedi prima al docente.
