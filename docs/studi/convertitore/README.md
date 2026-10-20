# Studio: il convertitore mzML di mzLab

Obiettivo: ogni file che mzLab legge esce come mzML indicizzato, valido e senza perdite dichiarate, più in fretta e con meno memoria degli strumenti di riferimento, su Mac, Linux e Windows senza Wine né Docker. Solo computer (decisione di Federico: niente tablet né telefono). I prompt DEV pronti sono i file `C1.md` … `C8.md` di questa cartella (§8).

## 1. Cosa esiste oggi (dal codice)

- **Scrittore**: `crates/mzlab-core/src/mzmlwrite.rs` scrive spettro per spettro su qualunque `Write` (zlib livello 1, numpress facoltativo con perdita). `MzmlWriter::indicizzato` (`mzmlwrite/indice.rs`) scrive `indexedmzML` con offset e SHA-1 in streaming; nel `.wasm` non c'è (`mzmlwrite/indice_wasm.rs` → `err.route.unavailable`), perché lo SHA-1 e il testo dell'indice spingerebbero i moduli lettori oltre i tetti.
- **Percorso generico**: `lettori_mzml::scrivi_con` / `scrivi_indicizzato` (`crates/mzlab-core/src/lettori_mzml.rs`) porta un qualunque `Lettore` (`lettori.rs:191` `scansione`, `:208` `testata_mzml`, `:212` `extra_mzml`) nello scrittore. Lo usano «Scarica .mzML» del sito e del desktop (`native_to_mzml`, `nat2mzml`) e il passo `convert.mzml@1` (`step/core_steps.rs:30`, solo nativo, eseguito da `mzlab-cli flow run`, `flow_base.rs:199`).
- **Percorsi paralleli**: `raw::to_mzml_progress` (`raw.rs:147`, scrittore di opentfraw + `MzmlFilter`, con trailer `opentfraw.*`, profilo facoltativo; lo usa solo `examples/raw2mzml.rs` e `tests/raw_profile_parity.rs`), `wiff::Wiff::to_mzml_progress` (`wiff/run.rs:169`), `agilent::…::to_mzml_progress` (`agilent/run.rs:216`, `agilent_ext/run.rs:232`). Quattro strade per lo stesso lavoro: i metadati dipendono da quale si prende.
- **Centroidi**: `centroidi/` (stima dei parametri su ≤128 scansioni, picchi, spalle; `convert.rs`: mzML a profilo → copia a centroidi, letta due volte, mai tenuta intera) e `lettori_centroidi.rs` (lo stesso sopra un `Lettore`). mzFinder costruisce da sé un `.cen.mzidx` per un campione a profilo.
- **Validazione**: `mzmlwrite/tests_xsd.rs` controlla l'indicizzato con `xmllint` contro `mzML1.1.2_idx.xsd`, ma passa in silenzio senza rete o senza `xmllint`. Nessun controllo semantico del CV, nessun confronto automatico con MSConvert/ThermoRawFileParser (TRFP).
- **Strumenti Python**: `tools/converti.py` chiama TRFP per i `.raw` e stampa i comandi di MSConvert in Docker per gli altri formati: è l'opposto dell'obiettivo e resta solo come oracolo.

### Lacune trovate leggendo il codice
1. **Thermo perde metadati nel percorso generico**: `LettoreRaw` (`rawlettore.rs:297`) non implementa `extra_mzml`. L'mzML del sito, del desktop e di `convert.mzml` per un `.raw` non ha carica, energia di collisione, tipo di attivazione, finestra d'isolamento, tempo d'iniezione, FAIMS né i campi del trailer; li ha solo `raw::to_mzml` (che nessun utente usa). I valori esistono già: `header_fields` (`rawlettore.rs:418`) li calcola per la finestra «Intestazione».
2. **Un solo precursore**: `Spectrum::precursor` è un `Option<f64>` (`mzml.rs:27`) e lo scrittore mette lo stesso valore come target d'isolamento e ione selezionato (`mzmlwrite.rs:410`): niente catena MSn (`spectrumRef`), niente carica, niente offset della finestra, niente `activation`.
3. **TIC e picco base ricalcolati** dai punti scritti (`mzmlwrite.rs:381`) e non quelli del produttore: su un Orbitrap a centroidi differiscono dal TIC del file.
4. **Testata incoerente**: `version="1.1.0"` con CV 4.1.163 nel non indicizzato e 4.2.6 nell'indicizzato (`mzmlwrite.rs:49`); il software è `MS:1000799 custom unreleased software tool`.
5. **Profilo Thermo non raggiungibile** dal `Lettore`: `scansione` chiama `extract_spectrum(…, false)` (`rawlettore.rs:314`), cioè i centroidi del file; il profilo esiste solo in `raw::to_mzml_progress`.
6. **Cromatogrammi scartati**: `chromatogram_kind` (`mzmlwrite.rs:417`) scrive solo TIC, BPC, SIC, SRM e PDA; pressione e altre tracce analogiche dei lettori si perdono nel percorso generico.
7. **Errori nascosti**: un'eccezione JS nelle rotte LR diventa `err.route.unavailable` (`mzlab/web/lr.js:207`), la stessa chiave di «rotta che il motore non serve»: la diagnosi perde la causa.

