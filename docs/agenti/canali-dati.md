# Canali dei dati: un file = un insieme di canali
Studio architetturale (ARCH-H). Il «tipo di esperimento» diventa un'etichetta e un'euristica per i default; gli strumenti lavorano sul canale selezionato. Nulla qui è ancora implementato: le sezioni 1-4 descrivono lo stato e il piano, la 6 i prompt dei DEV.

## 1. Stato attuale (prove nel codice)
**Classificazione (Rust).** `item_info` (`crates/mzlab-core/src/iteminfo.rs:459-530`) dà a ogni elemento `kind` ∈ `full|ms2|mrm|infusion|empty`: `infusion` se `infusione::infusione` (`infusione.rs:110`) riconosce precursori fissi (B: ≤64 chiavi MSn, mediana ≥10 scan) e blocchi lunghi (C) o metodo `.raw` senza LC o nome `infus|fia|direct|syringe`; poi `ms2` (MSn), `full` (solo MS1), `mrm` (solo SRM). Un file «misto» è spezzato in elementi (`file_parts`, `iteminfo.rs:92`: livello × tipo di scan PREC/NL/NG/EMC/TDF/ER × polarità + «MRM»), ognuno con il suo `kind`.

**Dove il kind cambia l'interfaccia (JS).** Registro unico `ESP` in `mzlab/web/esperimenti.js:6-16` (`spettri`, `vista`, `nodi`).
- Tabella «1. Data files» (`shell.js`): ordine `GORD` (85), gruppi (87-88), colonna Experiment `expCell` (99-102), avviso nome≠kind (103), apribili solo i 4 kind (139), concentrazione solo `mrm` (71).
- Schede di pannelli per kind: `explore.js:162` `tabFiles(t)` = file con `kind===t`; ogni blocco di pannelli appartiene a un kind.
- Cromatogramma: sulla scheda `ms2` forzato a TIC degli MS2 di un precursore, senza intervallo m/z, bianco, fondo, log (`pannelli.js:643-645`); selettori distinti per lo stesso concetto: `kind` (tic/bpc/pda/press/an:*/reg, 660-662, 678), `prec` (663-664, tolleranza 0,6 Da, `grafici.js:85-86`), `tr` MS2 (`api/ms2tr`, 676-677, q3±0,5), `tr` MRM (687-689), `filt` solo nel Banco HR (`banco.js:38-71, 252-272`).
- MRM: tipo di pannello a parte `"mrm"` (`tabs.js:146-156`, `api/mrm`), esclusi da tabella, profilo, QA/QC, mappa (`tabella.js:61`, `profilo.js:20`, `qaqc.js:30`, `lr-map.js:10`).
- Infusione: albero `api/msn_albero` (`infusione.js:9-64`), schede «alb»/«gra» solo se `INF.any()`/`INF.single()` (`barra.js:105-106`), filtri `msn:<id>`, voci del menu dei picchi (`infusione-menu.js`).
- PDA: `kind=pda` del cromatogramma con nm in `mz0/mz1` (`uv-spettro.js:60-63`), spettro UV `api/uvspec`; analogici `api/analog?i` (`grafici.js:53-61`).
- La lente XIC non guarda il kind (`xic-lente.js:175,196`): il ramo `claude/xic-infusione` (VISORE-ZOOM) la spegne sulle infusioni.

