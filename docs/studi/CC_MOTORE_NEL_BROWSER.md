# Chimica computazionale nel browser: motore di reattività in Rust per il «Parent compound» di Tabui

Studio dell'Architetto sulla richiesta di Federico: dopo il calcolo dei TP, Tabui calcola da solo (in background, dopo i TP) la reattività del genitore e la mostra nel riquadro «Parent compound»: molecola 2D colorata, modello 3D interattivo, indici per atomo f(+)/f(0)/f(−), altri indici globali. Tutto nel browser, in Rust, senza servizi esterni. Prompt dei pacchetti: `docs/studi/PROMPT_CC_BROWSER/`. Script e numeri della misura di §3: `docs/studi/cc_browser/`.

## Sintesi per Federico
1. **Sì, si può fare, ed è più vicino di quanto pensassimo.** Qualcuno l'ha già fatto: esiste un crate Rust con licenza MIT (sci-form 0.15.2) che contiene GFN2-xTB, Hückel esteso (EHT), geometria 3D da SMILES (ETKDG) e campi di forza UFF/MMFF94, compilabile in WebAssembly.
2. Ho misurato qui: le cariche GFN2 di sci-form sulla molecola neutra coincidono con tblite (il riferimento ufficiale di Grimme) entro 1e-4 sulle 10 molecole della bozza CC-05; il `.wasm` con GFN2 + EHT + geometria pesa 1,04 MB (307 KB compresso).
3. Mancano due pezzi, che scriviamo noi: il calcolo di catione e anione (serve per le Fukui: oggi non converge in 9 casi su 10) e la solvatazione ALPB in acqua (senza, la carbamazepina cambia il sito primo).
4. Con GFN2 + ALPB (tblite) f(−) mette al primo posto il sito sperimentale in 9 casi su 10; il paracetamolo resta un pari merito, come nella tua bozza. EHT da solo fa 8 su 10 e sbaglia nitrobenzene e paracetamolo: è il ripiego, non la strada.
5. **Raccomandazione**: crate nostro `crates/mzlab-chem` che riprende (con la licenza MIT e il nome degli autori) solo i moduli necessari di sci-form, li corregge e li prova contro tblite; `.wasm` a parte `chem`, caricato solo in modalità amministratore. Viewer 3D nostro in Canvas, leggero e accessibile.
6. Lavoro stimato: circa 14-17 giorni-chat in 6 pacchetti; la prima cosa visibile (geometria + Fukui GFN2 in tabella) dopo B1 e B2, circa 7-9 giorni-chat.
7. Il programma propone e il chimico decide: tutto è «Prev.», nessun livello di Schymanski, le classi di posizioni non si toccano.
8. **Cosa serve da te**: (a) sì/no alla strada principale; (b) sul Mac, per le 20 molecole di §6, il file `.xyz` e l'uscita di `xtb --gfn 2 --alpb water --vfukui` (xtb 6.7.1) per il controllo incrociato xtb ↔ tblite; (c) per le 10 molecole nuove, il sito sperimentale con la fonte (DOI), se lo conosci.
9. Niente lascia il computer dell'utente: il calcolo gira in un worker del browser (o nell'app desktop, nativo).
10. Lo studio `CHIMICA_COMPUTAZIONALE.md` diceva «niente xtb nel browser»: valeva per xtb in Fortran; la misura di §3 cambia la risposta per GFN2 scritto in Rust.

## 1. Cosa c'è già (da non rifare)
- Grafo SMILES in Rust (`crates/mzlab-feat/src/chem/`: `smiles.rs`, `grafo.rs`, `posizioni/`), con classi di simmetria e indici degli atomi nell'ordine dello SMILES: la stessa numerazione deve valere per il nuovo motore (`docs/agenti/posizioni.md`).
- Riquadro «2 · Parent compound» di Tabui (`TP_Mine/js/20_tpmine.js`, `structure()`): disegno 2D con OpenChemLib, già nel sito per il Disegno.
- Pannello «Substitution position» e contratto `posiz/1` (CC-01, CC-02): classi di posizioni compatibili con la MS2; un campo «Prev.» era già previsto (CC-03).
- Validazione dell'assistente locale (bozza `docs/bozze/teoria_cc05.md` §2): xtb 6.7.1 GFN2, ALPB water, `--vfukui` su geometrie RDKit ETKDGv3 + MMFF94, 0,04-0,15 s per molecola; f(−) buon descrittore per •OH, f(0) sbaglia su nitrobenzene e acido benzoico, addotti σ GFN2 inutili.

