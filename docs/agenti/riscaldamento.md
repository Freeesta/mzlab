# Riscaldamento in background della cartella puntata
Dopo che la cartella puntata (`CartPunt`, `CartPuntDesk`) è elencata, i file mzML vengono preparati di nascosto così che il clic successivo sia un colpo di cache. Niente cambia in ciò che lo studente vede.

## Costo di apertura misurato (sito, Chromium headless, file sintetici)
- Cartella TRASCINATA o scelta con il dialogo: `upload()` copia in OPFS, indicizza e riempie già tutto in modo visibile; lo studente preme «Carica dati» (`#opbtn`). Qui non manca nulla.
- Cartella PUNTATA: la copia in OPFS e l'indice partono solo al clic. Clic → file in tabella: 11 MB 1179 ms a freddo, 125 ms a caldo; 40 MB 2443 ms a freddo, 123 ms a caldo. Il costo a freddo cresce con i MB (lettura, copia in OPFS, SHA-256/SHA-1, indice con TIC dentro il `.mzidx`); a caldo resta solo `lr_entry`.
- `.raw` nativo: ~0,2-0,3 s per 32 MB, `.wiff(+.scan)` e `.D` non misurabili senza dati veri: non vengono riscaldati (guadagno piccolo o ignoto).

## Come funziona
- `riscalda.js` (modulo ES, caricato al bisogno da `cartella-puntata.js`): coda nell'ordine dell'elenco, un file alla volta, solo `.mzML` di dati, nomi doppi saltati. Parte quando la pagina è quieta (scheda visibile, `UPBUSY` zero, nessun input da 1,5 s). Salta telefono, `saveData`, `deviceMemory` < 4, batteria < 25 % non in carica. Limite: 128 MB sul sito (`prepare_upload` è sincrono nel worker), 1 GiB su desktop. Errori silenziosi, nessun testo nuovo.
- Sito: `/api/warm` → `lr_warm` (`rust-scalda.js`, lazy nel worker) esegue `lr_put` + `lr_index` e scrive il marcatore `warm/<nome>.json` (size, mtime); `lr_boot` nasconde i file con marcatore. Budget: un quarto della quota con 256 MB di scorta; un caricamento vero espelle i file caldi (`makeRoom`).
- Desktop: `lr_warm` (`rpc_warm.rs`) costruisce l'indice senza voce nel negozio; `lr_boot` salta i nomi caldi.
- All'apertura `upload()` chiama prima `lr_warm_claim`: se dimensione e mtime coincidono salta copia e indice (stessi byte dell'apertura a freddo), altrimenti il percorso normale. Un clic su un file in coda lo toglie dalla coda (`claim`) o attende quello in corso: lavoro mai doppio.
- `forget()` della cartella ferma la coda; `lr_clear`, `lr_remove` cancellano i caldi.

## Prove
`tests/test_riscalda.py` (coda, ordine, pause, limiti, errori), `e2e_riscalda` (ordine, nascosto dopo il ricaricamento, hit senza fase di carico, stesso SHA-256 dell'indice e stessa TIC a freddo e a caldo), `cargo test -p mzlab-desktop --test rpc`.
Limiti noti: il lavoro in corso su un altro file non si annulla (finisce in silenzio); i caldi di una cartella dimenticata restano nascosti ed espellibili.