**Motore: ciò che esiste già ed è quasi un modello a canali.**
- `Scan::key()` (`idxread.rs:132`) = filtro senza intervallo per MS1, `ms{n} <masse nominali> @act` per MSn; `/api/filters` (`routes.rs:212`) raggruppa gli scan per chiave `{key, level, n, rt0, rt1, pol}`, calcolato a richiesta, non salvato.
- `/api/chrom?kind=tic|bpc&level&prec&filt&mz0&mz1` (`routes.rs:151`) filtra per chiave (`Filtro::Testo`) o per nodo dell'albero (`filt=msn:<id>`, `msnroute.rs:55`); `/api/scanlist?filt` idem.
- Cromatogrammi non-scan: `mzml::ChromKind{Tic,Bpc,Pda,Pressure,Srm,Sic,Analog,Other}` (`mzml.rs:57`), lista a richiesta `mzmlhead::Chrom{index,id,kind,q1,q3,ce,name,dwell,n}` (`mzmlhead.rs:96`), nei `Facts` come `srm`, `pda`, `analog[]`.
- `.wiff`: modi in `wiff/modi.rs:42-60` (Q1, Q1MI, Q3, Q3MI, MRM, Prec, PI, NL, EPI, ER, MS3, TDF, EMS, EMC; confermati i codici 4, 5, 7, 11, 15); MRM → un TIC + un cromatogramma `SRM SIC Q1= Q3= … experiment= transition=` per transizione (`wiff/run.rs:25-49`); DAD → `DadData` (`wiff/dad.rs:33`).
- Thermo: filtro testuale in `Scan.filter`, `path` da `filter_path` (`scantable.rs:238`); SRM/CRM a scan → cromatogrammi per transizione (`mzlab-lettori/src/srm.rs`).

**Dove si perde o si mescola informazione.**
1. Il kind decide gli strumenti: un errore dell'euristica (MS2 mirato del bentazone → «infusione») spegne strumenti utili e ne accende di sbagliati.
2. Un file misto diventa più righe e più schede (DDA = «Full Scan» + «MS2»; MRM+EPI = due elementi): il file perde l'unità, il TIC complessivo non esiste.
3. Lo XIC (`routes.rs:125` `xic_raw`) somma TUTTI gli scan del livello: sul livello 2 mescola precursori ed energie diversi; non ha `filt`.
4. Quattro meccanismi per «quale sottoinsieme di scan»: `prec` (0,6 Da), `tr` MS2 (`ms2tr`), `filt`/`msn:` (solo HR e infusione), pannello `mrm`.
5. `.mzidx`: nessun id di esperimento, periodo o ciclo (per il `.wiff` stanno solo nel testo `native`, `wiff/run.rs:107-113`); si legge solo il `Period0` (`wiff/mod.rs:120`); finestra di scan, `p/c` e `[a-b]` del filtro Thermo non sono campi.
6. Cromatogrammi mzML non indicizzati (solo il numero in `run.n_chromatograms`): ogni lista rilegge la coda del file.
7. PDA Agilent e Shimadzu non decodificati (`agilent_ext/lettore_lc.rs:11`, `mzlab-lettori/src/shimadzu/lettore.rs:20`): oggi il canale manca, non è nascosto.

## 2. Modello «file → canali»
**Principio.** Ogni file espone un elenco di canali; ogni canale ha una traccia (tempo → intensità) e, se li ha, i suoi scan con lo spettro (di massa o UV). Gli strumenti chiedono «canale + file», mai «kind».

