# C32 · Reattività precalcolata per i composti di `composti.csv` (studio)

Solo studio: nessuna funzionalità di prodotto, `.wasm` `chem` invariato. Misure fatte con `crates/mzlab-chem/examples/precalc_reatt.rs` (release, container a 4 CPU, motore `mzlab-chem/1`, opzioni predefinite dell'app: acqua ALPB, 4 conformeri, seme 42, forma «As drawn»). Contesto: `docs/agenti/reattivita.md`.

## 1. Misure
| Misura | Valore |
|---|---|
| Composti in `TP_Mine/data/composti.csv` | 625 (108 767 B) |
| Giro completo, 4 thread (due giri) | 743 s e 774 s di tempo reale; somma dei tempi per composto 1504 s e 1540 s |
| Per composto (4 thread, quindi con contesa) | mediana 1,04 s e 1,01 s; p90 3,5 s; massimo 182-184 s (un solo composto) |
| Campione di 40 composti (1 ogni 16, un filo solo, nessuna contesa) | somma 110,8 s, mediana 1,28 s, p90 8,8 s, massimo 33,7 s, media 2,77 s |
| Scala del campione su 625 | 625 × 2,77 s ≈ 29 min a un filo (incertezza alta: la coda lunga è di pochi composti; il giro vero a 4 thread ha somma 25-26 min) |
| Esito | 620 riusciti, 5 falliti (stessi 5 in entrambi i giri) |
| Tabella compatta (SHA-256 identico nei due giri) | 776 817 B non compressa, 167 133 B gzip -9, 132 352 B xz -9 |
| `reatt/1` completo di tutti i 620 | 5 685 674 B non compressi (gzip non misurato) |
| Riferimento browser già misurato (`e2e_chem_tempo.py`) | carbamazepina 1,03 s in Chromium contro 0,54 s nativo (×1,9) |

- **Falliti** (5): levofloxacina, flumequina, pirene e acenaftene danno `error.chem.embed` (l'ETKDG non trova un conformero valido dopo 220-460 tentativi; il fallimento costa molto: levofloxacina 182 s, flumequina 49 s, pirene 22 s, acenaftene 9 s) e flusilazolo dà `error.chem.smiles` per l'elemento Si, non supportato. Sono 0,8 % dei composti ma 263 s su 1504 s (17,5 %) del tempo totale.
- **Tempo di un composto**: nativo 1 s mediano e 3-9 s per i grandi (p90 3,5-8,8 s a seconda della contesa); in Chromium la stima è ×1,9 (non misurata sui 625; solo sui due composti di `e2e_chem_tempo.py`), cioè mediana ≈ 2 s e coda di molte decine di secondi.
- **Dimensione della tabella**: il formato compatto della misura contiene per atomo (H compresi) `[q, f+, f−, HOMO, LUMO]` a 3 decimali e per composto HOMO e LUMO globali in eV, chiave InChIKey. Mancano `f_zero` e duale (si ricavano: f0 = (f+ + f−)/2, duale = f+ − f−), `bonds`, coordinate, `hbonds`, `ch_sites`, `conformers`, `notes`: tutto ricostruibile al volo dal grafo e dalla geometria, salvo `ch_sites`.
- **Soglia proposta**: ≤ 250 KB gzip per la tabella (il `.wasm` `chem` ottimizzato pesa 551 415 B: la tabella da 167 KB gz ci sta). Con tutti i campi di `reatt/1` si supererebbe la soglia (5,7 MB non compressi): non è un'opzione.
- **Determinismo**: due giri completi sulla stessa macchina danno tabelle identiche byte per byte (stesso SHA-256), anche con thread diversi (ogni composto è calcolato da un solo filo; il seme è fisso). Non verificato: native contro `.wasm` e fra CPU diverse (le istruzioni in virgola mobile e `libm` possono spostare le ultime cifre; a 3 decimali la differenza attesa è nulla salvo casi al limite dell'arrotondamento). Per questo la tabella va generata dal motore nativo o dal `.wasm` ma non confrontata con l'altro bit per bit.

## 2. Chiave di cache e invalidazione
- **Chiave (come `reatt-auto.js` per la cache di sessione)**: SMILES canonico di OpenChemLib + `version()` del motore + solvente + numero di conformeri + protomero. Nella tabella precalcolata solvente, conformeri e protomero sono costanti (`water`, 4, `none`) e vanno scritti nell'intestazione, non per riga.
- **Chiave di riga**: InChIKey del CSV (già presente, stabile) più lo SMILES con cui è stato calcolato. Il pannello oggi cerca per SMILES canonico: servirebbe canonicalizzare gli SMILES della tabella con OpenChemLib in fase di generazione (script in `tools/`, Node o Playwright) oppure far confrontare al pannello lo SMILES canonico della riga con quello del genitore.
- **Invalidazione**: l'intestazione porta `engine` (`mzlab-chem/1`), l'hash SHA-256 del `.wasm` `chem` (già nel manifesto) e quello di `composti.csv`; se l'hash del `.wasm` non coincide la tabella è ignorata e si calcola al volo come oggi. Il solo `version()` non basta: oggi non cambia quando cambiano i numeri del motore (parametri, correzioni), quindi serve l'hash del binario o un contatore di versione dei risultati incrementato a ogni modifica numerica.
- **Riga assente o fallita**: si calcola al volo, come adesso (barra «Recompute», solvente, conformeri e forma diversi dai predefiniti non usano la tabella).

## 3. Dati già calcolati riutilizzabili
- **Nel repository**: nessuno; solo `crates/mzlab-chem/tests/data/xtb/` (20 molecole, xtb 6.7.1) e `cc_ref.json` (20 composti, tblite).
- **Pubblici**: due ricerche (indici di Fukui condensati calcolati; Zenodo/Figshare, QM9) non hanno trovato nessun insieme di indici di Fukui condensati scaricabile con licenza libera.
- QM9 (134 mila molecole con al massimo 9 atomi pesanti C, N, O, F; cariche di Mulliken in DFT B3LYP in fase gassosa; dichiarato libero da una sola fonte secondaria, da verificare nell'articolo originale prima di qualunque uso) non contiene Fukui, copre pochi composti di `composti.csv` e usa un metodo e un solvente diversi da GFN2/ALPB acqua: non è confrontabile né sostituibile. Un articolo di ML (2016, circa 23 000 atomi con Fukui DFT) non dichiara dati rilasciati. Conclusione: i dati vanno prodotti col motore di mzLab;
  il confronto con valori esterni resta quello già fatto con tblite e xtb sui 20 composti.

## 4. Dove si inserirebbe
- **Posizione**: `TP_Mine/lazy/reatt-auto.js`, prima del calcolo: se l'amministratore è attivo, le opzioni sono quelle predefinite e c'è una riga con la chiave giusta, `TPREATT.mount` riceve un `reatt/1` ricostruito dalla riga (stato «done (precomputed)»); altrimenti il flusso resta quello attuale. Il pannello è già solo amministratore e lazy, quindi la tabella (file `lazy/` o `data/`) si scarica solo con amministratore.
- **Campi da mostrare**: gli stessi di oggi (f(−), f(0), f(+), duale, q, HOMO share, LUMO share, HOMO e LUMO globali) per atomo; restano da calcolare al volo (o omessi) `ch_sites`, `conformers`, `f_minus_spread`, legami H, geometria 3D per il visore (servirebbe l'ETKDG, ~50-150 ms nativi, senza SCC).
- **Il Dig non cambia**: la tabella serve il genitore, che è uno per sessione; oggi il calcolo parte a TP finiti e non rallenta il Dig (+76 ms misurati).

## 5. Costo di manutenzione
- **Rigenerazione**: ogni modifica numerica del motore (parametri, SCC, ALPB, Fukui, geometria, versione di sci-form) rende la tabella obsoleta: 743-774 s con 4 CPU (≈ 13 min) più i tempi di CI; un workflow separato (non `test.yml`) o `tools/rigenera_reatt.py` che lancia l'esempio e scrive la tabella e l'intestazione con gli hash. Il file va riscritto nel repository (167 KB gz, 777 KB non compressi: diff grandi a ogni cambio) o prodotto dalla CI nel sito senza passare dal repository.
- **Prova di coerenza**: un test a campione (es. 5 composti) che ricalcola e confronta con la tabella; se l'hash del `.wasm` non coincide, il test impone di rigenerare.
- **Rischi**: la tabella invecchia in silenzio se l'hash non è controllato; i 5 falliti vanno gestiti (nessun valore e calcolo al volo che fallirebbe lo stesso: oggi il pannello mostra l'errore); `composti.csv` cambia a ogni rigenerazione da PubChemLite (`tools/genera_composti.py`) e porta nuovi composti da calcolare.

## 6. Raccomandazione: NO per ora

- Motivi: (1) il guadagno è solo l'attesa di ~1-6 s (stima nel browser) per un genitore per sessione, già in background, solo in modalità amministratore e senza costo per il Dig; (2) la tabella è piccola (167 KB gz) ma la rigenerazione costa 13 min di 4 CPU a ogni cambio numerico del motore e un diff di 777 KB;
  (3) i valori sono «Prev.» e non sono dati che gli studenti usano; (4) non esistono dati esterni riutilizzabili; (5) nessun gruppo di composti resta senza calcolo al volo, quindi il codice di ripiego è comunque necessario e la tabella è solo una scorciatoia; (6) il determinismo native/`.wasm` non è verificato. Il parere cambia se la reattività esce dalla modalità amministratore e se ne servono molti genitori alla volta.

## 7. Piano a pacchetti (solo se si decide sì)
- **P1 · generatore e formato**: `tools/rigenera_reatt.py` che chiama l'esempio e scrive `TP_Mine/data/reatt-tabella.json` (intestazione con motore, hash `.wasm`, hash CSV, opzioni; righe compatte come sopra); canonicalizzazione OpenChemLib; test di coerenza a campione.
- **P2 · innesto**: lettura in `reatt-auto.js`, ricostruzione di `reatt/1`, stato «done (precomputed)», test `e2e_reattivita_auto.py` (con e senza hash coerente, composto assente e fallito), voce in `docs/agenti/reattivita.md`, caricamento lazy e `indice.json`.
- **P3 · verifica nativo/`.wasm`**: giro completo con il `.wasm` ottimizzato in Chromium sullo stesso insieme, confronto con la tabella nativa a 3 decimali (la parte di determinismo non verificata qui), ed eventuale regola di tolleranza.
