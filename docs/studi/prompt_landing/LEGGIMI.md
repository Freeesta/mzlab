# Build prompts: landing on the work area

Study: `docs/studi/LANDING_AREA_LAVORO.md` (option C). Start only after Federico's yes to decisions 1, 2, 5 (L1, L2) and 3, 6 (L3) of §8 of the study.

| Wave | Prompt | Branch | Zone (owned files) | Depends on |
|---|---|---|---|---|
| 1 | L1 landing shell | `claude/landing-shell` | `index.html` (`#start`, `#v-data`, `#v-workspace`), `applyView` in `shell.js`, `app.css` start rules, `first-paint.js`, `syncWorkEmpty` in `lista-file.js`, `lang/*/load.js` | — |
| 1 | L2 landing measure | `claude/landing-measure` | new `tests_e2e/e2e_landing.py` only | — |
| 2 | L3 add files in place | `claude/landing-add-in-place` | `eventi.js` (`#addf`), `shell.js` (`S.adding`, `renderLoaded`, `leaveAdding`, `#cancadd`), `stato.js` (`aggiunta`), file row menu (`explore.js` `bindFileRows` menu only), reset dialog (`QQRESET`) | L1 merged |
| 2 | L4 restore, desktop, tour | `claude/landing-restore` | `avvio-progressivo.js`, restore path of `init()` (`shell.js` lines of `init` only), `tour.js`, desktop shim open-with path (read, fix only if needed) | L1 merged |
| 3 | L5 small screens, axe, docs | `claude/landing-finish` | `barra.js` sheet bits, `app.css` < 900 px rules of the card, new e2e at 820 px, axe states, `docs/agenti/` sweep | L3, L4 merged |

L3 and L4 both touch `shell.js`, in disjoint functions (L3: adding; L4: `init`): the second to merge runs `tools/fondi_main.py` only if GitHub reports a conflict.
