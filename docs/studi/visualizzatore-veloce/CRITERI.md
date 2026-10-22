# Visualizzatore veloce: criteri, numeri da battere, prove

Obiettivo (Federico): un visualizzatore MS che batta Xcalibur/FreeStyle sul tempo di risposta, in bassa e in alta risoluzione, con zero perdita nei valori mostrati. Funzione prioritaria: un pulsante che estrae subito lo XIC dell'ione scelto nello spettro.
Questo file è il contratto dei pacchetti V-0…V-6 (prompt in questa cartella, ordine in `LEGGIMI.md`): ogni PR riporta i numeri dei criteri che tocca, anche quelli che non passano. Nessun numero di Xcalibur/FreeStyle è inventato: solo quelli che Federico misura in locale (§7).

## 1. A chi serve
- **Studenti dell'esperienza 3** (3200 QTRAP, bassa risoluzione, nel browser su portatili qualsiasi): 5-20 file Full Scan da ~1500 scansioni × ~500 centroidi. Gesto tipico: clic sul TIC → spettro → «di chi è questo picco nel tempo?» → XIC. Oggi l'XIC dallo spettro c'è ma è nascosto (menu del tasto destro, tasto «x» sul picco sotto il puntatore, doppio clic quando non c'è una MS2).
- **Ricerca in alta risoluzione** (Orbitrap/Q-TOF, DDA, 15+ file da ~80 MB, 3000-6000 MS1): stesso gesto con tolleranza in ppm e molti file sovrapposti; oggi l'XIC su 15 file HR costa ~500 ms (misure-diario: 475 000 numeri scritti e riletti in JSON).
- **App desktop con file grandi** (≥ 2 GB di `.raw`, indice mappato con `memmap2`): la stessa pagina con il motore nativo.
I principi di AGENTS.md restano: il programma non dà risposte nel mondo LR (un'area si mostra solo dove l'integrazione è già uno strumento dello studente, §6 D2); a risoluzione unitaria un m/z è un candidato.

