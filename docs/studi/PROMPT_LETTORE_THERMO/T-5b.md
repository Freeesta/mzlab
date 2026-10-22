# T-5b — Thermo `.raw`: LTQ Orbitrap XL v63 con gli eventi non letti

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

Pacchetto T-5b del lettore Thermo, seguito di T-5 (`T-5.md`, in main). Esito della riapertura del corpus con main dopo T-5 (R-06, chat locale, `esperimenti/` nel privato): 228 file su 238 aperti come RFR, scansioni uguali su 227; i 10 non aperti non li apre neanche RFR (5 AppleDouble, 3 corrotti, 2 senza MS) e restano errori chiari. Resta UN file: R215, LTQ Orbitrap XL, FileHeader v63: 4 scansioni contro le 1141 di RFR; la `ScanEventTable` non si legge (richiesta di allocazione da 592 MB, evento da 5 byte).
File da leggere oltre ai comuni: `vendor/opentfraw/src/reader.rs` (`walk_events`, `walk_counted`), `scan_event.rs`, `crates/mzlab-lettori/tests/robustezza_thermo.rs`, il rapporto R-06 nel privato.

## Da fare
1. Riprodurre su R215 (dalla chat locale, file nel privato) e trovare perché il conteggio degli eventi è letto male in v63 (posizione, larghezza del contatore o variante del preambolo per l'LTQ Orbitrap XL); nessun limite alzato per «far passare» 592 MB.
2. Correggere la lettura per quella versione e strumento; i file v63 che già si aprono restano uguali byte per byte (`raw_equivalenza.rs`).
3. Test con un file costruito che riproduce la struttura (nessun byte del file reale nel pubblico); riga in `MODIFICHE-MZLAB.md`.

## Atteso
R215: 1141 scansioni come RFR, campi di `scandump/1` confrontati con il dump RFR se la chat locale lo produce; altrimenti «non verificato contro RFR». Criterio di chiusura: ogni file che RFR apre si apre anche da noi con lo stesso numero di scansioni, oppure ha una causa scritta in `LETTORE_THERMO_PERFETTO.md` §2.

## Regole comuni (valgono per tutta la chat)
- Sei una chat DEV del lettore Thermo di mzLab (repository `Freeesta/mzlab`). Coordinatore = mz COO 11 (`session_01SqVGjScVGPMmoMETWKcX2F`): a fine lavoro UN messaggio con `send_message` (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Bloccato o davanti a una scelta: PR aperta con la spiegazione e domanda al Coordinatore con una raccomandazione; mai unire dalla chat.
- AGENTS.md è nel tuo contesto. Leggi SOLO: `docs/studi/LETTORE_THERMO_PERFETTO.md`, `docs/agenti/scandump.md`, `docs/agenti/lettori-modulo.md` (righe Thermo), `vendor/opentfraw/MODIFICHE-MZLAB.md` e i file indicati sopra. Codice: `python3 tools/indice.py`, `grep -n`, subagente `cercatore`.
- Ramo `claude/thermo-<argomento>` da `origin/main`; `tools/fondi_main.py` prima di spingere; `tools/pre_pr.py` completo.
- **Oracolo RFR** (RawFileReader di Thermo): solo sul Mac, nella chat locale, fuori da ogni repository; produce `scandump/1` in `mzlab-dati/scratchpad/rfr/`. Nel repository nessuna DLL, pacchetto, codice che lo usa o testo copiato; mai decompilare.
  Si confrontano solo le uscite: `thermo_scandump` (nostro) contro il dump RFR con `tools/rosina/scandump_compare.py`; un campo verificato su tutto il corpus entra in `VERIFICATI` di `crates/mzlab-lettori/tests/scandump_thermo.rs` (da lì in poi il test fallisce se diverge). Se i dump RFR non ci sono ancora: lavora su file costruiti nei test, scrivi «non verificato contro RFR» e chiedi al Coordinatore i dump che servono (nome dell'accession, non del file).
- **Dati**: solo nel privato `Freeesta/mzlab-dati` con il risolutore unico (`tools/dati_percorsi.py`/`.rs`); i test con file veri saltano senza `MZLAB_DATI`. Nel pubblico solo file costruiti e aggregati; nessun nome di file privato né cognome di `nomi_vietati.txt` (usa l'accession). File > 90 MB solo dalla chat locale.
- **Compatibilità**: per i file su cui il pacchetto non cambia nulla `.mzidx`, mzML, `metodo/1` e l'ingresso di msngrafo restano byte per byte uguali (`crates/mzlab-core/tests/raw_equivalenza.rs`, `raw_parity.rs`, `msntree_parity.rs`); dove cambiano, prima/dopo su un file nella PR.
- **Tetti**: modulo `thermo` 546 304 B (`tools/build_lettori_wasm.sh`, `tests/test_lettori_wasm.py`, `docs/agenti/wasm-tetti.md`); regola del Coordinatore: un aumento fino a +10 % è ammesso se dichiarato nella PR con la misura prima/dopo e aggiornato nei tre file. Il `.wasm` del motore non cresce (`cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm`, misura prima e dopo). File d'area ≤ 25 600 B, righe ≤ 600 caratteri.
- **Contratto `Lettore`**: ogni numero letto dal file passa da `Limiti` prima di allocare; errori come chiavi `err.file.*`, mai panico; ciò che il file non ha è assente, mai inventato. Ogni modifica a `vendor/opentfraw` = una riga in `MODIFICHE-MZLAB.md` (licenza Apache-2.0).
- Documenti = stato attuale: riscrivi la riga che descrive il comportamento (`lettori-modulo.md`, `scandump.md`, `LETTORE_THERMO_PERFETTO.md` §2 punto chiuso).
- Subagenti Haiku solo per lavoro meccanico; mai per decisioni, codice nuovo, asserzioni, commit, push. Contesto oltre 250k token a fine PR: blocco di consegna al Coordinatore e fermati.
