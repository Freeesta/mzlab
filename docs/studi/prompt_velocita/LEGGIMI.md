# Launch prompts: reload and click speed

Study: [`../VELOCITA_RICARICA.md`](../VELOCITA_RICARICA.md). Probe: `tests_e2e/misura_ricarica.py`, summary `tools/misura_ricarica_riassunto.py`. Every package measures before and after with the same command and puts both tables in its PR. The chat title starts with «mzLab».

| Package | What | Zone (files) | After / collides with | Prompt |
|---|---|---|---|---|
| V01 | service worker cache-first by site version | `mzlab/web/sw.js`, `tools/build_site.py` (only the `sw.js` part), `tests/test_sw_cache_node.py` | none of the open work | [01](01-sw-cache-versione.md) |
| V02 | lean «Full reset» + isolation after reset | `mzlab/web/reset.js`, `isolateOnce` in `mzlab/web/browser.js`, `tests_e2e/e2e_reset.py` | V01 (same worker); **decision for Federico** | [02](02-reset-snello.md) |
| V03 | Tabui loads after the page is usable | `mzlab/web/tpmine-loader.js` | LAZY-WIFF (idle prefetch, `shell.js`, `carica.js`) | [03](03-tabui-dopo.md) |
| V04 | first visit: no lost page, one language file | `isolateOnce` in `browser.js`, `lang/it.js`, `lang/en.js`, `tools/build_site.py` | V02 (`isolateOnce`), landing D3 #2207 / D1b #2211 | [04](04-primo-arrivo.md) |
| V05 | Ketcher start-up off the click path | `mzlab/web/first-paint.js`, `mzlab/web/draw.js` | landing D3 / D1b (`first-paint.js`) | [05](05-ketcher-click.md) |

Order: V01 → V02 → V04; V03 and V05 in parallel with them once their collisions are merged. V01 alone gives most of the gain.

Rules for all:
- No product change outside the zone; the start-up cap (`tests/test_peso_avvio.py` `MAX_TOTAL` 1 900 000) and the `.wasm` caps never go up.
- Parity site / app: the service worker, `reset.js` and the isolation reload exist only in the browser (the desktop app serves `mzlab://` and never downloads `sw.js`); the PR says so in one line, no CODA entry is needed. V04's language bundle must reach the desktop too (it ships `site/static`).
- `e2e_isolamento.py`, `e2e_aggiorna.py`, `e2e_reset.py`, `e2e_avvio*.py`, `e2e_tour.py` stay green; `python3 tools/pre_pr.py` before pushing.
- Definition of done: in `main`, «pages» green, and `misura_ricarica.py` on the live site shows the expected gain (numbers in the PR).