## 2. Ricerca: cosa esiste (fonti e licenze)
### A. Calcolo quantistico o semiempirico nel browser
| Progetto | Linguaggio, licenza | Browser/WASM | Cosa calcola | Peso, velocità | Giudizio |
|---|---|---|---|---|---|
| sci-form 0.15.2 (github.com/jigonzalez930209/sci-form) | Rust, MIT | sì (npm `sci-form-wasm`) | GFN0/1/2-xTB, EHT, PM3, HF-3c, ANI-2x, ETKDGv2, UFF, MMFF94, cariche, Fukui da EHT, HOMO/LUMO, ALPB a sé | misurato qui: 1 041 149 B, 307 KB gzip (§3) | **base della strada principale**: un solo autore, 3 stelle, 124 000 righe, molte affermazioni non verificate; si riprendono solo i moduli provati |
| xtbloom (github.com/jinzhezenggroup/xtbloom) | C++17, GPL-3.0-or-later | sì, adattatore WASM sperimentale; demo xtbloom.jinzhezeng.group | GFN1/GFN2, energia, forze, cariche; solvatazione «Not implemented» | peso non dichiarato | no: GPL nel sito, C++, senza solvente |
| cheminfo/xtb-wasm (github.com/cheminfo/xtb-wasm) su OCC (github.com/peterspackman/occ) | C++17; OCC LGPL-3.0, xtb-wasm GPL-3.0 | sì | GFN2 a guscio chiuso, ottimizzazione, IR/Raman; OCC ha CPCM-X e SMD; GFN2 a guscio aperto «out of scope» | 21 MB `.wasm` + 8,2 MB dati; caffeina ~200 ms (M1) | no: peso, licenza, niente ioni radicali |
| tblite (github.com/tblite/tblite), xtb | Fortran, LGPL-3.0 | nessuna compilazione WASM trovata | GFN1/GFN2, ALPB, cariche | — | **riferimento dei test** (programma separato, mai nel sito) |
| dxtb (github.com/grimme-lab/dxtb; Friede et al. 2024) | Python/PyTorch, Apache-2.0 | no | GFN1/GFN2 differenziabili | — | secondo riferimento leggibile per il port e fonte dei parametri con licenza compatibile |
| MOPAC 23.2.5 (github.com/openmopac/mopac) | Fortran, Apache-2.0 | nessuna build WASM trovata | MNDO, AM1, PM3, PM6, PM7 | — | no: Fortran; PM7 non serve se c'è GFN2 |
| PySCF, Psi4 su Pyodide | Python + C | nessun pacchetto Pyodide trovato (da verificare) | DFT | decine di MB | no: Python nel sito vietato (principio 2) |
| ORCA | proprietario | no | DFT | — | no |
| xtb-model, rgpot (crates.io) | involucri Rust del codice C/Fortran | no | — | — | no: non sono port |

