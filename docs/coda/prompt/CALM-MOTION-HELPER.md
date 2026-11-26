Read docs/coda/BRIEF.md.

# CALM-MOTION-HELPER: One reduced-motion check for the site, honoured by every smooth scroll

**Goal.** 19 sites ask `matchMedia("(prefers-reduced-motion: reduce)")` on their own and only a few (`avvio.js`, `transizioni.js`, `tema-morbido.js`, `logo-piccone.js`) also read the in-app `data-a11y-reduce` switch; nine `scrollIntoView` or `scrollTo` calls always scroll smoothly. One helper, used everywhere: about 10 lines go and reduced motion is respected.

**Read first.** `docs/agenti/web-modern.md` (CALM-MOTION-HELPER), `docs/agenti/accessibility.md` (reduced motion, `data-a11y-reduce`). Sites: `grep -n 'prefers-reduced-motion' mzlab/web/*.js`; smooth scrolls: `grep -n 'smooth' mzlab/web/*.js` (`banco.js`, `infusione-albero.js`, `libreria.js`, `libreria-finestra.js`, `tabs.js`, `pannelli.js`, `telefono-scheda.js`, `area-lavoro.js`, `tour.js`). Existing helpers to reuse first: `isReducedMotion` in `tour.js` and the check in `settings.js`.

**Done means.** (1) One helper (system setting or `data-a11y-reduce`) returns the `behavior` value; classic scripts and modules both reach it without a new load-order hazard (check `carica.js`). (2) Under reduced motion no scroll is animated; without it, behaviour and timing are unchanged. (3) A test per changed path: extend `e2e_a11y_dati` or `e2e_scroll` with reduced motion emulated (Playwright `reduced_motion`), then `python3 tools/verifica.py --changed`; `e2e_tour` and `e2e_pannelli_stabili` stay green. (4) PR body: `git diff --shortstat`.

**Out of scope.** CSS `scroll-behavior` on `html` (it would animate every programmatic scroll, `scrollTo(0, 0)` included), changed animations, texts.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
