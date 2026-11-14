# Canali dei dati: un file = un insieme di canali
Il «tipo di esperimento» è un'etichetta e un'euristica per i default; gli strumenti lavorano sul canale selezionato. Motore (§2: `channels.rs`, `channels_uv.rs`, `api/canali`, `canale=`, `mzlab-cli channels`) e selettore della pagina (§5) esistono; §1 dice dove il kind decide ancora, §4 i lavori aperti.

## 1. Dove decide ancora il kind (prove nel codice)
**Classificazione (Rust).** `item_info` (`crates/mzlab-core/src/iteminfo.rs:459-530`) dà a ogni elemento `kind` ∈ `full|ms2|mrm|infusion|empty`: `infusion` se `infusion::infusione` (`infusion.rs:110`) riconosce precursori fissi (B: ≤64 chiavi MSn, mediana ≥10 scan) e blocchi lunghi (C) o metodo `.raw` senza LC o nome `infus|fia|direct|syringe`; poi `ms2` (MSn), `full` (solo MS1), `mrm` (solo SRM). Un file «misto» è spezzato in elementi (`file_parts`, `iteminfo.rs:92`: livello × tipo di scan PREC/NL/NG/EMC/TDF/ER × polarità + «MRM»), ognuno con il suo `kind`.

**Dove il kind cambia l'interfaccia (JS).** Registro unico `ESP` in `mzlab/web/esperimenti.js:6-16` (`spettri`, `vista`, `nodi`).
- Tabella «1. Data files» (`shell.js`): ordine `GORD` (85), gruppi (87-88), colonna Experiment `expCell` (99-102), avviso nome≠kind (103), apribili solo i 4 kind (139), concentrazione solo `mrm` (71).
- Schede di pannelli per kind: `explore.js:162` `tabFiles(t)` = file con `kind===t`; ogni blocco di pannelli appartiene a un kind.
- Cromatogramma: sulla scheda `ms2` forzato a TIC degli MS2 di un precursore, senza intervallo m/z, bianco, fondo, log (`ms2Chrom` in `pannelli.js`; un cromatogramma `pda`/`press`/`an:*` della scheda è ordinario); selettori distinti per lo stesso concetto: `kind` (tic/bpc/pda/press/an:*/reg, 660-662, 678), `prec` (663-664, tolleranza 0,6 Da o `ptol` del pannello, `grafici.js:85-86`), `tr` MS2 (`api/ms2tr`, 676-677, q3±0,5), `tr` MRM (687-689), `filt` solo nel Banco HR (`banco.js:38-71, 252-272`).
- MRM: tipo di pannello a parte `"mrm"` (`tabs.js:146-156`, `api/mrm`), esclusi da tabella, profilo, QA/QC, mappa (`tabella.js:61`, `profilo.js:20`, `qaqc.js:30`, `lr-map.js:10`).
- Infusione: albero `api/msn_albero` (`infusione.js:9-64`), schede «alb»/«gra» solo se `INF.any()`/`INF.single()` (`barra.js:105-106`), filtri `msn:<id>`, voci del menu dei picchi (`infusione-menu.js`).
- PDA: `kind=pda` del cromatogramma con nm in `mz0/mz1` (`uv-spettro.js:60-63`), spettro UV `api/uvspec`; analogici `api/analog?i` (`grafici.js:53-61`).
- La lente XIC lavora nel canale dello spettro (`api/xic?canale`, `xic-lente.js`); se è proposta lo dice il tipo (`ESP.predefinito`, campo `lente`).

