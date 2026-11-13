---
paths:
  - "mzlab/web/**"
  - "tests_e2e/**"
  - "TP_Mine/**"
---
# Interface rules
- Classic scripts in `index.html` order, ES modules for new files; texts only in `lang/it.js` and `lang/en.js` (`tools/controlla_i18n.py` enforces it). Tabui (`TP_Mine/`) is English-only, no `lang/` keys, never the bare word «formula».
- Start-up scripts have a weight cap (`tests/test_peso_avvio.py`): new JS is lazy via `carica()`.
- **Front end not tried in a browser is broken.** Every interface change keeps the guided tour (`tour.js`, `tests_e2e/e2e_tour.py`) valid.
- Accessibility (WCAG 2.2 AA): colours only from CSS variables, `prefers-reduced-motion`, keyboard and `aria-label`, colour-blind-safe (Okabe-Ito) charts, targets at least 24x24 px; every new chart ships with its data table (`TDATI`).
- Tooltips: one short sentence (≤ 90 characters) saying what the control does; no explanatory text on screen.
