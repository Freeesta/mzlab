# Build prompts: landing on the work area, option D

Study: `docs/studi/LANDING_AREA_LAVORO_D.md` (Federico's design: work area from the first paint, drop zone in the centre, no «Load data», files open at once into the side panel, sample table until the first click). Start wave 1 after Federico's answers to §8 of the study (D1 needs answer 1 and 5; D3 answers 2 and 3; D5 answer 4); a «no» changes only the line of the prompt that cites it.

Old prompts `docs/studi/prompt_landing/`: L1 and L3 are superseded (by D1 and D5); L2 is reused and extended in D2; L4 is reused in D4; L5 in D6. Do not run the L prompts.

| Wave | Prompt | Branch | Zone (owned files) | Depends on |
|---|---|---|---|---|
| 1 | D1 landing shell | `claude/landing-d-shell` | `index.html`, `applyView` in `shell.js`, `app.css` landing rules, `first-paint.js`, `barra.js` (open at start, width freeze), `lang/*/load.js` | — |
| 1 | D2 landing measure | `claude/landing-d-measure` | new `tests_e2e/e2e_landing.py` only | — |
| 2 | D3 open at once, sample table | `claude/landing-d-open` | `shell.js` (all but `applyView` and the adding leftovers), `explore.js` `bootSession0`, `ruoli-carica.js`, `lista-file.js`, `tests_e2e/lib.py`, D3 test list | D1 |
| 2 | D4 restore, desktop, tour | `claude/landing-d-restore` | `avvio-progressivo.js`, `tour.js`, `tour-dopo.js`, `predefinito.js` (read), D4 test list | D1 |
| 3 | D5 add and remove | `claude/landing-d-addremove` | `eventi.js` `#addf`, `shell.js` adding leftovers, `stato.js`, `lista-file-menu.js`, `fileRemoval` in `lista-file.js`, `reset.js`, D5 test list | D3 |
| 3 | D6 small screens, axe, docs | `claude/landing-d-finish` | `barra.js` sheet bits, `app.css` < 900 px, new `tests_e2e/e2e_landing_stretto.py`, axe steps, `docs/agenti/` sweep (not the lines D5 writes) | D3, D4 |

In wave 3 D5 owns `lista-file.js` (`fileRemoval` only) and D6 does not touch it; `docs/agenti/dati-barra.md` lines on adding and removing are D5's, every other `docs/agenti/` line is D6's.

Common rules (every prompt): AGENTS.md is in context; English for code, comments, commits, PR; every new user text is a key in `lang/it` and `lang/en`; JS only for the interface, no computation (Rust); no new dependencies; wasm caps never rise; no `.github/workflows`; Teoria and Pratica untouched; colours only from CSS variables, `prefers-reduced-motion`, keyboard and `aria-label`; `python3 tools/pre_pr.py` before pushing; PR ready for review; at green CI ONE message to the Coordinator (session_01NawSmneBkv8sosEHpoC8k4, ≤ 5 lines: PR, 40-char SHA, what remains); PR events: skill `steward`; above 200k tokens at the end, hand-over block and stop.
