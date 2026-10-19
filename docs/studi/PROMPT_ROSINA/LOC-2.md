# LOC-2 — Oracoli su tutto il corpus: TRFP sul Mac, workflow `oracoli.yml` per msconvert e `timsdata`, verità

Prompt pronto per la chat locale: copia tutto ciò che segue la riga «---».

---

Pacchetto LOC-2 del progetto «Rosina» (`docs/studi/ROSINA.md`): Oracoli su tutto il corpus: TRFP sul Mac, workflow `oracoli.yml` per msconvert e `timsdata`, verità.
File da leggere oltre a quelli comuni: `docs/studi/ROSINA.md` §4 e F, `tools/rosina/` (dopo R-00).

## Obiettivo
Ogni file nativo del corpus (privato e `~/mzLab/rosina-dati/`) ha la sua verità `verita/1` prodotta da un oracolo con versione dichiarata.

## Da fare
1. msconvert: non gira sul Mac arm64. Avvia il workflow `oracoli.yml` del privato (scritto in R-00) sui file del corpus, a lotti, e controlla che ogni nativo abbia la sua verità in `scratchpad/rosina/verita/`; se il workflow fallisce per un produttore, scrivi l'errore e usa l'mzML pubblicato nello stesso deposito (V2), se c'è.
2. Thermo: ThermoRawFileParser sul Mac (`dotnet ThermoRawFileParser.dll -f 2 -m 0`), JSON e mzML in `riferimenti/trfp-<versione>/` accanto al nativo (≤ 90 MB) o solo sul Mac, poi `verita_trfp.py`. Bruker: m/z e 1/K0 vengono dal workflow (`libtimsdata` per Linux); se non va, «oracolo assente» per m/z e 1/K0 e lo scrivi in `ORACOLI.md`.
3. `python3 tools/rosina/verita.py` (e `verita_trfp.py`) per i riferimenti prodotti sul Mac: `scratchpad/rosina/verita/<percorso sotto dati/formati>/<file>.verita.json`. Riferimenti > 90 MB: nel privato solo la verità.
4. Tabella finale: aggiorna `coordinamento/dati_esterni/rosina/ORACOLI.md` con file × oracolo × esito (va, errore con il messaggio, tempo). Un file che nessun oracolo apre resta nell'indice con `riferimento=nessuno`.

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
