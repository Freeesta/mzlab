# Disegno: «Ioni attesi»
Il nucleo è in CLAUDE.md; il resto del Disegno in `disegno.md`, il menu del tasto destro in `disegno-menu.md`.

## Ioni attesi
**Ioni attesi** (`#ions-card`, `disegno-ioni.js`, modulo ES caricato da `draw.js` al clic di «Mostra gli ioni attesi» o della riga `expected-ions` del menu della tela; atomi da `TPDraw.structures()`: pezzi della selezione o della tela, senza controioni monoatomici): tabella per struttura a tre colonne (ione e formula, m/z e finestra, «XIC»), HTML con `h```. Clic: disegno → «Mostra gli ioni attesi» → «XIC» = 3. Prova `e2e_disegno_ioni`.
Con un file HR nella scheda Dati: decimali e ppm del profilo (`HR.prof`, impostazioni dell'utente); «XIC» apre Dati con `xicDirect` + `HR.ionTrace` (etichetta «[M+H]+ formula · m/z ± ppm»). Senza file HR il titolo è «Ioni attesi (teoria)», 4 decimali, pulsanti XIC spenti con il motivo nel `title`.
Motore `ions/1` (`chem/ions.rs`; `ions` di `grafo`, `feat/grafo` `kind: "ions"`, `mzlab-cli mol-ions REQUEST|- [--set decimals=N] [--set ppm=X]`; parità `test_feat_grafo_wasm.py`, desktop `tests/labels.rs`; 17 goldens): atomi come `etich/1`, addotti solo da `adduct_table::catalog`; per riga `mz`, `mzText`, `lo`/`hi` a ±ppm, `odd`.
Regole: una struttura carica dà solo sé stessa; [M]± non vale per una neutra; manca la riga che toglie ciò che non c'è (CCl4 [M−H]−); prima l'H naturale, poi D (CD3COOD: [M−D]−); 2+ con ≥2 N, 2− con ≥4 O, entrambi ≥150 Da. Ogni riga è un candidato, mai un'identificazione.