## 2. Cosa c'è già (verificato nel codice, `origin/main` `ea8b19c`)
- **Indice per m/z**: il `.mzidx` (`docs/agenti/mzidx.md`) è già un indice colonnare SoA, array allineati a 64 byte: per ogni sezione livello × polarità tutti i picchi ordinati per m/z (`mz`, `inten`, `pos` = scansione) con un recinto ogni 16 384 picchi. Un XIC è due ricerche binarie più una somma per scansione (`Routes::xic_raw` → `Index::search`, `sum_window`). È più preciso di un indice a secchi da 1 Da (finestra esatta, nessun filtro a valle).
- **Sonda nativa** (questa chat, `--release` del workspace = `opt-level="z"`, container cloud 4 CPU, file sintetico `mzml_tempi genera 8000 4000` → `.mzidx` di 451,6 MB, 27,1 M picchi, 200 XIC): `xic_raw` 0,11 ms mediana, 0,31 ms al 95°; la stessa XIC come JSON (`Routes::xic`, arrotondata) 2,5 / 3,1 ms. **Il lavoro sull'indice è 1/20 del costo: il resto è la scrittura del JSON**, e nel browser si aggiungono la stringa, il trasporto e `JSON.parse`.
- **Trasporto**: `rust-worker.js` restituisce già `Float64Array` trasferiti e riciclati (`recycle`) per TIC e XIC senza indice (`write_trace`, layout `[rt(n), y(n)]`, RT a 4 decimali e y a 1 come Python); `api/scanbin` è già una rotta binaria (`scroll.js` `scBin`, `browser.js` `r.buf`); il desktop risponde in binario a `tic`, `xic`, `spectra` (`rust-desktop.md`). L'XIC dall'indice (`Engine.index_xic`, `rust-bridge.js:192`) invece è JSON.
- **Decimazione per pixel**: il disegno dei cromatogrammi (`grafici.js`, ciclo con `flush`) e dei profili degli spettri (`picchi.js`) tiene già per ogni colonna di pixel primo, minimo, massimo e ultimo (M4: nessun apice perso), ma scorre tutti i punti anche fuori dalla finestra di zoom; i bastoncini dei centroidi si disegnano uno per uno. Nel motore `analysis::minmax_indices` (parametro `px`) c'è, usato solo sulla via senza indice.
- **XIC dallo spettro**: azioni `spec.xic` (menu, tasto «x»), `spec.peak` (doppio clic/Invio: MS2 se c'è, altrimenti XIC), `spec.overlay` (`spettro.js`); `xicDirect(mz, after, panel)` (`pannelli.js`) apre il pannello XIC sotto lo spettro con la finestra LR `[n−0,2; n+0,8]` (`XIC_BELOW`) o la finestra ppm del profilo HR (`HR.ionTrace`, `HR.prof(f).tol`, regolabile in Impostazioni; vedi D7). Cursore della scansione corrente sui cromatogrammi (`p.cur`), integrazione automatica col tasto (`scorc.integra`).
- **Vincoli**: SharedArrayBuffer non c'è nel sito (GitHub Pages non manda COOP/COEP): niente fili nel `.wasm` del sito, solo più worker; il motore `.wasm` ha un tetto di 1 703 936 B scritto nei workflow (`pages.yml`, `sito-rust.yml`, `release.yml`) e dichiarato «non alzabile» (`docs/studi/WASM_PACCHETTI.md`): ogni crescita si misura e, se serve alzarlo, si scrive al Coordinatore (le righe dei workflow le tocca lui).

## 3. I suggerimenti esterni, valutati
| # | Suggerimento | Verdetto | Pacchetto |
|---|---|---|---|
| 1 | Cache binaria SoA mappata, decodifica in parallelo | Per l'asse m/z c'è già (`.mzidx`, `memmap2` nel desktop). Manca l'asse «per scansione» (lo spettro si rilegge dall'mzML o dal `.raw`): è il blocco colonnare di T-7, che il visualizzatore consuma (`INTERFACCIA-T7.md`) | V-6 |
| 2 | Indice per m/z a secchi + matrice scansione × secchio | Non serve per l'XIC: l'ordinamento globale per m/z dà già 0,1 ms su 27 M picchi. Si dimostra su un indice da file ≥ 2 GB (G7); un indice nuovo entra solo se G7 fallisce, con la misura. La matrice CSR resta di T-7 (B3, feature detection) | V-4 |
| 3 | Binario, mai JSON | Sì: è il guadagno misurato più grande (×20 nativo, 500 ms → obiettivo ≤ 50 ms su 15 file HR). Rotta binaria nuova sullo schema di `scanbin`, stessi valori al bit | V-2 |
| 4 | Decimazione per pixel (LTTB o min-max) | Il disegno fa già M4 (min-max con primo e ultimo); LTTB no (sceglie punti «belli», non garantisce min e max per colonna). Manca: limitarsi alla finestra di zoom con ricerca binaria, bastoncini al massimo per pixel, prova formale di nessun apice perso | V-3 |
| 5 | XIC con rayon sulle sole MS1 | Utile solo nel nativo e solo fra file diversi (un XIC su un file è 0,1 ms). Nel sito non ci sono fili; si misura se un pool di worker serve | V-4 |
| 6 | Canvas 2D o WebGL | Con M4 un tracciato costa ≤ 4 punti per colonna di pixel: si misura il tempo del fotogramma; WebGL entra solo se G5 fallisce a 1 M punti | V-3 |

## 4. Gesti misurati e numeri da battere
Macchina di riferimento: Chromium headless del container cloud (4 CPU), sito costruito come `pages.yml`; ripetuta sul Mac di Federico quando la chat locale risponde (R-03 in `coordinamento/SCAMBIO_LOC.md` del privato). Ogni numero: mediana e 95° percentile di ≥ 30 ripetizioni, prima e dopo, con file, macchina e commit. Il tempo va dall'evento dell'utente (pointerup/keydown) alla fine del disegno del canvas interessato (`performance.now()` nella pagina), misurato senza toccare il prodotto (ganci sulle funzioni globali come `tools/bench_mappa.py`).

