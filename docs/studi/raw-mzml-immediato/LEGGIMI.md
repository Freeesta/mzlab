# Thermo `.raw` → mzML immediato: studio e prompt

Questo studio risponde al compito: convertire un `.raw` Thermo HRMS in mzML nel minor tempo possibile, senza perdere informazione e con centroidi corretti, solo in Rust (nativo e `.wasm`), su dati molto densi. È stato scritto prima di ogni codice. Nel prodotto non entrano né RFR né codice del produttore.

## Cosa leggere
- **`RISPOSTE.md`**: le quattro domande (significati, da dove si parte, come si lavora, prove esterne) e il controllo di `RAW_VELOCE.md` e `PROMPT_LETTORE_THERMO/`.
- **`MISURE.md`**: i numeri di partenza e come ripeterli.
- **`CRITERI.md`**: i criteri di accettazione con le soglie, scritti prima del codice:
  - F: fedeltà;
  - C: centroidatura;
  - V: velocità e memoria;
  - E: prove esterne.
- **`DUBBI.md`**: i dubbi D1-D11 e cinque decisioni per il Coordinatore, ognuna con una raccomandazione.
- **`RICHIESTE_LOCALE.md`**: le richieste R-a…R-e per la chat locale (RFR, msconvert, calibranti, validatori, file pubblici), da copiare in `SCAMBIO_LOC.md`.
- **`VALUTAZIONE_GEMINI.md`**: verdetti, con le prove, sulle idee e sul codice del documento Gemini del repository privato, e cosa entra nei prompt.
- **`prompt/`**: le regole comuni (`COMUNI.md`) e i prompt RM-0…RM-9 per le chat DEV.

## Tesi in quattro righe
1. **Centroidi.** Nelle scansioni FT i centroidi giusti sono quelli che lo strumento ha già scritto nel file. Si copiano al bit (costo ~0) insieme alle colonne per picco che oggi buttiamo. La nostra centroidatura serve solo dove la lista manca, misurata contro simulati, calibranti e lista dello strumento.
2. **Informazione.** Senza perdite vuol dire che dall'mzML si devono poter ricostruire profilo, lista e colonne, metadati completi e bin di frequenza. Gli zeri di RFR si scrivono solo su richiesta, e marcati.
3. **Tempo.** Il pavimento fisico su LAB-B è ≈ 0,15-0,2 s su 4 core. Oggi siamo 15-40 volte sopra. Il costo non sta nella compressione: sta nell'azzeramento del compressore per ogni array, nel produttore seriale, nell'XML con allocazioni e nell'SHA-1 sul percorso critico.
4. **Thread.** Oggi più thread rallentano.

## Ordine e dipendenze
| Prompt | Cosa | Dopo | In parallelo con | File principali |
|---|---|---|---|---|
| RM-0 | banco, roofline, caso denso sintetico, D8 e D11 | – | – | `examples/`, `tools/bench/`, `docs/agenti/raw-mzml.md` (nuovo) |
| RM-1 | metadati fedeli (attivazione, DIA, trailer) | RM-0 | RM-3, RM-6 | `rawlettore.rs`, `extra`, `vendor/opentfraw/src/mzml.rs` |
| RM-3 | scrittore veloce | RM-0 | RM-1, RM-6 | `mzmlwrite*`, `lettori_mzml.rs` |
| RM-2 | colonne per picco, zeri, `--peaks tutto` | RM-1, dump R-b; decisioni 1, 2 e 4 | RM-6 | `scan_data.rs` (o T-7b), `rawlettore.rs` |
| RM-4 | produttore parallelo nativo | RM-1, RM-3 | RM-6 | modulo nuovo della pipeline, `mmap` isolato |
| RM-5 | browser con più worker | RM-4 | – | Rust `.wasm`, worker, OPFS |
| RM-6 | banco della centroidatura | RM-0 | tutti | banco nuovo in `mzlab-core` |
| RM-7 | prove esterne con la locale | RM-2, RM-3, RM-4 (si ripete) | – | `tools/` |
| RM-8 | fuzzing e casi limite | RM-4 | RM-5, RM-7 | test e bersagli di fuzzing |
| RM-9 | Numpress veloce (opzione con perdita) | RM-3, RM-0 (denso) | RM-5…RM-8 | `numpress.rs`, `mzmlwrite/codifica.rs` |

- Una PR alla volta per gruppo di file: RM-1 e RM-2 su `rawlettore.rs`; RM-3 e RM-4 su `mzmlwrite`.
- T-7 resta il prompt del lettore. RM-2 e T-7b si toccano su `scan_data.rs` (decisione 4 di `DUBBI.md`). RM-6 è il banco di T-7e (B4).

## Come si usa un prompt
Si apre una chat nuova per prompt. Le si incolla il testo dopo «---» del file `prompt/RM-n.md`: il prompt le dice di leggere per primo `prompt/COMUNI.md`.
