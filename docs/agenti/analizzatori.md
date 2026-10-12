# Analizzatori animati della schermata di caricamento

- tre ioni disegnati sempre allo stesso modo (punto con scia) in uno di sette analizzatori scelto a caso, ognuno in un file `an-*.js` che registra `window.LDAN.<nome>(ctx)` → `{n, dt, T?, at(j, t), draw?(t), mid?}` (`an-util.js`: aiuti):
  quadrupolo (4 barre, RF/DC, oscillazione secolare di Mathieu), TOF con reflectron (stessa energia, stessa profondità nello specchio, tempo ∝ √(m/z)),
  Orbitrap (`an-orbi.js`: fuso e barile dalle equipotenziali esatte del potenziale quadro-logaritmico con la W di Lambert, `radiusAt`/`zAt`; Rm > r2; ω_z ∝ 1/√m, ω_φ e ω_r di Makarov; test in `tests/test_analizzatori.py`),
  sorgente ESI (cono di Taylor, gocce che si restringono), trappola ionica lineare, cella FT-ICR (ω_c ∝ z/m), triplo quadrupolo (Q1, q2 con gas, Q3).
- `ldIon(on, mode)` in `index.html` crea gli analizzatori da `window.LDAN`, sceglie `mode` o uno a caso e disegna punto con scia; `q.s` scala la dimensione di un ione (gocce, frammenti).
- Orbitrap: r1=6, r2=15, Rm=17 (unità arbitrarie). Con Rm² = (r2²−r1²)/ln(r2²/r1²) i due elettrodi avrebbero tensione zero (`rmEqualPotential`): serve Rm maggiore; con Rm > r2 il barile esterno si chiude verso le punte, con Rm < r2 si apre (ramo W₋₁).