## 2. Confronto onesto

Fonti: misure del 18-20/10 sul Mac (Apple Silicon) e su Windows 11 ARM in Parallels (x64 emulato), tabelle complete nel repository privato (`coordinamento/dati_esterni/rosina/LIMITI.md`, `ORACOLI.md`); le righe «da documentazione» non sono misurate.

| | MSConvert (ProteoWizard) | ThermoRawFileParser | OpenMS FileConverter | mzmine / MS-DIAL (import) | mzLab oggi |
|---|---|---|---|---|---|
| Formati dei produttori | tutti i maggiori; i lettori sono DLL dei produttori → solo Windows (Wine/Docker fragile: sul Mac abortisce) | solo Thermo (.NET, RawFileReader di Thermo) | formati aperti; `.raw` tramite TRFP (da documentazione) | importano; non sono convertitori (da documentazione) | Thermo, SCIEX `.wiff`, Agilent `.D`, Bruker TDF/TSF, Waters, Shimadzu, mzXML, ANDI, mzML numpress; Rust puro su ogni sistema |
| Centroidi | vendor o CWT, a scelta | del produttore (predefinito) | `PeakPickerHiRes` a parte | propri | quelli del file; algoritmo nostro solo su mzML a profilo |
| Indice + SHA-1 | sì | sì | sì | – | sì solo nativo (`convert.mzml`) |
| Tempo / RAM, DDA Q Exactive 1,6 GB | – | 58,7 s / 1656 MB | – | – | 23,4 s / 112 MB (`raw2mzml`) |
| Tempo / RAM, 22 file di 6 produttori (Windows) | 1-123 s / 43-229 MB; TDF timsTOF: 25,9 GB di mzML, interrotto | – | – | – | da misurare (C7) |
| Valori (m/z, intensità, RT, precursori) | oracolo | oracolo | – | – | identici a TRFP/MSConvert su Q Exactive, 3200 QTRAP (20 campioni), Waters MRM, Agilent QqQ; Agilent Q-TOF: 4/3852 precursori a 0,013 Da |
| Dove perde | – | – | – | – | TSQ MRM (0 punti), profilo FTMS, 11/41 `.raw` non aperti, TripleTOF e Agilent IMS rifiutati, lockmass e precursori Waters MSe, pressioni Agilent, metadati Thermo nel percorso generico |

Lettura: mzLab vince già su portabilità, velocità e memoria (2,5-4× più veloce e 15-30× meno RAM di TRFP su 4 file Thermo). Perde su ampiezza dei formati (TripleTOF, IMS) e su fedeltà dei metadati per spettro. «Migliore» si può dichiarare solo dove la tabella del §4 lo misura; dove MSConvert legge e noi no, lo studio lo dice.

## 3. Decisioni (architettura scelta)