**Motore: ciò che esiste già ed è quasi un modello a canali.**
- `Scan::key()` (`idxread.rs:132`) = filtro senza intervallo per MS1, `ms{n} <masse nominali> @act` per MSn; `/api/filters` (`routes.rs:212`) raggruppa gli scan per chiave `{key, level, n, rt0, rt1, pol}`, calcolato a richiesta, non salvato.
- `/api/chrom?kind=tic|bpc&level&prec&filt&mz0&mz1` (`routes.rs:151`) filtra per chiave (`Filtro::Testo`) o per nodo dell'albero (`filt=msn:<id>`, `msnroute.rs:55`); `/api/scanlist?filt` idem.
- Cromatogrammi non-scan: `mzml::ChromKind{Tic,Bpc,Pda,Pressure,Srm,Sic,Analog,Other}` (`mzml.rs:57`), lista a richiesta `mzmlhead::Chrom{index,id,kind,q1,q3,ce,name,dwell,n}` (`mzmlhead.rs:96`), nei `Facts` come `srm`, `pda`, `analog[]`.
- Lettura dei cromatogrammi SRM in blocco: `Mzml::chromatograms_each` (tempi e intensità) e `chromatograms_times` (solo tempi) leggono i cromatogrammi vicini insieme, in tratti fino a 16 MiB (`reading.rs::in_spans`), e decodificano ognuno dalla sua fetta: array identici bit per bit a `chromatogram`/`chromatogram_times`; uno oltre il tratto o fuori ordine fa tratto a sé. Usati da `facts_from` (`srm_rt`), `fileroutes::mrm_transitions` (`api/mrm`) e `Sorgente::srm`: nel sito ogni lettura è una fetta di Blob, quindi migliaia di transizioni costano poche letture (`reading_tests.rs`: 4000 SRM, <60).
- `.wiff`: modi in `wiff/modes.rs:42-60` (Q1, Q1MI, Q3, Q3MI, MRM, Prec, PI, NL, EPI, ER, MS3, TDF, EMS, EMC; confermati i codici 0, 4, 5, 6, 7, 11, 15); MRM → un TIC + un cromatogramma `SRM SIC Q1= Q3= … experiment= transition=` per transizione (`wiff/run.rs:25-49`); DAD → `DadData` (`wiff/dad.rs:33`).
- Thermo: filtro testuale in `Scan.filter`, `path` da `filter_path` (`scantable.rs:238`); SRM/CRM a scan → cromatogrammi per transizione (`mzlab-lettori/src/srm.rs`).

**Dove si perde o si mescola informazione.**
1. Il kind decide gli strumenti: un errore dell'euristica (MS2 mirato del bentazone → «infusione») spegne strumenti utili e ne accende di sbagliati.
2. Un file misto diventa più righe e più schede (DDA = «Full Scan» + «MS2»; MRM+EPI = due elementi): il file perde l'unità, il TIC complessivo non esiste.
3. Lo XIC senza `canale=` (`routes.rs` `xic_raw`, `api/xicbin`) somma TUTTI gli scan del livello: sul livello 2 mescola precursori ed energie diversi.
4. Quattro meccanismi per «quale sottoinsieme di scan»: `prec` (0,6 Da), `tr` MS2 (`ms2tr`), `filt`/`msn:` (solo HR e infusione), pannello `mrm`.
5. `.mzidx`: nessun id di esperimento, periodo o ciclo (per il `.wiff` stanno solo nel testo `native`, `wiff/run.rs:107-113`); si legge solo il `Period0` (`wiff/mod.rs:120`); finestra di scan, `p/c` e `[a-b]` del filtro Thermo non sono campi.
6. Cromatogrammi mzML non indicizzati (solo il numero in `run.n_chromatograms`): ogni lista rilegge la coda del file.
7. PDA Agilent e Shimadzu non decodificati (`agilent_ext/reader_lc.rs`, `mzlab-lettori/src/shimadzu/reader.rs`): il canale manca, non è nascosto.

## 2. Modello «file → canali»
**Principio.** Ogni file espone un elenco di canali; ogni canale ha una traccia (tempo → intensità) e, se li ha, i suoi scan con lo spettro (di massa o UV). Gli strumenti chiedono «canale + file», mai «kind».