### B. Rowan e i visualizzatori
- **Rowan** (labs.rowansci.com; blog «Fukui Indices & Global Electrophilicity», rowansci.substack.com/p/fukui-indices-and-global-electrophilicity): f(+), f(−), f(0) condensati come differenza tra le cariche atomiche di neutro, anione e catione; indice di elettrofilicità globale «elettronegatività al quadrato diviso durezza» (Parr 1999; cita Maynard 1998), in eV; storicamente cariche e indice con xTB, poi livello di teoria a scelta (rowansci.substack.com/p/2d-scans-more-fukui-options-and-max).
- Rowan è un servizio remoto: i dati lasciano il computer. Riprendiamo solo l'idea pubblica (schede per indice, tabella per atomo con soglia, 3D, JSON), con nomi nostri, senza codice né marchio.
- **Visualizzatori 3D**: 3Dmol.js BSD (~563 KB minimizzato, dato di depscope.dev, da verificare), NGL MIT, Mol* MIT (pesante), JSmol LGPL. Tutti gestiscono il mouse, non la nostra accessibilità (tastiera, `aria-label`, TDATI, colori da variabili CSS).
- **Scelta**: viewer nostro in Canvas 2D (sfere e bastoncini ordinati per profondità, aloni proporzionali al valore, legami H tratteggiati): per 10-80 atomi basta, pesa pochi KB, ha pulsanti e tasti per ruotare e ingrandire, tabella dei dati. WebGL non serve. Ripiego se servissero superfici vere: 3Dmol.js (BSD) caricato solo in amministratore.

