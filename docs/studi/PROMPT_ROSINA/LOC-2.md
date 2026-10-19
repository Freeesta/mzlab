# LOC-2 — Oracoli su tutto il corpus: mzML di riferimento, JSON di TRFP, 1/K0, verità

Prompt pronto per la chat locale: copia tutto ciò che segue la riga «---».

---

Pacchetto LOC-2 del progetto «Rosina» (`docs/studi/ROSINA.md`): Oracoli su tutto il corpus: mzML di riferimento, JSON di TRFP, 1/K0, verità.
File da leggere oltre a quelli comuni: `docs/studi/ROSINA.md` §4 e F, `tools/rosina/` (dopo R-00).

## Obiettivo
Ogni file nativo del corpus (privato e `~/mzLab/rosina-dati/`) ha la sua verità `verita/1` prodotta da un oracolo con versione dichiarata.

## Da fare
1. Per ogni nativo: msconvert (Docker, o il PC Windows se il Docker non va) due volte, senza peak picking e con `peakPicking vendor msLevel=1-`, in `riferimenti/msconvert-<versione>/`; per Waters con IMS anche senza `combineIonMobilitySpectra`; per Bruker con e senza.
2. Thermo: anche ThermoRawFileParser `-f 2 -m 0` in `riferimenti/trfp-<versione>/`. Bruker: tabella di 1/K0 di `timsdata` (ogni scansione di 3 frame per file) in `riferimenti/timsdata-<versione>/`.
3. `python3 tools/rosina/verita.py` (e `verita_trfp.py`, `verita_timsdata.py`) per ogni riferimento: `scratchpad/rosina/verita/<percorso sotto dati/formati>/<file>.verita.json`. Riferimenti > 90 MB: nel privato solo la verità, l'mzML resta sul Mac.
4. Tabella finale `dati/formati/ORACOLI.md` nel privato: file × oracolo × esito (va, errore con il messaggio, tempo). Un file che nessun oracolo apre è un'informazione: resta nell'indice con `riferimento=nessuno`.

## Atteso
Verità per tutto il corpus apribile; tabella degli oracoli; commit nel privato (mai nel pubblico). Messaggio al Coordinatore con i numeri per produttore.

## File reali
Tutto il corpus (`dati/formati/` del privato) e `~/mzLab/rosina-dati/`.

## Regole comuni della chat locale
- Sei la chat locale sul Mac di Federico (`~/mzLab/`). Coordinatore = `session_01Au8wAn7udx8RMY9ggu8RmG`: a fine lavoro UN messaggio con `send_message` (al massimo 5 righe: cosa è scaricato o prodotto, byte totali, cosa non va, cosa resta). Se una scelta cambia il piano: domanda al Coordinatore con una raccomandazione.
- Scarichi SOLO le voci di un elenco preciso (URL, formato, byte, licenza, checksum atteso, scopo) arrivato dal Coordinatore o dall'Architetto, mai altro. Tetto: 20 GB in `~/mzLab/rosina-dati/` (controlla `du -sh` prima di ogni voce; oltre, fermati e chiedi). Un checksum che non torna: file in `~/mzLab/_cestino/<data>_checksum/`, voce segnata, avanti con la successiva.
- Struttura: `~/mzLab/rosina-dati/<produttore>/<famiglia>/<accession>_<esperimento>/` (stesse cartelle di `dati/formati/` del privato; verità in `scratchpad/rosina/verita/`) (vocabolario degli esperimenti e colonne dell'indice: `docs/studi/ROSINA.md` D). Nel repository privato `Freeesta/mzlab-dati` vanno solo i file ≤ 90 MB, le verità e le righe di `INDICE.tsv`/`GRANDI.tsv`; i grandi restano sul Mac con la loro riga in `GRANDI.tsv`.
- Mai dati nel repository pubblico. Nessun nome di persona o di file del corpus nei messaggi che finiscono nel pubblico. Non cancelli file: li sposti in `~/mzLab/_cestino/<data>_<motivo>/`.
- Licenze: la licenza di ogni fonte e di ogni SDK usato va in `dati/formati/_licenze/` del privato. Gli SDK dei produttori (RawFileReader, `timsdata`, MassLynx SDK) si usano solo sul Mac come oracoli, mai copiati in un repository.
