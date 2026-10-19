# LOC-1 — Scaricare ELENCO-1, contare MassIVE, provare gli oracoli sul Mac

Prompt pronto per la chat locale: copia tutto ciò che segue la riga «---».

---

Pacchetto LOC-1 del progetto «Rosina» (`docs/studi/ROSINA.md`): Scaricare ELENCO-1, contare MassIVE, provare gli oracoli sul Mac.
File da leggere oltre a quelli comuni: `docs/studi/ROSINA.md` §3 e D, `docs/studi/rosina/FONTI.md` (sezione ELENCO-1).

## Obiettivo
Avere sul Mac i 18 file di ELENCO-1 (≈ 0,8 GB) con checksum controllato, il conteggio dei formati di MassIVE che manca al censimento, e sapere quali oracoli girano sul Mac.

## Da fare
1. Elenco: righe complete in `mzlab-dati/scratchpad/rosina/ELENCO-1.tsv` (inviate anche per messaggio dall'Architetto). Per ogni voce: scarica con ripresa (`curl -C - -L --retry 5`), controlla byte e checksum (md5, SHA-1 o SHA-256 come scritto; `n/d` = calcola SHA-256 e scrivilo), metti nella struttura delle regole comuni, aggiungi la riga a `INDICE.tsv` (≤ 90 MB: anche nel privato) o a `GRANDI.tsv`.
2. Workbench ST004286: scarica solo 2 run `.d` (range HTTP sullo zip, o lo zip intero se il range non va) e l'mzML degli stessi run; Workbench ST003942 e ST003984: 2 run nativi + gli stessi mzML. Per gli zip annidati (ST001609) estrai 2 file.
3. MassIVE: conta i dataset pubblici per strumento e per estensione dei file (`.raw` file e cartella, `.wiff`, `.wiff2`, `.d`, `.lcd`, `.qgd`, `.mzML`, `.mzXML`, `.mgf`), con il metodo di `ROSINA.md` A (query e data scritte). Se l'API non risponde, FTP `massive.ucsd.edu` con un campione casuale di 300 dataset dichiarato come tale.
4. Oracoli: (a) Docker con l'immagine `proteowizard/pwiz-skyline-i-agree-to-the-vendor-licenses`: un file per produttore (Thermo, SCIEX, Agilent, Waters, Bruker, Shimadzu), `msconvert --mzML -z` e con `--filter "peakPicking vendor msLevel=1-"`; scrivi architettura del Mac (`uname -m`), versione di Docker, tempo, esito. (b) ThermoRawFileParser (release ufficiale, .NET) su un `.raw`: `-f 2` e `-m 0`. (c) `timsdata` (SDK Bruker per macOS, se esiste per l'architettura) su una cartella TDF: 1/K0 di una scansione. (d) Se (a) non va su Apple Silicon: dillo, con l'errore.
5. Licenze: testo della licenza di ogni fonte e di ogni oracolo in `dati/formati/_licenze/`.

## Atteso
File di ELENCO-1 sul Mac e nel privato (≤ 90 MB) con indice; tabella MassIVE; tabella «oracolo × produttore: va / non va / errore». Messaggio al Coordinatore.

## File reali
Le 18 voci di ELENCO-1; nient'altro.

## Regole comuni della chat locale
- Sei la chat locale sul Mac di Federico (`~/mzLab/`). Coordinatore = `session_01Au8wAn7udx8RMY9ggu8RmG`: a fine lavoro UN messaggio con `send_message` (al massimo 5 righe: cosa è scaricato o prodotto, byte totali, cosa non va, cosa resta). Se una scelta cambia il piano: domanda al Coordinatore con una raccomandazione.
- Scarichi SOLO le voci di un elenco preciso (URL, formato, byte, licenza, checksum atteso, scopo) arrivato dal Coordinatore o dall'Architetto, mai altro. Tetto: 20 GB in `~/mzLab/rosina-dati/` (controlla `du -sh` prima di ogni voce; oltre, fermati e chiedi). Un checksum che non torna: file in `~/mzLab/_cestino/<data>_checksum/`, voce segnata, avanti con la successiva.
- Struttura: `~/mzLab/rosina-dati/<produttore>/<strumento>/<accession>_<esperimento>/` (vocabolario degli esperimenti e colonne dell'indice: `docs/studi/ROSINA.md` D). Nel repository privato `Freeesta/mzlab-dati` vanno solo i file ≤ 90 MB, le verità e le righe di `INDICE.tsv`/`GRANDI.tsv`; i grandi restano sul Mac con la loro riga in `GRANDI.tsv`.
- Mai dati nel repository pubblico. Nessun nome di persona o di file del corpus nei messaggi che finiscono nel pubblico. Non cancelli file: li sposti in `~/mzLab/_cestino/<data>_<motivo>/`.
- Licenze: la licenza di ogni fonte e di ogni SDK usato va in `dati/formati/_licenze/` del privato. Gli SDK dei produttori (RawFileReader, `timsdata`, MassLynx SDK) si usano solo sul Mac come oracoli, mai copiati in un repository.
