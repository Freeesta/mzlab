# Attributio: liste regolatorie come istantanea in sola lettura (Q5, dentro RG-03 e RG-05)

Proposta, nessun codice; parti da realizzare dentro RG-03 e RG-05. Fonti: `docs/studi/ATTRIBUTIO.md` §1 (destino «Dati») e §8 Q5; il pacchetto Python `reglists` di Attributio letto per noi dalla chat locale (~2 760 righe + `sources/` ~1 350); PR #1053/#1054/#1058 (binario RG: registro, ecotox, liste). Tabui resta solo in inglese; Attributio è dietro `uipAdminOn()`.

## 1. Cosa serve davvero all'utente
- **Domanda 1 (identità nota)**: «questa feature, che il Banco ha già riportato a una struttura, sta in qualche lista regolatoria o prioritaria, e perché?» Chiave: InChIKey intero; se manca, solo il primo blocco (14 caratteri), sempre segnalato come ripiego.
- **Domanda 2 (nessuna identità)**: «tra le voci delle liste che il metodo può vedere, quali hanno uno ione ([M+H]+, [M+NH4]+, [M+Na]+ in ESI+; [M-H]-, [M+Cl]- in ESI−, ±5 ppm) che coincide con questa feature?» Sono **candidati per massa**, non identificazioni: la classe del colpo (A-D, X, come in `screen.py`) dice quanta prova c'è. Vale il principio 4 di AGENTS.md (a risoluzione unitaria un m/z è un candidato) e il programma non dice «ecco i composti pericolosi».
- **Cosa si mostra**: nome della voce, lista (con versione e data dell'istantanea), `list_attribute` (criteri HHP, categoria WHO, motivo SVHC, EQS…), `is_tp` e `parent`, `lc_esi_amenable` con la regola scritta, `resolved_by`, tipo di abbinamento (chiave intera / primo blocco). Una tabella (con esportazione CSV), un distintivo nella riga del Banco, nessun grafico (quindi niente `TDATI` nuovo).
- **Regole da non perdere** (dal sorgente): abbinare solo per InChIKey, mai per nome o formula; lavorare sulla forma rilevabile (parent organico del sale) con la regola di amenability già scritta nell'istantanea; assenza = «non rilevata con questo metodo», mai «assente»; niente scartato in silenzio (colpi di sola massa con la loro classe; voci non risolte contate e mostrate come numero); finestra MS1 e liste da interrogare sono scelte della campagna (`[reglists]` di `campaign.toml`), non del codice; confronto tra campagne solo per primo blocco (fuori da questo pacchetto).
- **Fuori da RL**: costruzione delle liste (rete, PDF, ECHA a mano) e `resolve`/`build` (RG-04); `amenability` come calcolo (RG-03: qui si **legge** il valore già calcolato); `confirm` B3 (SIRIUS/MetFrag) e `compare` P3; ecotossicologia (RG-02).

## 2. Rapporto con il binario RG (da decidere con il Coordinatore)
Il piano RG (#1053) mette le liste in RG-03 (logica) e RG-05 (pannello). RG-00 (#1054) ha già `tox/snapshot.rs` con le 18 colonne di `reglist_membership.csv` (`MEMBERSHIP_COLUMNS`, `MembershipRow`) e mini-istantanee inventate. RL **non rifà** quel lettore: RL aggiunge (a) il formato compatto servito al sito e il suo scrittore, (b) B1 e B2 in sola lettura sopra quel tipo, (c) la scheda «Regulatory lists» a metà di RG-05. **Decisione**: nessun pacchetto RL separato. Le parti di questo documento entrano in RG-03 (`match`, `screen`, tabella di appartenenza, scrittore e formato `reglists/1`) e in RG-05 (scheda «Regulatory lists», `.wasm`); `schema`, `catalog`, `confirm`, `compare` e `amenability` restano in RG-03 come da piano. «RL» nei paragrafi seguenti indica queste parti.

## 3. Formato compatto `reglists/1`
Un solo file, testo UTF-8 a sezioni (nessuna dipendenza nuova: lo legge `frame.rs`), servito da `static/` e caricato **a richiesta** quando l'utente apre la scheda; il `.wasm` non contiene dati.
- **Testata**: `format=reglists/1`, `snapshot_date`, `built_from` (sha256 del `reglist_membership.csv` di partenza), numero di righe per sezione, `redistributable_only` (vero/falso).
- **Sezione `lists`** (una riga per lista, ~28): `list_id`, `list_group`, `title`, `body` (ente), `act`, `list_version`, `access_date`, `url`, `license`, `redistributable` (0/1), `n_entries`.
- **Sezione `compounds`** (una riga per primo blocco, ~4 800): `first_block`, `inchikey` (completa), `formula`, `mono_mass` (4 decimali), `amenable` (0/1/2 = vero/dubbio/falso), `rule` (indice in un dizionario di regole), ordinata per `mono_mass` (ricerca per massa con bisezione, nessun indice a parte).
- **Sezione `entries`** (una riga per appartenenza, ~7 700): indice del composto, indice della lista, `name`, `list_attribute` (indice in un dizionario), `is_tp`, `parent`, `resolved_by` (0-3: struttura, CAS, nome, non risolta).
- **Tolti rispetto al CSV**: `smiles` e `xlogp` (peso, e il Banco ha già le sue strutture), `cas` e `pubchem_cid` restano solo come colonne facoltative dell'ultima sezione. Le 2 373 voci non risolte non entrano come righe: stanno come conteggio per lista nella sezione `lists` (`n_unresolved`), con il file completo nel privato.
- **Dimensione attesa (da misurare alla consegna)**: ~120 B per appartenenza e dizionari condivisi ⇒ ~0,9 MB grezzi con tutte le liste, ~250 KB dopo gzip del server. Tetti proposti: grezzo ≤ 1,5 MB, trasferito ≤ 400 KB; un test fallisce se il file nel repository li supera.
- **Integrità**: sha256 nel manifest del sito (`tools/build_site.py`, come i `.wasm`), cache del browser nella cache esistente `mzlab-big-*`; versione sconosciuta ⇒ `error.newerFormat` come negli altri derivati (`docs/agenti/formato-nuovo.md`).

## 4. Licenze e che cosa va nel sito (decisione di Federico, Q-RL1)
Dal catalogo: **PlastChem** CC BY-NC 4.0 (2 845 voci); **WHO_PEST_HAZARD** e **WHO_MIA** CC BY-NC-SA 3.0 IGO (non commerciali e share-alike); **EDLISTS** e **HHP_PAN** senza licenza dichiarata (uso con citazione); SVHC con citazione (ECHA); le altre UE, UBA (CC BY 4.0), EPA (pubblico dominio), Stoccolma/Rotterdam (con citazione), atti italiani e thailandesi sono distribuibili con la fonte.
- **Predefinito proposto**: l'istantanea pubblica contiene solo le liste con `redistributable = 1`, cioè senza PlastChem, WHO_* ed EDLISTS/HHP_PAN, e porta in vista la citazione di ogni fonte (`LICENZE-TERZI.md` ottiene una riga per le liste). Stima con questa scelta: ~3 400 appartenenze su 7 707.
- **Istantanea completa**: lo stesso formato con `redistributable_only = falso`, scelta dall'utente da una cartella sua (come le librerie `mzlab-libs`), mai nel repository né nel sito. Il lettore è lo stesso; la scheda mostra quale delle due è in uso.
- Un test impedisce che una lista con `redistributable = 0` finisca nel file del repository.

## 5. Costruzione e aggiornamento dell'istantanea
- **Chi costruisce**: Federico sul Mac, con Attributio (`attributio liste --costruisci`), che produce `reglist_membership.csv` e il catalogo. Il passo nuovo è un comando Rust, `mzlab-cli reglists-pack` (stesso crate della CLI esistente, nessun Python nuovo), che legge quei due file e scrive `reglists-1.rl` con i filtri di licenza; stampa le dimensioni e l'elenco delle liste incluse/escluse.
- **Dove sta**: `mzlab/web/dati/reglists-1.rl` (dati derivati da liste pubbliche, non dati del laboratorio; il privato resta per le liste non distribuibili). Un aggiornamento = un commit del file + la riga di versione nel manifest; la scheda mostra `snapshot_date` e avvisa se l'istantanea ha più di 12 mesi.
- **Fonti manuali**: la copia SVHC ferma al 04.02.2026 (l'ufficiale del 03.06.2026 ha 3 voci in più) e `fonte.toml` scritto a mano restano compiti di Attributio; la scheda riporta la `list_version` di ogni lista, così la differenza è visibile.

## 6. Codice e tetti
- **Rust**: modulo `crates/mzlab-feat/src/attributio/reglists/` (`pack.rs` scrittura, `read.rs` lettura sopra `tox::snapshot::MembershipRow`, `match_.rs` B1, `screen.rs` B2), ogni file con `// Ported from Attributio reglists/<file>.py.`; funzioni ≤ 80 righe.
- **`.wasm`**: feature `liste` dello stesso crate `mzlab-attributio-wasm` ⇒ nuovo `attributio-liste` in `tools/build_feat_wasm.sh` con tetto proprio (stima < 100 KB, da fissare a misura + 3 %), come `-rev` ed `-export`. Se RG-05 crea `attributio-dati` per primo, RL entra lì. **Non si toccano** i tetti di finder (512 000), sciex (327 680), chem (569 344), engine (1 703 936) né quelli già esistenti di attributio.
- **Pagina**: `TP_Mine/js/5N_attributio_liste.js` (`// @ts-check`, in `tools/tipi/pagina.json`), testi in inglese senza chiavi `lang/`, mai «formula» da sola; scheda «Regulatory lists» solo con `uipAdminOn()`; bersagli ≥ 24 px e tabella navigabile da tastiera (WCAG 2.2 AA). Desktop: rotta `attributio/liste/*` in `feat_attributio.rs`. Tour: la scheda è dietro l'amministratore, il tour non cambia.
- **Rete**: nessuna richiesta esterna (CSP `connect-src 'self'`), il file arriva dal sito.

## 7. Prove
- **Parità** con Attributio: `tools/attributio_riferimento.py` ottiene gli stadi `reglists-match` e `reglists-screen` (Python solo riferimento) su una mini-istantanea inventata e la campagna sintetica; uguaglianza di chiavi, tipo di abbinamento, classi e conteggi; massa ±5 ppm identica; dati d'oro piccoli (< 100 KB in tutto).
- **Pack/lettura**: andata e ritorno CSV → `reglists/1` → righe uguali; versione sconosciuta rifiutata; file troncato = errore con chiave, mai panico.
- **Dimensioni**: il file di repository ≤ tetti del §3; nessuna lista non distribuibile nel file pubblico; `wasm` sotto il tetto.
- **e2e**: scheda dietro amministratore, tabella dei colpi, distintivo nel Banco, esportazione, nessuna chiamata di rete, `MZLAB_LR_MISS` vuota.
- **Nomi vietati**: `tests/test_esempi_anonimi.py` resta valido (nessun nome di persona nelle liste).

## 8. Decisioni
- **Q-RL1** (deciso): fuori dal sito PlastChem, WHO_PEST_HAZARD, WHO_MIA, EDLISTS e HHP_PAN (§4); l'istantanea completa resta per l'utente.
- **Q-RL2** (deciso): fuso in RG-03/RG-05, coordinato con #1053/#1054/#1058.
- **Q-RL3**: le classi dei colpi di massa (A-D, X) si portano tali e quali da `screen.py`; i criteri esatti si rileggono dal sorgente all'inizio del codice di RG-03.