| Gesto | Serie | Oggi (fonte) | Obiettivo |
|---|---|---|---|
| G1 spettro al clic sul cromatogramma | LR, HR | motore 2,5 ms LR, 7,5 ms HR (misure-diario); fino al disegno: V-0 | p50 ≤ 16 ms LR, ≤ 33 ms HR; p95 ≤ 50 ms |
| G2 XIC dal pulsante, un file con indice | LR, HR | V-0 (via `spec.xic`) | p50 ≤ 16 ms, p95 ≤ 33 ms fino al disegno; file senza indice p95 ≤ 100 ms |
| G3 XIC su tutti i file della scheda | 20 LR, 15 HR | 45 ms LR, ~500 ms HR (misure-diario) | LR p50 ≤ 30 ms; HR p50 ≤ 50 ms, p95 ≤ 100 ms |
| G4 anteprima al passaggio sul picco (lente) | LR, HR, un file | non esiste | dal movimento alla lente p95 ≤ 33 ms; in una spazzata continua fotogrammi a ≥ 55 FPS (mediana), p95 ≤ 20 ms, nessun long task > 50 ms |
| G5 zoom/spostamento di cromatogramma e spettro | 3k, 50k, 1 M punti | V-0 | ridisegno p95 ≤ 8 ms per pannello fino a 50 k punti, ≤ 16 ms a 1 M; ricalcolo della decimazione ≤ 1 ms |
| G6 scansione successiva con le frecce | LR, HR | V-0 | nessun peggioramento > 10 % |
| G7 XIC su indice di un file ≥ 2 GB | nativo, sito | sonda: 0,11/0,31 ms su 451 MB di indice | nativo p95 ≤ 5 ms fino ai byte di risposta; sito p95 ≤ 50 ms (budget dichiarato: letture OPFS) |
| M memoria | 1000 anteprime | – | heap del worker +≤ 5 MB, nessuna crescita della pagina (buffer riciclati) |

Serie: sintetiche generate dal banco (V-0: LR 3200-like 1500 × 500; HR Orbitrap a centroidi 3000 × 3000 e 6000 × 5000; profilo 400 × 50 000; 20 LR e 15 HR per G3); file grande ≥ 2 GB di sorgente simulato con `mzml_tempi genera` o `mzidx_tempi`; file veri solo dalla chat locale, nel pubblico solo aggregati.