**Famiglie e id stabili (stringhe, uguali in CLI, wasm e pagina).**
- `tic` (tutti gli scan MS, ordine del tempo; per un file solo MRM = somma delle transizioni, il TIC del costruttore se c'è), `bpc`.
- `ms:<chiave>`: un gruppo di scan con la stessa `Scan::key()` (MS1 full, SIM, EMS, EPI a precursore fisso, MS2/MS3… di un percorso). Figli di `tic`; per MSn il genitore è il canale del precursore (`ms:<chiave>` di livello n-1), così l'albero dei precursori È la gerarchia dei canali.
- `dd:<livello>[:<sigla>]`: aggregato degli MSn dipendenti dai dati (DDA, IDA EMS→EPI) quando le chiavi distinte di quel livello superano 64 (stesso tetto di `infusion.rs`): un solo canale con gli scan e i precursori come sotto-vista a richiesta (non 3 000 canali).
- `srm:<q1>><q3>[@ce]` (una transizione MRM/SRM/CRM), `sim:<id>` (cromatogrammi SIC), `uv:twc`, `uv:<nm>[±bw]` (lunghezza d'onda chiesta dall'utente: canale derivato), `uv:auto` (solo CLI: la banda proposta), `an:<i>` (pressione, flusso, UV singolo).
- `omologo`: chiave per sovrapporre file diversi (livello + masse nominali del percorso + attivazione + CE arrotondata; q1/q3 a 0,1; nm). Due file condividono un canale se hanno lo stesso `omologo`.

**Struttura in Rust (`crates/mzlab-core/src/channels.rs` e `channels/`; nomi dei campi in inglese nel codice).**
```rust
pub enum Famiglia { Tic, Bpc, Ms, Dd, Srm, Sim, Uv, Analog }
pub struct Canale {
    pub id: String,            // "ms:FTMS + p ESI ms3 317 261 @cid30"
    pub famiglia: Famiglia,
    pub genitore: Option<u32>, // indice del canale padre (tic, precursore)
    pub livello: u8,           // 0 per uv/analog
    pub pol: i8,
    pub sigla: Option<&'static str>, // EMS, EPI, MRM, Prec, NL, Q1MI… (modes.rs) o "SIM", "DDA"
    pub etichetta: String,     // breve, per la lista: "MS3 317→261 @CID30"
    pub omologo: String,
    pub n: u32,                // scan o punti
    pub rt: (f64, f64),
    pub spettri: bool,         // ha scan con spettro (ms, dd, uv)
    pub sorgente: Sorgente,    // dove sta la traccia
}
pub enum Sorgente { Scan { da: u32, a: u32 }, Cromatogramma(i64), Dad, DadNm(f64, f64) }
pub struct Canali { pub v: Vec<Canale>, pub scan_ids: Vec<u32>, pub tipo: Tipo }
```
`scan_ids` è una partizione degli scan MS ordinata per canale (`Sorgente::Scan{da,a}` = fetta): 4 B per scan, nessun dato spettrale copiato. Costruzione O(n scan) dalla tabella già in memoria (`idx.scans`, ~640 B/scan) + la lista dei cromatogrammi (`Facts`/`mzmlhead::Chrom`); fatta una volta per file e tenuta nelle `Routes` (100 000 scan → 400 KB + qualche centinaio di canali). Nessun cambio di formato del `.mzidx`.

**Tipo di esperimento come etichetta (`Tipo`).** `Tipo { proposto: &'static str, motivo, utente: Option<..> }` con valori descrittivi `lc-full`, `lc-ms2-mirato`, `lc-dda`, `lc-mrm`, `lc-sim` (solo cromatogrammi SIM, anche con una traccia UV), `infusione-msn`, `misto`, `solo-uv`; calcolato da `infusion.rs` + canali (`tipo_di`); è una proposta: il valore dell'utente, quando esiste, vince e sta nella sessione. Serve SOLO ai default (`ESP.predefinito` in `esperimenti.js`): canale iniziale, lente XIC proposta o no, scheda «alb» aperta all'avvio.

**API (rotte `/api`, stesse in wasm e desktop; JSON con versione).**
- `GET api/canali?k` → `{"v":"canali/1","tipo":{…},"canali":[{id,fam,gen,liv,pol,sigla,et,omo,n,rt0,rt1,spettri}]}`.
- `GET api/chrom?k&canale=<id>&kind=tic|bpc&mz0&mz1` → traccia del canale (per `srm:`/`uv:`/`an:` reindirizza al lettore attuale: `api/mrm`, `pda_trace`, `analog`).
- `GET api/xic?k&mz&tol&canale=<id>` → XIC dentro gli scan del canale (senza `canale=`: per livello).
- `GET api/scanlist?k&canale=<id>`, `api/spectrum?k&canale=<id>&rt|scan&modo=somma`.
Adattatore: `canale=ms:<k>` ≡ `filt=<k>`, `canale=msn:<nodo>` resta, `level`/`prec` restano accettati e tradotti; nessuna rotta vecchia sparisce finché un test la usa. In JS un solo modulo `canali.js`: `canali(k)` (cache), `traccia(k, id, opz)`, `omologhi(files, omo)`. CLI: `mzlab-cli channels FILE` (`cli.md`).

**Costruzione per formato.**
- mzML/mzXML e Thermo `.raw`: chiavi da `Scan::key()`; SIM = chiave MS1 con «SIM»; cromatogrammi `srm`/`pda`/`pressure` dalla coda (`mzmlhead`).
- `.wiff` (EMS, EPI, MRM; `channels/wiff.rs`): sigla da `modes.rs` per livello; MRM → `srm:` dai cromatogrammi per transizione (nome del metodo come etichetta); EMS/EPI → `ms:`, o `dd:2:EPI` se IDA; Prec/NL → `ms:` con sigla; DAD → `uv:`.
- Shimadzu/Agilent `.D`/Waters: dai cromatogrammi dei lettori (`ChromKind`); il PDA entra quando il lettore lo decodifica.

## 3. Regole e decisioni in vigore
- Nomi: «Canali» (it) / «Channels» (en); «Tipo di esperimento» resta come etichetta. Nomi visibili del Tipo: «LC full scan», «LC MS²», «LC DDA», «LC MRM», «MSn infusion», «mixed», «UV only» (id interni invariati).
- TIC del file = tutti gli scan MS (come il TIC del costruttore), con `MS1` come figlio.
- DDA: tetto 64 chiavi per livello prima dell'aggregato `dd:`.
- Default per tipo (canale iniziale, lente proposta): tabella `predefiniti` in `esperimenti.js`, modificabile; un default, mai un permesso: un tipo non spegne mai uno strumento.
- Sovrapposizione di file: il pannello salva il canale come `omologo`, non come indice; ogni file risolve il suo; un file che non lo ha compare in legenda «—» (nessun errore). Overlay con CE arrotondata all'intero, CE esatta nel `title` del canale.
- La scheda «alb» esiste per ogni file con canali MSn (`infusione-albero.js` legge `api/canali`), non solo per le infusioni.
- Testi: niente paragrafi dove l'interfaccia si capisce da sola; tooltip solo per dire cosa fa un campo o cosa inserirvi.
- Nessuna dipendenza nuova; nessun cambio del formato `.mzidx`.

## 4. Lavori aperti
- **Una riga per file** nella tabella «1. Data files»: `file_parts` (`iteminfo.rs`) spezza ancora un file misto in più righe; le parti diventano canali, colonna «Tipo» come etichetta modificabile (il campo dell'utente in `Tipo` non esiste ancora), colonna «Canali» (n). Il registro `ESP` diventa solo `default` per tipo.
- **Schede per kind** (`tabFiles` in `explore.js`): la direzione è un'area di lavoro unica, con gli stessi strumenti per ogni file. Schede e `file_parts` si spengono per ultimi (molti test e2e li usano).
- **Selettori doppioni**: `prec` (0,6 Da), `tr` MS2 (`api/ms2tr`), `filt`/`msn:` restano dietro l'adattatore finché e2e li usano, poi si spengono per primi; il pannello `mrm` resta un tipo a sé.
- **Lista scan** del canale selezionato (`api/scanlist?canale`) in ogni vista, anche in LR (la usa solo il Banco HR, `banco.js`).
- **`.wiff`**: periodi oltre `Period0` ed esperimento come campo (colonna `exp` facoltativa del `.mzidx` solo se il testo `native` non basta).
- **PDA** Agilent e Shimadzu (§1, punto 7).
- **Flow**: il nodo Raw files mostra `tipo` e numero di canali da `api/canali`; i passi ricevono un canale (`ms:<chiave MS1>` per le feature, `srm:` per la quantificazione) invece di indovinare il livello dal kind.

## 5. Selettore dei canali (pagina Dati)
`canali.js` (`window.CANALI`, `canali.css`, chiavi `canali.*`): il selettore unico del cromatogramma (pulsante `data-o=kind`, etichetta «TIC», «TIC + 1») apre un albero a gruppi (`role=tree`, frecce/Spazio/Invio/Esc): «Spettrometria di massa» (nome senza la sigla del modo, già nella colonna «Esperimento»; a scelta multipla: `p.canali`, salvato da `panelRec`; solo le famiglie che `traccia()` disegna), «UV/Vis» (DAD), «Rivelatori» (analogici) e «Registri», righe a scelta singola (`data-kind`, impostano `p.kind`); il pannello MRM ha «Transizioni» / «Ioni» (`data-o=canali`, `p.tr`).
- UV/Vis fuori dal Full Scan: se i file della scheda hanno un DAD o una traccia analogica, il selettore MRM/SIM e il cromatogramma MS2 (riga fissa «MS2» al posto del `select` spento) hanno i gruppi «UV/Vis» e «Rivelatori»: la riga apre (o mostra, se c'è già) un cromatogramma `kind` di quel rivelatore nella stessa scheda, sotto il pannello, con l'asse del tempo collegato (`CANALIUV` in `canali-uv.js`, caricato su richiesta, annullabile); `addPanel` dalla scheda `mrm` lascia lì un cromatogramma DAD/analogico. Senza segnale i gruppi mancano. Prova: `tests_e2e/e2e_dad_mrm.py`.
Selettore MRM/SIM (`ESP.mrm`): clic sul nome = solo quel canale; casella `.can-hit` (24 px), Ctrl/⌘+clic o Spazio = aggiungi/togli (l'ultimo resta); Invio = solo quello; «Tutti» (`.can-all`) = `p.tr` vuoto; «Nessuno» (`.can-none`) = `p.tr` `[]`, grafico vuoto con `canali.pick.none.msg`. Aiuto `.can-hint` `sr-only` (`aria-describedby`) e `title` dell'albero.
Memoria della scelta (`MRM_MEM` in `tabs.js`, solo sessione): ogni disegno di un pannello MRM/SIM ricorda `p.tr` per tipo (ioni SIM / transizioni) se non è la scelta automatica del layout (`p._trAuto`); al layout successivo (campione tolto e rimesso, rimosso, un altro campione) `mrmRecall` riapre i canali ricordati che i file mostrati hanno, uguali per tutti i campioni sovrapposti; se non ce n'è nessuno, il quantificatore/qualificatore come sempre. Prova: `e2e_sim_memoria.py`.
Solo ioni SIM (`can-sim`): una colonna, il m/z (o «ione N»), la polarità solo se i file le hanno entrambe; nome del motore, dwell, punti e tempo nel `title`. Prova: `e2e_sim_ui.py`.
Una traccia per file × canale, «—» in legenda se il file non ha il canale, XIC dentro il canale con `canale=`. Il motore serve `api/canali` (`Routes::canali`, metodo wasm `canali`) e `canale=` su chrom/xic/scanlist/spectrum/nearest_scan; transizioni e UV passano da `api/mrm`. Il canale iniziale per tipo sta in `ESP.predefinito` (`esperimenti.js`).
