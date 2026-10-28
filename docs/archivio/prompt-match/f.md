# MATCH-F — Catalogo `librerie/2`: verifica di URL e licenze, preset

- **Ramo**: `claude/match-catalogo`. **Dipende da**: nessuno (il campo `kind`/`tech` di MATCH-A si usa se già unita). **In parallelo con**: MATCH-A.

## Regole comuni
- Studio di riferimento: `docs/agenti/match-spettrale.md` (leggi il § indicato sotto, non tutto); stato del codice delle librerie: `docs/agenti/librerie.md`. Non rileggere AGENTS.md.
- Logica in Rust (`crates/`), JS solo colla; file ≤ 500 righe; nessuna dipendenza nuova (se serve, PR aperta con la proposta: decide Federico tramite il Coordinatore); nessun dato reale nel repository pubblico (test su dati sintetici o `tests/golden/librerie/`).
- Flow e Tabui: solo inglese, solo amministratore, nessuna chiave `lang/`. Vista Dati: ogni testo nasce in `lang/it.js` e `lang/en.js`.
- Regole dei testi di Federico: pochi testi, nessun paragrafo dove l'interfaccia è chiara; tooltip solo per dire cosa fa un campo o cosa inserire, mai opinioni, avvisi o cautele (segnalazioni = badge o colonne neutre); errori = cosa è successo + cosa fare, in una riga; MS¹, MS², MSⁿ con apice; tempo «14.32 min» senza «RT»; il tipo di esperimento è preselezionato ma modificabile.
- WCAG 2.2 AA: bersagli ≥ 24 px (40 al tocco), tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`; ogni grafico nuovo con la sua tabella dei dati.
- Principio 1: a risoluzione unitaria nessun livello, nessuna ΔFormula, nessun badge che suggerisca un'identità.
- Una PR ≤ ~600 righe, ramo indicato sotto da `origin/main`; `python3 tools/pre_pr.py` prima del push; documenti: riscrivi le righe che descrivono il comportamento cambiato in `docs/agenti/librerie.md` o nel file d'area indicato (stato attuale, niente date né numeri di PR).
- Primo messaggio al Coordinatore (send_message, session_01NawSmneBkv8sosEHpoC8k4): «MATCH-F: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`. Sopra 200k token: blocco di consegna (≤ 15 righe) e stop.

## 1. Ruolo
DEV catalogo e download. Leggi `docs/agenti/alta-risoluzione.md` (riga «Cartella delle librerie»), `tools/librerie_pacchetti.py`, `mzlab/web/libreria-scarica.js`, `crates/mzlab-desktop/src/scarica.rs`. Studio: §11.

## 2. Contesto
Il catalogo `librerie/1` (`site/static/librerie/catalogo.json`, generato dalla CI di Pages) ha `librerie` (pacchetti ridistribuiti dal sito a pezzi da 4 MB con SHA-256: oggi MassBank 2026.03 a licenze libere) ed `esterne` (GNPS, MassBank MSP: solo app desktop). La pagina scarica solo dal sito (CSP `connect-src 'self'`). IDENT-LIBRERIE-UI ha verificato CSP/CORS per il download nel browser: chiedi l'esito al Coordinatore prima di toccare il modo `external`. Il catalogo proposto (da Gemini) NON è verificato.

## 3. Perimetro
`tools/librerie_pacchetti.py`, nuovo `tools/librerie_catalogo.json` (elenco scritto a mano), `libreria-scarica.js` e `scarica.rs` solo per leggere `librerie/2` (con `librerie/1` ancora accettato), `crates/mzlab-lib/data/preset-1.json` (nuovo) + lettore, test `tests/test_libreria_scarica.py` e `crates/mzlab-desktop/tests/scarica.rs`, `pages.yml` solo se cambia la chiave della cache. Il menu del pannello lo disegna IDENT-LIBRERIE-UI: tu dai i dati.

## 4. Lavoro
1. Verifica con fetch reale (HEAD/GET, pagina della licenza) ogni candidata: MassBank (MSP e Zenodo), MS-DIAL MSP (LC pos/neg, GC-EI con RI), GNPS ALL_GNPS_NO_PROPOGATED, MoNA (GC-MS, LC-MS/MS Orbitrap, negativo), NIST DIMSpec PFAS, MultiMS2 (Zenodo 10.5281/zenodo.17250693), EMBL-MCF 2.0, LipidBlast 2022, NIST Glycan; solo link: MSnLib, GMD Golm, ENFSI/SWGDRUG, PeptideAtlas, ProteomeTools, ISDB/LOTUS.
   Per ognuna: URL del file e della pagina, licenza SPDX, formato, dimensione, SHA-256 se il file è fisso, DOI, `kind`, `tech`, polarità, data `verificato`. Una voce non verificabile resta fuori (scrivilo nella PR).
2. `librerie/2`: campi dello studio §11 e `modo` site|external|link; `site` solo per licenze che permettono la ridistribuzione (CC0, CC BY, CC BY-SA, con attribuzione nel pacchetto), il resto `external` (solo desktop) o `link`.
3. Preset `preset-1.json` (experimental, environmental, metabolomics, natural_products, forensic) per id di catalogo e `kind`, con lettore Rust che dà la maschera `u16` sulle librerie aperte.
4. Menu base nell'ordine: MassBank, MS-DIAL, GNPS/MoNA; gruppo «Specialised / in silico» separato. Testi solo nel catalogo (nome e licenza); nessuna frase d'avviso.

## 5. Accettazione
`python3 -m pytest tests/test_libreria_scarica.py -q` e `cargo test -p mzlab-desktop scarica` verdi; `python3 tools/librerie_pacchetti.py --check-catalog` (nuovo: schema, id unici, `site` solo con licenza libera, `verificato` presente) esce 0.

## 6. Prove
I due comandi sopra; nella PR la tabella di verifica (voce, URL, licenza, dimensione, esito).

## 7. Consegna
PR «Libraries catalog v2: verified sources, licences and presets».
