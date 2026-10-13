# Analizzatori animati della schermata di caricamento

- **Sviluppi futuri, spenti di default**: durante il caricamento compare solo l'animazione base (`base` in `ldIon`: tre ioni sfasati di 120° le cui scie avanzano e si intrecciano, punto con scia). Le schermate degli analizzatori restano nel codice ma non sono incluse in `index.html`, non si scaricano e non c'è alcuna voce nell'interfaccia. Per riattivarle: `LD_ANALYSERS = true` in `index.html` (le carica a richiesta da `LD_AN` e ne sceglie una a caso).
- **Testo del caricamento**: durante l'apertura dei file compaiono solo le frasi a rotazione (`loading(true)` senza messaggio). Aggancio per fase e %: `loadingStato(testo, pct)` in `shell.js` (`pct` nullo = barra indeterminata), `loadingStato(null)` torna alle frasi.
- Ogni analizzatore è in un file `an-*.js` che registra `window.LDAN.<nome>(ctx)` → `{n, dt, T?, at(j, t), draw?(t), mid?}` (`an-util.js`: aiuti). Un file nuovo si aggiunge al nome in `LD_AN` (`index.html`) e con una riga qui; i test pytest lo trovano da soli.
- `quad`: 4 barre, RF/DC, oscillazione secolare di Mathieu.
- `paul`: trappola di Paul 3D, anello r²=r0²+2z² e calotte z²=z0²+r²/2 (z0=r0/√2), ω_z=2ω_r, micromoto a frequenza RF; forma d'onda RF in un riquadro.
- `tof`: reflectron, stessa energia, stessa profondità nello specchio, tempo ∝ √(m/z).
- `settore`: settore magnetico, raggio r=√(2mU/q)/B: ioni di massa diversa curvano su raggi diversi e solo uno passa la fenditura del collettore.
- `orbi`: fuso e barile dalle equipotenziali esatte del potenziale quadro-logaritmico con la W di Lambert (`radiusAt`/`zAt`); ω_z ∝ 1/√m, ω_φ e ω_r di Makarov; test in `tests/test_analizzatori.py`.
- `ims`: mobilità ionica a tubo di deriva, v=K·E con K ∝ 1/CCS: gli ioni più compatti arrivano prima, urti con il gas tampone.
- `esi`: cono di Taylor, gocce che si restringono.
- `maldi`: sorgente MALDI con TOF lineare: impulso laser, pennacchio, accelerazione a energia uguale, tempo di volo ∝ √m.
- `trappola`: trappola ionica lineare. `fticr`: cella FT-ICR, ω_c ∝ z/m. `qqq`: triplo quadrupolo (Q1, q2 con gas, Q3).
- `ldIon(on, mode)` in `index.html` disegna punto con scia; con `LD_ANALYSERS` spento usa solo `base`; `q.s` scala la dimensione di un ione (gocce, frammenti).
- Orbitrap: r1=6, r2=15, Rm=17 (unità arbitrarie). Con Rm² = (r2²−r1²)/ln(r2²/r1²) i due elettrodi avrebbero tensione zero (`rmEqualPotential`): serve Rm maggiore; con Rm > r2 il barile esterno si chiude verso le punte, con Rm < r2 si apre (ramo W₋₁).