**Famiglie e id stabili (stringhe, uguali in CLI, wasm e pagina).**
- `tic` (tutti gli scan MS, ordine del tempo; per un file solo MRM = somma delle transizioni, il TIC del costruttore se c'è), `bpc`.
- `ms:<chiave>`: un gruppo di scan con la stessa `Scan::key()` (MS1 full, SIM, EMS, EPI a precursore fisso, MS2/MS3… di un percorso). Figli di `tic`; per MSn il genitore è il canale del precursore (`ms:<chiave>` di livello n-1), così l'albero dei precursori È la gerarchia dei canali.
- `dd:<livello>[:<sigla>]`: aggregato degli MSn dipendenti dai dati (DDA, IDA EMS→EPI) quando le chiavi distinte di quel livello superano 64 (stesso tetto di `infusione.rs`): un solo canale con gli scan e i precursori come sotto-vista a richiesta (non 3 000 canali).
- `srm:<q1>><q3>[@ce]` (una transizione MRM/SRM/CRM), `sim:<id>` (cromatogrammi SIC), `uv:twc`, `uv:<nm>[±bw]` (lunghezza d'onda chiesta dall'utente: canale derivato), `an:<i>` (pressione, flusso, UV singolo).
- `omologo`: chiave per sovrapporre file diversi (livello + masse nominali del percorso + attivazione + CE arrotondata; q1/q3 a 0,1; nm). Due file condividono un canale se hanno lo stesso `omologo`.

**Struttura in Rust (`crates/mzlab-core/src/canali.rs`, ≤500 righe, nuovo).**
```rust
pub enum Famiglia { Tic, Bpc, Ms, Dd, Srm, Sim, Uv, Analog }
pub struct Canale {
    pub id: String,            // "ms:FTMS + p ESI ms3 317 261 @cid30"
    pub famiglia: Famiglia,
    pub genitore: Option<u32>, // indice del canale padre (tic, precursore)
    pub livello: u8,           // 0 per uv/analog
    pub pol: i8,
    pub sigla: Option<&'static str>, // EMS, EPI, MRM, Prec, NL, Q1MI… (modi.rs) o "SIM", "DDA"
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
`scan_ids` è una partizione degli scan MS ordinata per canale (`Sorgente::Scan{da,a}` = fetta): 4 B per scan, nessun dato spettrale copiato. Costruzione O(n scan) dalla tabella già in memoria (`idx.scans`, ~640 B/scan) + la lista dei cromatogrammi (`Facts`/`mzmlhead::Chrom`); fatta una volta per file e tenuta nelle `Routes` (100 000 scan → 400 KB + qualche centinaio di canali). Niente cambio di formato del `.mzidx` nel primo passo; in un passo successivo (DEV-b) una colonna `exp` facoltativa (esperimento/periodo del `.wiff`) se il testo `native` non basta.

**Tipo di esperimento come etichetta (`Tipo`).** `Tipo { proposto: &'static str, motivo, utente: Option<..> }` con valori descrittivi `lc-full`, `lc-ms2-mirato`, `lc-dda`, `lc-mrm`, `infusione-msn`, `misto`, `solo-uv`; calcolato da `infusione.rs` + canali; il valore dell'utente (menu di VISORE-ZOOM) vince e sta nella sessione. Serve SOLO ai default (§3): canale iniziale, lente XIC proposta o no, scheda «alb» aperta all'avvio.

**API (rotte `/api`, stesse in wasm e desktop; JSON con versione).**
- `GET api/canali?k` → `{"v":"canali/1","tipo":{…},"canali":[{id,fam,gen,liv,pol,sigla,et,omo,n,rt0,rt1,spettri}]}`.
- `GET api/chrom?k&canale=<id>&kind=tic|bpc&mz0&mz1` → traccia del canale (per `srm:`/`uv:`/`an:` reindirizza al lettore attuale: `api/mrm`, `pda_trace`, `analog`).
- `GET api/xic?k&mz&tol&canale=<id>` → XIC dentro gli scan del canale (oggi per livello).
- `GET api/scanlist?k&canale=<id>`, `api/spectrum?k&canale=<id>&rt|scan&modo=somma`.
Adattatore: `canale=ms:<k>` ≡ `filt=<k>`, `canale=msn:<nodo>` resta, `level`/`prec` restano accettati e tradotti; nessuna rotta vecchia sparisce finché un test la usa. In JS un solo modulo `canali.js` (`m:`, `@ts-check`): `canali(k)` (cache), `traccia(k, id, opz)`, `omologhi(files, omo)`. CLI: `mzlab-cli canali FILE` (Regola CLI).

**Costruzione per formato.**
- mzML/mzXML e Thermo `.raw`: chiavi da `Scan::key()`; SIM = chiave MS1 con «SIM»; cromatogrammi `srm`/`pda`/`pressure` dalla coda (`mzmlhead`).
- `.wiff`: sigla da `modi.rs` per esperimento; MRM → `srm:` dai cromatogrammi per transizione (nome del metodo come etichetta); EMS/EPI → `ms:`, o `dd:2:EPI` se IDA; Prec/NL → `ms:` con sigla; DAD → `uv:`.
- Shimadzu/Agilent `.D`/Waters: dai cromatogrammi dei lettori (`ChromKind`); PDA appena decodificato (fuori da questo piano).

## 3. Interfaccia proposta
**Selettore dei canali** (sostituisce i selettori `kind`, `prec`, `tr`, `filt` del pannello cromatogramma e il pannello `mrm`): un albero `role="tree"` per file, TIC in cima; colonne Canale · Sigla · n · RT. Più file: le righe si uniscono per `omologo`, con il numero di file che lo hanno.
```
Canale                         Sigla   n     RT (min)
▾ TIC                                  4 812  0.0–30.0
  ▸ MS1  [100–1000]            Full    2 406  0.0–30.0
  ▾ MS2  222.1 @CID35          MS2     2 406  0.0–30.0     ← bentazone mirato
  BPC                                  4 812
▾ UV (DAD)                     TWC     9 000  0.0–30.0
  · 254 nm ±4                          9 000             [+ λ]
  Pressione (bar)                      6 000
```
- Timololo (infusione MSn a segmenti): `TIC` › `MS1` (0–1 min) › `MS2 317` (1–2) › `MS3 317→261` (2–3) … `MS8`; la colonna RT rende visibili i segmenti; default = spettro sommato del canale più profondo e scheda «alb» aperta; la lente XIC c'è ma non è proposta.
- MS2 mirato in LC (bentazone): TIC, MS1, MS2 222; default = cromatogramma del canale MS2 + spettro all'apice; XIC sui frammenti dentro il canale MS2.
- MRM QTRAP: `TIC (Σ transizioni)` › `MRM 230→162 @25 «bentazone 1»` …; spuntare più transizioni = più tracce; doppio clic su una transizione non apre uno spettro (canale senza `spettri`).
- EMS→EPI (IDA): `EMS` (MS1) e `EPI · dipendenti` (`dd:2:EPI`, n precursori); la lista scan del canale mostra i precursori.
- DDA Orbitrap: `TIC`, `MS1 FTMS`, `MS2 · DDA (2 811 precursori)`; XIC su MS1; il precursore si sceglie dall'albero o dalla lista scan.
- DAD Shimadzu/`.wiff`: `UV TWC` e `λ` aggiunte dall'utente; spettro UV per scan.
- SIM: un canale `SIM [m1–m2]` per finestra (Thermo) o `sim:` (SIC del `.wiff` Q1MI).

**Sovrapposizione di file.** Il pannello salva il canale come `omologo`, non come indice: ogni file risolve il suo; un file che non lo ha compare in legenda «—» (nessun errore). La stessa transizione in 12 campioni = 12 tracce di colori Okabe-Ito; tabella dei dati (`TDATI`) con una colonna per file.

**Pannello XIC / lente.** Lavora nel canale selezionato (`api/xic?canale`). Default: lente proposta se il canale ha `spettri` e un intervallo RT > 1 min con più di 20 scan; mai bloccata.

**Albero dei precursori.** Diventa la vista ad albero dei canali MSn (`ms:` con `genitore`); la scheda «alb» esiste per ogni file con MSn, non solo per le infusioni; tolleranza/CE/somma restano opzioni di raggruppamento (`msn:<nodo>` resta un canale derivato).

**Lista scan.** Sempre del canale selezionato (`api/scanlist?canale`), anche in LR.

**Testi.** Pochi testi: niente paragrafi dove l'interfaccia si capisce da sola; tooltip solo per dire cosa fa un campo o cosa inserirvi, mai opinioni, avvisi o cautele.

**Schede dei kind.** Nel primo giro restano (adattatore: la scheda sceglie il canale iniziale); quando i pannelli usano solo canali, le schede `full/ms2/mrm` si fondono in un'area di lavoro unica (decisione di Federico, §5).

## 4. Migrazione senza rompere
1. **Motore (DEV-a)**: `canali.rs` + `api/canali` + parametro `canale=` su `chrom`, `xic`, `scanlist`, `spectrum`, accanto ai parametri vecchi; parità CLI; nessun JS. Nessun test esistente cambia.
2. **.wiff (DEV-b)**: sigle, IDA, periodi > 0, esperimento come campo; utile subito alle priorità 1-2 (lettura nativa, `.wiff`→mzML: gli id SRM restano quelli di msconvert).
3. **UV/analogici (DEV-c)**: `uv:` e `an:` nella stessa lista; `uv:<nm>` derivato.
4. **Selettore (DEV-d)**: `canali.js` + selettore nel pannello cromatogramma; i selettori `kind/prec/tr` restano dietro l'adattatore finché `e2e` li usa, poi si spengono per primi (sono i doppioni).
5. **Viste (DEV-e)**: albero e lista scan come viste di canale; scheda «alb» per ogni MSn.
6. **Tabella (DEV-f)**: una riga per file (le parti diventano canali), colonna «Tipo» come etichetta modificabile, colonna «Canali» (n). Il registro `ESP` diventa `default` per tipo, non permessi.
Spegnere per ultimo: le schede per kind e `file_parts` (molti test e2e le usano).

**Sovrapposizioni con le chat attive.**
- VISORE-ZOOM (ramo `claude/xic-infusione`: lente XIC, classificazione, menu del tipo, barra spettro, titolo MSⁿ, mirino): il suo menu del tipo È il campo `Tipo.utente`; DEV-f lo riusa, non ne scrive un altro. DEV-d parte dopo l'unione di `claude/xic-infusione` e trasforma la sua condizione «infusione → lente spenta» nel default per tipo.
- ALBERO-PRECURSORI-UI (session_01Ly8ApkjDoaTTpDJiPcbjag): tocca `infusione-albero.js`; DEV-e parte dopo e usa `genitore` dei canali senza riscriverne la grafica.
- FILE-STRUMENTO-UI (session_01Ly8ApjSQyuqbLkkPmdEFrK): «file e strumento» legge `api/canali` per l'elenco degli esperimenti invece di ricostruirlo da `api/method`.
- ARCH-G FLOW-UX (session_01Ly8Apo2HXL1bfkPsxsxfBF, nodo Raw files): il nodo mostra `tipo` e numero di canali da `api/canali`; nessuna logica di classificazione nel Flow.
Pipeline TP (priorità 3): i passi del Flow ricevono un canale (`ms:<chiave MS1>` per le feature, `srm:` per la quantificazione) invece di indovinare il livello dal kind.

## 5. Decisioni per Federico (lettura scelta in attesa di risposta)
- Nomi: «Canali» (it) / «Channels» (en); «Tipo di esperimento» resta come etichetta. Scelto così.
- TIC del file = tutti gli scan MS (come il TIC del costruttore), con `MS1` come figlio. Scelto così.
- Schede `full/ms2/mrm`: restano finché DEV-d/e non sono uniti; la fusione in un'area unica è una decisione a parte.
- Una riga per file nella tabella (niente parti). Scelto così, in DEV-f.
- DDA: tetto 64 chiavi per livello prima dell'aggregato `dd:`. Scelto così.
- Default per tipo (canale iniziale, lente proposta): tabella in `esperimenti.js`, modificabile.
- Nessuna dipendenza nuova; nessun cambio del formato `.mzidx` salvo la colonna `exp` facoltativa di DEV-b (da confermare).

## 6. Prompt dei DEV
Uno per file in `docs/agenti/prompt-canali/` (`a.md` motore, `b.md` `.wiff`, `c.md` UV e analogici, `d.md` selettore, `e.md` albero e lista scan, `f.md` tabella): formato a 7 punti, con ramo e dipendenze in testa. Ordine: A → (B, C, D in parallelo; D dopo `claude/xic-infusione`) → (E dopo ALBERO-PRECURSORI-UI, F) in parallelo.
