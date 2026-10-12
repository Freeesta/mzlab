# Analizzatori animati della schermata di caricamento

- Tre ioni disegnati sempre allo stesso modo (punto con scia) in uno degli analizzatori, scelto a caso; ognuno in un file `an-*.js` che registra `window.LDAN.<nome>(ctx)` → `{n, dt, T?, at(j, t), draw?(t), mid?}` (`an-util.js`: aiuti). Un file nuovo si aggiunge con la sua riga `<script>` in `index.html` (una riga per file, in testa) e una riga qui; i test lo trovano da soli.
- `quad`: 4 barre, RF/DC, oscillazione secolare di Mathieu.
- `paul`: trappola di Paul 3D, anello r²=r0²+2z² e calotte z²=z0²+r²/2 (z0=r0/√2), ω_z=2ω_r, micromoto a frequenza RF; forma d'onda RF in un riquadro.
- `tof`: reflectron, stessa energia, stessa profondità nello specchio, tempo ∝ √(m/z).
- `settore`: settore magnetico, raggio r=√(2mU/q)/B: ioni di massa diversa curvano su raggi diversi e solo uno passa la fenditura del collettore.
- `orbi`: fuso e barile dalle equipotenziali esatte del potenziale quadro-logaritmico con la W di Lambert (`radiusAt`/`zAt`); ω_z ∝ 1/√m, ω_φ e ω_r di Makarov; test in `tests/test_analizzatori.py`.
- `esi`: cono di Taylor, gocce che si restringono.
- `trappola`: trappola ionica lineare. `fticr`: cella FT-ICR, ω_c ∝ z/m. `qqq`: triplo quadrupolo (Q1, q2 con gas, Q3).
- `ldIon(on, mode)` in `index.html` crea gli analizzatori da `window.LDAN`, sceglie `mode` o uno a caso e disegna punto con scia; `q.s` scala la dimensione di un ione (gocce, frammenti).
- Orbitrap: r1=6, r2=15, Rm=17 (unità arbitrarie). Con Rm² = (r2²−r1²)/ln(r2²/r1²) i due elettrodi avrebbero tensione zero (`rmEqualPotential`): serve Rm maggiore; con Rm > r2 il barile esterno si chiude verso le punte, con Rm < r2 si apre (ramo W₋₁).