## 5. Prove (ognuna nella PR che cambia ciò che prova)
- **E1 Stessi numeri**: l'XIC binario è uguale bit per bit ai valori della rotta JSON di oggi (RT arrotondato a 4 decimali, y a 1, `json::round`), su tutti i sintetici e gli esempi; test Rust di parità + passo e2e che confronta `api/xic` e la rotta nuova sugli stessi file. Tolleranza dichiarata: 0.
- **E2 Somma giusta**: l'XIC è la somma dei centroidi in `[mz − tol, mz + tol]` (estremi come `Index::search`) nell'ordine della tabella (come `np.bincount`): resta la parità `routes_parity.rs`; il pulsante non cambia finestra né tolleranza rispetto a `xicDirect`.
- **E3 Nessun apice perso**: per ogni colonna di pixel il tracciato decimato ha lo stesso minimo, massimo, primo e ultimo dei dati; il punto più alto di ogni picco è disegnato; area e apice si calcolano sempre sui dati interi (mai sui decimati). Prova: test JS puro su tracce sintetiche (gaussiane strette, rumore, picchi di un punto) e confronto delle immagini del canvas prima/dopo (differenza di pixel 0, o dichiarata e spiegata).
- **E4 Valori mostrati**: suggerimenti, tabella dei dati (`TDATI`), integrazione ed esportazioni leggono i valori del motore, mai quelli decimati.
- **A Accessibilità**: axe 0 violazioni sui pannelli nuovi (`e2e_axe_grafici` esteso); tutto da tastiera (scegliere il picco, estrarre, cambiare tolleranza, chiudere); `aria-pressed`/`aria-label` sui pulsanti; `prefers-reduced-motion` (nessuna animazione della lente); bersagli ≥ 24 × 24 px (40 al tocco); ogni grafico nuovo con la sua tabella `TDATI`; colori dalle variabili CSS (Okabe-Ito).
- **R Nessuna regressione**: verdi `e2e_tutti`, `e2e_cursore`, `e2e_spettro*`, `e2e_hr_xic`, `e2e_lr` (dati d'oro), `e2e_tour` (ogni passo trova il bersaglio), `e2e_rust`, `MZLAB_LR_MISS` vuoto, `e2e_shim` del desktop dove si tocca il protocollo; tetti `.wasm` (§2).
- **U Prova d'uso** (§8) prima di scegliere in via definitiva lente o pannello come destinazione predefinita.

## 6. Decisioni (proposta dell'architetto; il Coordinatore conferma o porta a Federico)
- **D1 Destinazione dello XIC**: predefinita il **pannello sotto lo spettro** (esiste, si integra, si esporta, ha `TDATI`, cursore e legenda: nessun grafico nuovo da rendere accessibile). La **lente** dentro lo spettro è l'anteprima al passaggio (V-5): transitoria, un file, nessuno stato; Invio/clic sulla lente o il pulsante la rendono pannello. Si conferma con la prova d'uso (U) e con G2/G4.
- **D2 Area e apice**: l'apice (RT, intensità) si scrive sempre nella legenda dello XIC nato dal pulsante (è una lettura dei dati). L'area è quella dell'integrazione che già esiste (stesso metodo `PK.method`), sul picco che contiene la scansione corrente: in HR si calcola subito; in LR con un tasto/clic («Integra», già scorciatoia), perché l'integrazione degli studenti resta un loro gesto (principio 1). Se Federico la vuole automatica anche in LR è una riga di configurazione.
- **D3 Nessun indice a secchi**: si dimostra con G7; entra solo se G7 fallisce.
- **D4 Canvas 2D**: resta salvo G5 fallito a 1 M punti.
- **D5 Arrotondamento**: i valori trasportati restano quelli di oggi (RT 4 decimali, y 0,1 conteggi, come Python e i dati d'oro); togliere l'arrotondamento cambierebbe i dati d'oro e i confronti con Python: non proposto.
- **D6 Motore `.wasm`**: ogni pacchetto che lo tocca misura prima/dopo; obiettivo crescita ≤ 2 KB per pacchetto entro il tetto attuale; se non basta, PR aperta e domanda al Coordinatore (il tetto sta nei workflow). Regole di velocità comuni: due target, guadagni anche piccoli contano (riga in `docs/agenti/misure-velocita.md`), dipendenze di velocità solo se ≥ 3 %.
- **D7 Tolleranza**: mai inventata. HR: `HR.prof(f, livello).tol` (profilo dell'analizzatore del file, 5 FTMS / 10 TOF; attenzione: `hr.js` oggi la sostituisce con `UIP.hrPpm` delle Impostazioni, 5 di base, anche per un TOF: V-1 lo verifica e lo riferisce, senza cambiarlo da solo); LR: la finestra unitaria `xicWin` (3200) o `HR.prof` per la trappola (0,5 Da). Visibile e modificabile accanto al pulsante; la modifica vale per quello XIC, non cambia le Impostazioni.

## 7. Confronto con Xcalibur/FreeStyle (solo numeri di Federico)
Protocollo per la chat locale: stesso file `.raw` (uno LR o un Orbitrap del corpus) e stesso Mac; gesti G1, G2, G4 (FreeStyle: XIC dal picco dello spettro) misurati con una registrazione dello schermo a 60 fps (fotogrammi dal clic al disegno), 10 ripetizioni, mediana; versione dei programmi. Nel pubblico solo la tabella aggregata in `docs/agenti/visore-veloce.md` con «misurato da Federico, data, macchina». Senza quei numeri la tabella dice «non misurato».

## 8. Prova d'uso con 5 persone (piano per Federico)
- **Chi**: 3 studenti o ex studenti dell'esperienza 3 (LR), 2 dottorandi/ricercatori HR; nessuno ha visto la funzione prima.
- **Compiti** (sito pubblicato, file di esempio, 10 minuti a persona, pensiero ad alta voce): (1) LR: «trova in quali minuti compare lo ione più intenso dello spettro a 6 min» (2) LR, solo tastiera: lo stesso su un altro picco (3) HR: «XIC del secondo ione più intenso a 5 ppm, poi a 10 ppm» (4) HR: «guarda tre ioni vicini e scegli quello con il picco più stretto» (lente, se presente) (5) «quanto vale l'area?».
- **Misure**: tempo per compito, errori/chiarimenti chiesti, numero di clic, preferenza lente/pannello (domanda finale), una frase libera. Registro anonimo (nessun nome) in `mzlab-dati/studio/` del privato; nel pubblico solo gli aggregati.
- **Soglia**: la destinazione predefinita (D1) è quella con tempo mediano minore e nessun errore bloccante; il compito 2 (tastiera) deve riuscire a 5 persone su 5.

## 9. Interfaccia con T-7
Il blocco colonnare opzionale del `.mzidx` è di T-7 (`docs/studi/PROMPT_LETTORE_THERMO/T-7.md`, B2-B3); ciò che il visualizzatore ne chiede sta in `INTERFACCIA-T7.md`. Il visualizzatore non scrive formati e non centroida.