1. **Un solo scrittore, un solo percorso**: tutto passa da `Lettore` → `lettori_mzml` → `MzmlWriter`. `raw::to_mzml_progress`, `wiff::…::to_mzml_progress` e `agilent*::…::to_mzml_progress` si tolgono dopo la parità (C2). Niente crate nuovo: lo scrittore è già in `mzlab-core` e un crate in più costa compilazione e tetti.
2. **Modello dello spettro esteso** (C1): `SpectrumExtra` porta una lista di precursori (`mz` selezionato, carica, intensità, finestra target/offset inferiore/superiore, attivazione come accession + energia, `spectrumRef` del genitore), TIC e picco base del produttore, tempo d'iniezione, finestre di scansione. `Spectrum` non cambia (lo usano tutte le rotte): il di più vive solo nella conversione.
3. **Zero perdite dichiarate**: ciò che ha un termine PSI-MS diventa cvParam con unità; il resto userParam `<formato>.<campo>` (come `opentfraw.trailer.*` oggi). Ogni conversione scrive nel `<run>` un userParam `mzLab conv/1` (JSON breve: lettore e versione, campi mappati a CV, a userParam, non portati con il motivo) e la stessa riga va nel resoconto della CLI. «Non portato» senza motivo = test rosso.
4. **Picchi, scelta esplicita** (C3): parametro `peaks` di `convert.mzml` = `file` (predefinito: ciò che il file contiene; per Thermo i centroidi del produttore se ci sono, come TRFP), `profile` (profilo del file dove esiste), `mzlab` (centroidi nostri, `processingMethod` MS:1000035 con i parametri stimati). Nessuna libreria dei produttori: i centroidi «del produttore» sono solo quelli scritti nel file. Un file a profilo che entra nel Flow dà l'avviso `warn.flow.profile` e usa i centroidi `mzlab` (come il `.cen.mzidx` di mzFinder).
5. **Velocità** (C4): lettura sequenziale nel lettore, codifica (zlib, base64, numpress) in un gruppo di thread con uscita in ordine; byte uguali con 1 o N thread; memoria = O(thread × spettro). Indice e SHA-1 restano in streaming. Nel motore del sito l'indice si accende solo se la misura del `.wasm` lo permette (lo SHA-1 c'è già per le impronte): altrimenti «Scarica .mzML» resta non indicizzato e lo dice nel `conv/1`.
6. **Dove vive** (C5-C7): `mzlab-cli convert` (sottile sopra l'esecutore di `convert.mzml`, una riga per file, codici d'uscita della CLI), insiemi di file (`.wiff` + `.wiff.scan`, cartelle `.D`/`.d`/`.raw`) come `FileSet`; app desktop: i file trascinati o caricati in Dati entrano in una coda di conversione in secondo piano (1 core libero alla UI, cartella cache dell'app, riuso per SHA-256 + versione dello scrittore + parametri), il Flow usa gli mzML pronti e mette in coda gli altri; sito: solo su richiesta e quando il Flow lo chiede (la quota OPFS non regge una copia di ogni file).
   Avanzamento e annullamento con le righe `CONVROWS` già esistenti; errore = forma unica di `diagnosi.js`, mai `err.route.unavailable` per un file.
7. **Validazione** (C6, C8): validatore in Rust `mzlab-cli validate` (struttura, offset e SHA-1 dell'indice, CV semantico con una tabella generata da `psi-ms.obo`, unità, termini obbligatori per elemento) + `xmllint` contro l'XSD nei test quando c'è; confronto numerico con gli oracoli in Rust (`examples/mzml_confronta.rs`), eseguito sul Mac sui dati veri.
8. **Licenze**: opentfraw è Apache-2.0 (già in `vendor/`, `LICENZE-TERZI.md`). RawFileReader di Thermo (.NET) e gli SDK di SCIEX, Agilent, Bruker (`timsdata`), Waters non si propongono: richiedono un runtime o DLL per sistema, licenze con vincoli di ridistribuzione e non girano in WASM. Nessuna nuova dipendenza di build o CI; il vocabolario `psi-ms.obo` (CC BY 4.0) entra come tabella generata con attribuzione, che va dichiarata in `LICENZE-TERZI.md` (C6).

## 4. Metriche: oggi, obiettivo, come si misura

| Metrica | Oggi | Obiettivo | Come si misura |
|---|---|---|---|
| Percorsi di scrittura mzML | 4 | 1 | `grep -rn "fn to_mzml" crates/` → solo `lettori_mzml` |
| Campi Thermo per spettro nel percorso generico | 0 oltre livello, RT, polarità, filtro, 1 precursore | tutti quelli di `header_fields` + trailer (62) | `tests/conv_fedelta.rs` con lettore finto; sul Mac conteggio cv/userParam per scansione contro TRFP |
| Campi non portati senza motivo | non contati | 0 | `conv/1` di ogni file di `dati/formati/` (Mac) |
| Catena MSn, carica, attivazione, finestra | assenti | presenti dove il file li ha | parità con TRFP/MSConvert: stessi valori sul 100 % delle MS2 |
| Valori m/z, intensità, RT contro oracolo | identici su 5 famiglie (Mac) | identici su ogni file letto da entrambi | `mzml_confronta`: m/z 0 ppm se stessa fonte (≤0,05 ppm se l'oracolo scrive f32), RT ≤1e-6 min, intensità relativa ≤1e-6 (f32); numpress: errore dichiarato ≤2e-4 |
| Validità | XSD solo se c'è rete | 100 % dei file convertiti: XSD + CV + indice + SHA-1 | `mzlab-cli validate` su ogni uscita dei test e dei file veri |
| DDA Q Exactive 1,6 GB (Mac) | 23,4 s, 112 MB | ≤ 12 s con 4 thread, ≤ 256 MB | `examples/convert_tempi.rs` (tempo e picco RSS), 3 prove |
| BAF Orbitrap 595 MB (nativo) | 13,99 s | ≤ 6 s | idem; `docs/studi/RAW_VELOCE.md` §2.1 |
| TDF timsTOF ~100 MB | MSConvert: 25,9 GB, non finisce | mzML centroidi in < 10 min, RAM ≤ 2 GB | `convert_tempi` sul Mac |
| `.raw` di `dati/formati/` convertiti | 30/41 | ≥ 38/41 | Mac, elenco del privato |
| Byte uguali con 1 e N thread | – | sì | test in C4 |
| Errori con causa | `err.route.unavailable` generico in LR | 0 file con `err.route.unavailable`; ogni errore con fase e causa | `tests_e2e/e2e_diagnosi.py` esteso (C7) |
| Clic per convertire una cartella (desktop) | non c'è | trascina + 1 | `MISURA` in C7 |

## 5. Rischi

- **Formati chiusi**: Thermo profilo, TSQ MRM (0 punti oggi), `.raw` non aperti (11/41) dipendono da opentfraw (`vendor/opentfraw`); la correzione del TSQ 9000 GC (`refusing to allocate`, `reader.rs`) è già assegnata alla chat locale del Mac. Il convertitore non può essere migliore del lettore: queste voci restano fuori da C1-C8 e si dichiarano nella tabella.
- **Tetti dei `.wasm`**: ogni riga nuova di `mzmlwrite` entra in tutti i moduli lettori (`tools/build_lettori_wasm.sh`); il modello esteso si compila nei moduli solo dove serve (feature come `sciex` oggi) e ogni PR misura prima/dopo. Mai alzare un tetto in silenzio.
- **Oracoli**: MSConvert va solo su Windows (Parallels sul Mac); TRFP sul Mac. Il confronto è debito della chat locale (C8), non della CI.
- **Parità che cambia byte**: togliere `raw::to_mzml` cambia l'mzML di `raw2mzml` e il test `raw_profile_parity.rs`; la parità diventa sui valori, non sullo SHA-256.
- **Quota del browser**: la conversione automatica nel sito raddoppierebbe OPFS; per questo nel sito resta a richiesta.
- **«Migliore al mondo»**: non si dichiara finché la tabella §4 non è verde sul Mac e i formati che solo MSConvert legge (TripleTOF, Agilent IMS) sono elencati come tali.

## 6. I 4 file che non si caricano

Segnalazione del 20/10 (serie di 12 file di Federico): 4 file danno `err.route.unavailable` e il doppio clic sul TIC non apre lo spettro. Dalla coda del Coordinatore: i 4 file sono mzML (non `.raw`) e la chat del Mac li riproduce nel browser; causa non ancora stabilita.
Collegamento con il convertitore: `err.route.unavailable` nasce anche dal ramo `catch` di `mzlab/web/lr.js:207`, che trasforma ogni eccezione JS (memoria, `RangeError`, worker caduto) in quella chiave; la stessa chiave è l'errore di `MzmlWriter::indicizzato` nel `.wasm`. C7 separa le due cose (errore del file con fase e testo originale) e aggiunge un e2e; la causa vera dei 4 file la dà la riproduzione sul Mac (C8 punto 1), con `tools/mzml_diagnosi.py` e `mzlab-cli validate` sui file.

## 7. Chat e ordine

Tre chat DEV + la chat locale del Mac (regola: lavori sugli stessi file = stessa chat, una PR alla volta).

| Chat | PR in ordine | Zona | Dopo |
|---|---|---|---|
| DEV Convertitore-core | C1 → C2 → C3 → C4 | `mzmlwrite*`, `lettori_mzml.rs`, `rawlettore.rs`, `centroidi/`, `step/core_steps.rs` | – |
| DEV CLI (accorpare con la chat CLI se aperta) | C5 → C6 | `crates/mzlab-cli` | C5 dopo C2; C6 dopo C1 |
| DEV App | C7 | `conversione.js`, `lr.js`, `crates/mzlab-desktop`, Flow web | C5 |
| Locale Mac | C8 | dati veri, misure | C4, C6 |

## 8. Prompt DEV

`C1.md` scrittore e fedeltà · `C2.md` un solo percorso · `C3.md` picchi · `C4.md` velocità · `C5.md` `mzlab-cli convert` · `C6.md` validatore · `C7.md` app e Flow · `C8.md` misure sul Mac.