### C. Metodi calcolabili in Rust nel browser (in ordine di costo)
| Metodo | Accuratezza attesa sulle 10 molecole | Tempo per 30 atomi (WASM, stima) | Peso | Rischio | Lavoro | Licenza |
|---|---|---|---|---|---|---|
| (1) Hückel π / EHT, f(−) = densità HOMO | misurato: 8/10 col sito sperimentale, sbaglia nitrobenzene (para) e paracetamolo | < 50 ms | < 100 KB | basso; qualità limitata | 1 giorno (si riprende da sci-form) | MIT |
| (2) PM3/AM1/MNDO, GFN0/GFN1 | non misurato; GFN1 di sci-form dichiarato fedele a tblite (da verificare); nessuna prova su Fukui | 0,5-2 s | ~0,5 MB | medio | 4-6 giorni | MIT (sci-form) |
| (3) **GFN2-xTB in Rust** con carica, ALPB water, Fukui per differenze finite | come tblite: 9/10 + 1 pari merito (§3); parità con xtb 6.7.1 dalla bozza | 1-3 s (nativo misurato 0,03-0,5 s quando l'SCC converge) | ~1 MB | medio: convergenza degli ioni, ALPB da scrivere | 5-7 giorni | MIT (codice) + parametri da verificare (§5) |
| (4) ML pre-addestrato: AIMNet2 (MIT, cariche anche per ioni, repository archiviato, ora aimnetcentral) o random forest delle Fukui (Zhang 2016, R² 0,68) | non misurato; nessun banco su f(−) | 0,1-1 s con `tract`/`candle` | MB di pesi (da verificare) | alto: dominio, formato del modello, deprecazione | 6-10 giorni | MIT (AIMNet2), dati da verificare |

### D. Geometria 3D da SMILES in Rust
- sci-form: ETKDGv2 con 846 SMARTS di torsioni, UFF e MMFF94, ricerca di conformeri con raggruppamento; dichiara RMSD medio 0,064 Å (distanze tra atomi pesanti) contro RDKit su 131 molecole (da verificare). Misurato qui: 2-17 ms per molecola, geometria valida 10/10.
- COSMolKit (MIT, ETKDG allineato a RDKit 2026.09.1, UFF/MMFF), chematic 1.1.5 (MIT OR Apache-2.0, distance geometry + UFF semplificato): alternative se sci-form si rivelasse fragile.
- RDKit MinimalLib (BSD-3, C++ in WASM): l'embedding 3D non risulta esposto nel README (da verificare); non serve.
- OpenChemLib (già nel sito) ha un generatore di conformeri in JS: escluso, il motore resta in Rust.
- **Conformeri**: 8 conformeri ETKDG, minimizzazione MMFF94, si tiene il più basso; si calcola f(−) anche sui 3 più bassi e si mostra la dispersione (onestà: se cambia il sito primo, lo si dice). Molecole piccole e rigide (le 10 della bozza): un conformero basta.

### E. Altri indici dallo stesso motore
| Indice | Come | Limite da scrivere accanto |
|---|---|---|
| Cariche parziali | Mulliken GFN2 (le stesse delle Fukui) | dipendono dallo schema di popolazione |
| HOMO, LUMO, gap | autovalori GFN2 | GFN2 sottostima il gap; solo confronto tra molecole |
| IP e EA verticali | ΔSCF: E(N−1) − E(N), E(N) − E(N+1), con la correzione empirica che xtb applica per GFN2 (`--vipea`, da verificare il valore) | stime: errore di ~0,5 eV o più; IPEA-xTB sarebbe più adatto |
| Elettrofilicità globale ω | μ = −(IP+EA)/2, η = IP − EA, ω = μ²/2η (Parr 1999), in eV | ordina molecole simili; non è una costante di velocità |
| Duale Δf = f(+) − f(−) | dalle Fukui | qualitativo |
| BDE C–H stimata | facoltativo: E(R•) + E(H•) − E(RH) GFN2 a geometria rilassata, oppure le classi di CC-03 | da validare su una tabella di BDE sperimentali prima di mostrarla |
| pKa, siti di protonazione | no | nessun metodo affidabile a questo costo |

### F. Catalogo Rowan per i TP
Valutazione completa (valore, metodo in Rust, costo, rischio, priorità) in `CC_CATALOGO_ROWAN.md`. Priorità: 1 Fukui e conformeri; 2 HOMO/LUMO e BDE C–H; 3 tautomeri con ionizzazione qualitativa, logP/polarità per l'ordine di eluizione (solo dopo validazione su isomeri con RT misurato); 4 redox; 5 CCS (solo con dati IMS). Non pertinenti: docking, proteine, fononi, IRC/TS, superfici, solubilità, ADME-Tox, NMR.

## 3. Misura fatta in questo studio (container cloud, 16/10/2026)
Script e dati in `docs/studi/cc_browser/` (`riferimento_tblite.py`, `classi.py`, `confronto_sciform.py`, `geometria_sciform.py`, `sciform_main.rs`, `risultati_10.csv`). Riferimento: tblite 0.7.0 (PyPI), GFN2-xTB, ALPB water, cariche di Mulliken, f(−) = q(N−1) − q(N), f(+) = q(N) − q(N+1), geometria RDKit 2026.09.1 ETKDG + MMFF94. Siti: C insaturi con H, mediati sulle classi di simmetria (`CanonicalRankAtoms`).

| Molecola | Sito sperimentale (bozza §2) | f(−) tblite ALPB: primo | f(0) tblite: primo | f(−) EHT sci-form: primo | f(−) tblite su geometria sci-form: primo |
|---|---|---|---|---|---|
| fenolo | orto/para | para 0,081 | orto | para | para |
| anisolo | orto/para | para 0,067 | orto | para | para |
| toluene | orto/para | para 0,072 | para | para | para |
| acido benzoico | meta/orto | meta 0,044 | **para** | orto = meta | meta |
| nitrobenzene | meta | meta 0,043 | **para** | **para** | meta |
| clorobenzene | orto/para | para 0,071 | para | para | para |
| anilina | orto/para | para 0,070 | orto | para | para |
| naftalene | α | α 0,054 | α | α | α |
| carbamazepina | C10=C11 | C10=C11 0,030 | C10=C11 | C10=C11 | C10=C11 |
| paracetamolo | orto a OH | orto a OH 0,033 = orto a NHAc 0,032 (pari merito) | orto a NHAc | **orto a NHAc** | orto a OH |

- tblite riproduce i valori xtb 6.7.1 della bozza alla terza cifra (per esempio fenolo para 0,081, nitrobenzene meta 0,043): tblite può fare da riferimento nei test.
- In fase gassosa la carbamazepina mette due C aromatici sopra C10=C11: **ALPB serve**.
- Cariche GFN2 neutre di sci-form contro tblite (stessa geometria, fase gassosa): differenza massima < 1e-4 (carica elementare) su tutte e 10. Tempo nativo di 3 SCC: 28-650 ms; carbamazepina 1,4 s con SCC non convergenti (250 iterazioni).
- Con una correzione minima (numero di elettroni e cariche iniziali dalla carica totale) catione e anione convergono solo nel nitrobenzene, dove f(−) coincide con tblite entro 7e-4. Il formalismo è giusto; l'SCC degli ioni va reso robusto (B2).
- Geometria sci-form senza minimizzazione MMFF: stesso sito primo 10/10; l'ordine completo cambia in 2 casi su posizioni pari merito (scarto < 0,005); i valori calano fino a 0,012 (acido benzoico): la minimizzazione MMFF va fatta (B1).
- `.wasm` di prova (`opt-level = "z"`, LTO, senza `wasm-opt`) con GFN2 + EHT + embedding: 1 041 149 B, 307 KB gzip. Il tetto comune dei `.wasm` di mzFinder (512 000 B) non basta: modulo `chem` con tetto proprio.

## 4. Raccomandazione
**Strada principale**: GFN2-xTB in Rust (`crates/mzlab-chem`), ripreso dai moduli MIT di sci-form (`xtb/gfn2.rs`, `gfn2_params.rs`, `d4*.rs`, `broyden.rs`, `sto_overlap.rs`, `distgeom/`, `etkdg.rs`, `forcefield/`) e completato da noi: carica totale, SCC robusto per N±1 (partenza dalla densità neutra, smorzamento, temperatura elettronica di 300 K come xtb), ALPB water accoppiato all'SCC (Ehlert et al. 2021), Fukui condensate, indici di §2E. Geometria: ETKDG + MMFF94 dallo stesso crate. Prova di parità contro tblite su file di riferimento.

**Ripiego** (se B2 non passa i criteri di §7 entro il suo tempo): f(−) da EHT nello stesso crate, mostrato con l'avviso «EHT, validated 8/10» e senza f(0); il pannello e il viewer restano uguali. Secondo ripiego, solo per la geometria: COSMolKit o chematic.

**No**: xtb/tblite/OCC/xtbloom compilati in WASM (Fortran o C++, GPL/LGPL, 21 MB); Pyodide; ML (nessun banco su f(−), pesi e formato incerti); servizi remoti; superfici di isodensità (servono griglie e marching cubes: dopo, se Federico le chiede).

## 5. Rischi, licenze e regole
- **Qualità del codice ripreso**: sci-form ha un solo autore attivo e affermazioni non verificate. Regola: si copia solo ciò che passa una prova di parità contro tblite nei nostri test; ogni file ripreso porta in testa la provenienza (versione, commit) e la nota MIT; `crates/mzlab-chem/LICENSE-sci-form` con il testo della licenza; riga nello SBOM se `docs/agenti/rilascio.md` lo prevede. Niente dipendenza da crates.io: il crate porta 124 000 righe e `getrandom` per il bersaglio WASM.
- **Parametri GFN2 e ALPB**: sci-form li dichiara presi dall'articolo (Bannwarth et al. 2019). I file TOML di tblite/xtb sono LGPL-3.0; dxtb (Apache-2.0) li contiene con licenza compatibile. Regola: si confrontano i numeri di sci-form con quelli di dxtb e dell'articolo; se una tabella esiste solo nei file LGPL, la domanda va a Federico (da verificare). Lo stesso per i parametri ALPB dell'acqua (informazioni di supporto di Ehlert et al. 2021).
- **Scienza**: f(−) ordina i siti di un attacco elettrofilo su una molecola isolata in acqua implicita; non vede la superficie di TiO2, l'ingombro sterico né lo stato di transizione (bozza CC-05 §1, Barrera e Anderson 2024 e 2025). Si chiama sempre «Prev.», non riordina e non toglie classi di `posiz/1`, nessun livello di Schymanski.
- **f(0) e f(+)**: si mostrano (come chiede Federico) ma la scheda predefinita è f(−) e una nota fissa dice che per •OH f(0) sbaglia sugli anelli disattivati (bozza §2.2).
- **Visibilità**: solo con `uipAdminOn()`, in Tabui e Attributio; niente nella vista Dati, nel Disegno degli studenti, in Teoria o nella relazione.
- **Dati**: tutto in un worker del browser o nel motore nativo; nessuna rete; i risultati restano nella sessione (ed eventualmente in una cache locale con chiave SMILES + versione del motore). Nei test solo composti pubblici; nessun cognome di `nomi_vietati.txt`.
- **Prestazioni**: il calcolo non deve rallentare i TP: parte dopo, in un worker separato, si ferma se l'utente rilancia i TP o cambia genitore.
- **Elementi**: GFN2 copre H-Rn; il viewer e i test coprono H, C, N, O, F, P, S, Cl, Br, I; un elemento non coperto dà `error.chem.element`.

## 6. Insieme di prova (composti pubblici)
- I 10 della bozza: fenolo, anisolo, toluene, acido benzoico, nitrobenzene, clorobenzene, anilina, naftalene, carbamazepina, paracetamolo (sito sperimentale dalla bozza §2.1).
- 10 inquinanti tipici dei TP (parità numerica sempre; sito sperimentale solo se Federico o la letteratura lo danno con DOI): diclofenac, sulfametossazolo, atrazina, bisfenolo A, caffeina, triclosan, ibuprofene, metoprololo, benzofenone-3, imidacloprid.
- Riferimenti nel repository: `crates/mzlab-chem/tests/data/cc_ref.json` (geometria, cariche q(N), q(N±1) e f per atomo, tblite 0.7.0 ALPB water e fase gassosa), prodotto da `tools/cc_riferimento.py` (Python solo come strumento). Le uscite xtb 6.7.1 di Federico (`*.xyz` + `--vfukui`) entrano in `tests/data/xtb/` come secondo controllo.

## 7. Criteri di successo (misurabili)
1. **Parità** (stessa geometria, tutte e 20 le molecole): SCC convergente per N, N+1, N−1 nel 100 % dei casi; |Δq| ≤ 2e-3 e e |Δf| ≤ 3e-3 per atomo contro tblite, in ALPB water e in fase gassosa.
2. **Sito**: sulle 10 della bozza, la classe prima di f(−) coincide con quella di tblite in 10/10 (pari merito se scarto < 0,005) e con il sito sperimentale in ≥ 9/10.
3. **Geometria**: con ETKDG + MMFF94 nostri, classe prima di f(−) uguale a quella su geometria RDKit in 10/10; RMSD mediano degli atomi pesanti dopo allineamento ≤ 0,3 Å contro RDKit ETKDGv3 + MMFF94.
4. **Tempo** (Chromium, CI, un worker): geometria + 3 SCC con ALPB ≤ 5 s per la carbamazepina (30 atomi) e ≤ 15 s per 60 atomi; nativo (desktop) ≤ 1 s per 30 atomi.
5. **Peso**: `.wasm` `chem` ≤ 1 300 000 B dopo `wasm-opt -Oz`; tetto proprio = 1,3 × misura; nessun peso aggiunto per chi non è amministratore (caricamento solo su richiesta).
6. **Pagina**: nessun aumento del tempo dei TP (stessa misura con e senza il calcolo); `window.MZLAB_LR_MISS` vuoto; e2e e axe verdi; senza `qqq.admin` niente pannello e nessun download del `.wasm`.

## 8. Architettura
- **Crate** `crates/mzlab-chem` (nuovo, feature `std`, unica dipendenza nuova ammessa: `nalgebra`, Rust puro, Apache-2.0, già usata dal codice di sci-form; non è ancora nel workspace): `mol3d` (ingresso da `mzlab_feat::chem::grafo::Mol`: atomi pesanti nell'ordine dello SMILES, poi gli H; uscita XYZ), `geom` (ETKDG, MMFF94, conformeri), `xtb` (GFN2, ALPB, Fukui), `eht` (ripiego), `indici` (§2E). Funzione unica `reactivity_json(smiles, opts) -> Result<String>` con contratto `reatt/1`.
- **Contratto `reatt/1`**: `{format:"reatt", version:1, engine:"mzlab-chem/N", method:"GFN2-xTB", solvent:"water|none", smiles, atoms:[{i, el, heavy_index|null, x, y, z, q, f_plus, f_zero, f_minus, dual}], bonds:[{a, b, order}], hbonds:[{donor_h, acceptor}], global:{homo_ev, lumo_ev, gap_ev, ip_ev, ea_ev, mu_ev, eta_ev, omega_ev}, conformers:{n, chosen, f_minus_spread}, scc:{converged, iterations}, notes, cite}`.
- `heavy_index` = indice di `posiz/1`; `scc.converged` = tre valori (N, N+1, N−1); `notes` e `cite` = elenchi di testi e DOI.
- **WASM**: `crates/mzlab-chem-wasm` → `wasm/chem/` in `tools/build_feat_wasm.sh` con tetto proprio; manifesto come `grafo`; caricato solo con `uipAdminOn()` e un genitore con SMILES. App desktop: stessa funzione nativa dietro una rotta come `feat/grafo` (`kind:"reactivity"`).
- **Pagina**: `TP_Mine/js/46_reattivita.js` (nuovo, modulo con `// @ts-check`): pannello nel riquadro «Parent compound» con schede «f(−)», «f(0)», «f(+)», «Info», «Notes», «JSON»; disegno 2D OpenChemLib con aloni; `TP_Mine/js/47_viewer3d.js` (Canvas). Worker `TP_Mine/files/chem-worker.js` a bassa priorità.
- **Con `posiz/1`**: campo facoltativo `prior:[{atom, value, rank, quantity:"f_minus", method, note:"Prev."}]` (resta `version:1`), colonna «Prev. f(−)» nel pannello «Substitution position».

## 9. Piano a fasi
| Pacchetto | Cosa | Dipende da | Parallelo con | Stima (giorni-chat) |
|---|---|---|---|---|
| CC-B1 | crate `mzlab-chem`: SMILES → 3D (ETKDG + MMFF94, conformeri), XYZ, strumento `tools/cc_riferimento.py` e dati di riferimento | — | B3 (con dati finti) | 2,5-3 |
| CC-B2 | GFN2 con carica, SCC robusto, ALPB, Fukui, ripiego EHT, `reatt/1`, `.wasm` `chem` | B1 | B3 | 5-6 |
| CC-B3 | viewer 3D Canvas + pannello «Parent compound» (2D con aloni, schede, soglia, tabella, esportazioni) | contratto `reatt/1` (finto fino a B2) | B1, B2 | 3-3,5 | Stato: `47_viewer3d.js`, `46_reattivita.js` e `e2e_reattivita.py` fatti con un `reatt/1` di prova; il riquadro «Parent compound» ospita il pannello (stato «waiting for TPs»); il calcolo vero lo avvia CC-B4.
| CC-B4 | avvio automatico dopo i TP in worker a bassa priorità, stato, annullamento, desktop | B2 + B3 | B5 | 1-1,5 |
| CC-B5 | `prior` in `posiz/1` e colonna «Prev. f(−)»; Attributio legge `reatt/1` | B2 (+ B3 per la pagina) | B4 | 1-1,5 |
| CC-B6 (facoltativo) | IP/EA, ω, HOMO/LUMO nella scheda «Info»; BDE C–H solo se validata | B2 | B4, B5 | 1,5-2 |
| CC-B7 (facoltativo) | tautomeri per regole sul grafo, ordinati con GFN2 + ALPB; gruppi ionizzabili e carica attesa a pH ~2,7 (qualitativi) | B2 | B6 | 2-3 |
| CC-B8 (facoltativo) | Crippen logP, dipolo, legame H intramolecolare per l'ordine di eluizione degli isomeri; visibile solo se ≥ 8/10 coppie di isomeri con RT misurato sono in ordine giusto | B2 + dati di Federico | B6, B7 | ~2 |

Ordine: B1 → B2 → (B4, B5, B6 in parallelo); B3 parte subito con un `reatt/1` di prova scritto a mano. Chi unisce per secondo fa `git merge origin/main` e riprova. Rapporto con lo studio precedente: B2 + B5 sostituiscono la parte •OH di CC-03 (Hückel); CC-04 (importare calcoli esterni) resta utile come controllo; l'alogenazione di CC-03 resta com'era.

## 10. Bibliografia (DOI controllati su Crossref il 16/10/2026)
- Bannwarth C., Ehlert S., Grimme S. (2019) GFN2-xTB. J. Chem. Theory Comput. doi:10.1021/acs.jctc.8b01176
- Grimme S., Bannwarth C., Shushkov P. (2017) GFN-xTB. J. Chem. Theory Comput. doi:10.1021/acs.jctc.7b00118
- Ehlert S. et al. (2021) Robust and efficient implicit solvation model for fast semiempirical methods (ALPB). J. Chem. Theory Comput. doi:10.1021/acs.jctc.1c00471
- Caldeweyher E. et al. (2019) D4. J. Chem. Phys. doi:10.1063/1.5090222
- Friede M. et al. (2024) dxtb. J. Chem. Phys. doi:10.1063/5.0216715
- Yang W., Mortier W. J. (1986) Fukui condensate. J. Am. Chem. Soc. doi:10.1021/ja00279a008
- Parr R. G., Szentpály L. v., Liu S. (1999) Electrophilicity index. J. Am. Chem. Soc. doi:10.1021/ja983494x
- Maynard A. T. et al. (1998) PNAS (citato da Rowan per ω). doi:10.1073/pnas.95.20.11578
- Hoffmann R. (1963) An extended Hückel theory. J. Chem. Phys. doi:10.1063/1.1734456
- Riniker S., Landrum G. A. (2015) ETKDG. J. Chem. Inf. Model. doi:10.1021/acs.jcim.5b00654
- Wang S. et al. (2020) ETKDGv3. J. Chem. Inf. Model. doi:10.1021/acs.jcim.0c00025
- Halgren T. A. (1996) MMFF94 I. J. Comput. Chem. doi:10.1002/(SICI)1096-987X(199604)17:5/6<490::AID-JCC1>3.0.CO;2-P
- Rappé A. K. et al. (1992) UFF. J. Am. Chem. Soc. doi:10.1021/ja00051a040
- Anstine D., Zubatyuk R., Isayev O. (2025) AIMNet2. Chem. Sci. doi:10.1039/D4SC08572H
- Zhang Q. et al. (2016) Machine learning estimation of atom condensed Fukui functions. Mol. Inf. doi:10.1002/minf.201500113
- Barrera Y., Anderson J. S. M. (2024) doi:10.1002/jcc.27314; (2025) doi:10.1002/jcc.70130
- Tentscher P. R., Lee M., von Gunten U. (2019) Acc. Chem. Res. doi:10.1021/acs.accounts.8b00610
- Löffler P. et al. (2025) In silico frontiers… («Prev.»). Environ. Sci. Technol. doi:10.1021/acs.est.5c06790
- Software (URL, licenza letta sul repository il 16/10/2026): sci-form github.com/jigonzalez930209/sci-form (MIT); tblite github.com/tblite/tblite (LGPL-3.0); dxtb github.com/grimme-lab/dxtb (Apache-2.0); xtbloom github.com/jinzhezenggroup/xtbloom (GPL-3.0-or-later); OCC github.com/peterspackman/occ (LGPL-3.0); cheminfo/xtb-wasm (GPL-3.0); MOPAC github.com/openmopac/mopac (Apache-2.0); AIMNet2 github.com/isayevlab/AIMNet2 (MIT, archiviato); COSMolKit github.com/cosmol-studio/COSMolKit (MIT); chematic docs.rs/chematic (MIT OR Apache-2.0); RDKit.js github.com/rdkit/rdkit-js (BSD-3).
- Visualizzatori (licenza letta sul repository): 3Dmol.js github.com/3dmol/3Dmol.js (BSD), NGL github.com/nglviewer/ngl (MIT), Mol* github.com/molstar/molstar (MIT).
