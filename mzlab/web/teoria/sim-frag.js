/* QqQ lab - Teoria: fragmentation chemistry.
   1) competition between a rearrangement (tight TS) and a simple cleavage (loose TS), classical RRK model;
   2) neutral-loss calculator: losses from the precursor and between fragments, nitrogen-rule parity. */
"use strict";
// The two simulations are functions of their container, so the Teoria pages and the side panel «Frammenta» (strumenti/frammenta.js) share them.
const SIMFRAG = (() => {
  const col = n => getComputedStyle(document.documentElement).getPropertyValue("--c" + n).trim();

  // ------------------------------------------------------------ 1) RRK competition
  function rrk(rk) {
    const v = TP.controls(rk, [
      { id: "e", label: "Energia interna dello ione E", min: 0.5, max: 10, step: 0.05, value: 3.5, unit: "eV", fmt: x => TP.fmt(x, 2) },
      { id: "s", label: "Oscillatori effettivi s (dimensione)", min: 8, max: 60, step: 1, value: 25, fmt: x => x },
      { id: "e1", label: "Barriera del riarrangiamento E₀", min: 0.8, max: 2.5, step: 0.05, value: 1.3, unit: "eV", fmt: x => TP.fmt(x, 2) },
      { id: "e2", label: "Barriera della scissione diretta E₀", min: 1.5, max: 4, step: 0.05, value: 2.2, unit: "eV", fmt: x => TP.fmt(x, 2) },
    ], () => draw());
    const c = TP.canvas(rk, 300); c.onresize = draw;
    const out = document.createElement("div"); out.className = "readout"; rk.appendChild(out);
    const NU1 = 1e9, NU2 = 1e14, KOBS = 1e4; // frequency factors (s^-1) and observation threshold
    const k = (E, E0, nu, s) => E <= E0 ? 0 : nu * Math.pow((E - E0) / E, s - 1);
    function draw() {
      const xs = []; for (let e = 0.5; e <= 10.0001; e += 0.02) xs.push(e);
      const ax = TP.axes(c, { x0: 0.5, x1: 10, y0: 1e-2, y1: 1e14, logy: true, xl: "energia interna E (eV)", yl: "k(E) (s⁻¹)", m: { l: 62 } });
      ax.clip();
      ax.ctx.fillStyle = "rgba(4,120,87,.07)"; ax.ctx.fillRect(ax.m.l, ax.m.t, ax.W - ax.m.l - ax.m.r, ax.Y(KOBS) - ax.m.t);
      TP.line(ax, [0.5, 10], [KOBS, KOBS], col(3), 1.2, [5, 4]);
      TP.line(ax, xs, xs.map(e => k(e, v.e1, NU1, v.s) || NaN), col(2), 2.4, TP.DASH[1]);
      TP.line(ax, xs, xs.map(e => k(e, v.e2, NU2, v.s) || NaN), col(1), 2.4);
      TP.line(ax, [v.e, v.e], [1e-2, 1e14], "#24231f", 1, [3, 3]);
      ax.ctx.restore();
      TP.label(ax, 9.9, KOBS, "osservabile nella cella (k ≳ 10⁴ s⁻¹)", col(3), "right", "bottom", -4);
      TP.legend(ax, [["riarrangiamento (stato di transizione «stretto», ν = 10⁹ s⁻¹)", col(2), TP.DASH[1]], ["scissione diretta («lasco», ν = 10¹⁴ s⁻¹)", col(1)]]);
      const k1 = k(v.e, v.e1, NU1, v.s), k2 = k(v.e, v.e2, NU2, v.s), kt = k1 + k2;
      const obs = kt >= KOBS;
      out.innerHTML = `A E = ${TP.fmt(v.e, 2)} eV: k<sub>riarr.</sub> = <b>${k1 ? TP.sci(k1, 1) : "0"}</b> s⁻¹, k<sub>scissione</sub> = <b>${k2 ? TP.sci(k2, 1) : "0"}</b> s⁻¹. ` +
        (kt > 0 ? `Frazione che segue il riarrangiamento: <b>${TP.fmt(100 * k1 / kt, 0)} %</b>. ` : "Sotto entrambe le soglie: lo ione non si frammenta. ") +
        (kt > 0 && !obs ? `<span class="bad">Le reazioni sono possibili ma troppo lente per avvenire durante il passaggio nella cella (spostamento cinetico).</span>` : kt > 0 ? `<span class="ok">Frammentazione osservabile.</span>` : "") +
        ` Modello RRK classico, k = ν [(E − E₀)/E]<sup>s−1</sup>: valori illustrativi.`;
    }
    draw();
  }

  // ------------------------------------------------------------ 2) neutral-loss calculator
  function perdite(nl, init = { p: "216", f: "174, 146, 132, 104" }) {
    // nominal neutral losses, [mass, label, radical?]
    const L = [
      [1, "H•", 1], [2, "H₂"], [15, "CH₃•", 1], [16, "O (N-ossidi), CH₄, NH₂•"], [17, "NH₃ / OH•"], [18, "H₂O"], [20, "HF"],
      [26, "C₂H₂"], [27, "HCN"], [28, "CO / C₂H₄"], [29, "CHO• / C₂H₅•", 1], [30, "CH₂O / NO•"], [31, "CH₃NH₂ / CH₃O•"],
      [32, "CH₃OH / S"], [34, "H₂S"], [35, "Cl•", 1], [36, "HCl"], [41, "CH₃CN"], [42, "C₃H₆ / CH₂CO / NH₂CN"], [43, "HNCO / C₃H₇•"],
      [44, "CO₂ / CH₃CHO / C₂H₄O"], [45, "(CH₃)₂NH"], [46, "HCOOH / NO₂• / C₂H₅OH"], [48, "CH₃SH / SO"], [56, "C₄H₈"],
      [57, "CH₃NCO"], [60, "CH₃COOH"], [64, "SO₂ / CH₃SOH"], [80, "HBr / SO₃"], [128, "HI"]
    ];
    const v = TP.controls(nl, [
      { id: "p", label: "m/z del precursore", text: true, value: init.p },
      { id: "f", label: "m/z dei frammenti (separati da virgole o spazi)", text: true, value: init.f },
    ], () => draw());
    const out = document.createElement("div"); out.className = "readout"; nl.appendChild(out);
    const num = s => parseFloat(String(s).replace(",", "."));
    const match = d => L.filter(x => x[0] === d).map(x => x[1] + (x[2] ? " <span class='bad'>(radicale)</span>" : ""));
    function draw() {
      const p = num(v.p), fr = String(v.f).split(/[\s;]+|,(?=\s)|,(?=\d{2,})/).map(num).filter(x => isFinite(x) && x > 0 && x < p).sort((a, b) => b - a);
      if (!isFinite(p) || !fr.length) { out.innerHTML = "<span class='bad'>Inserite il precursore e almeno un frammento più leggero.</span>"; return; }
      const P = Math.round(p), par = x => Math.round(x) % 2 === 0 ? "pari" : "dispari";
      const rows = fr.map(f => {
        const F = Math.round(f), d = P - F, m = match(d);
        const inner = fr.filter(g => g > f).map(g => { const dd = Math.round(g) - F, mm = match(dd); return mm.length ? `da ${Math.round(g)}: −${dd} (${mm.join(", ")})` : ""; }).filter(Boolean);
        const parity = (F % 2) === (P % 2) ? "stessa parità del precursore" : "<b>parità cambiata</b>: perso un numero dispari di N (o frammento radicalico)";
        return `<tr><td class="n"><b>${F}</b></td><td class="n">−${d}</td><td>${m.length ? m.join("; ") : "<span style='color:var(--muted)'>nessuna perdita semplice: forse più passaggi</span>"}</td><td>${inner.join("<br>") || "–"}</td><td>${par(F)}: ${parity}</td></tr>`;
      }).join("");
      out.innerHTML = `<div class="tw"><table><tr><th class="n">frammento</th><th class="n">Δ dal precursore</th><th>perdite neutre compatibili</th><th>da un altro frammento</th><th>regola dell'azoto (ioni a elettroni pari)</th></tr>${rows}</table></div>` +
        `<span style="color:var(--muted)">Masse nominali: ogni riga è un elenco di ipotesi. Il precursore ${P} è ${par(P)}: per uno ione a elettroni pari, m/z pari ⇒ numero dispari di atomi di N.</span>`;
    }
    draw();
  }

  function init() {
    const a = document.getElementById("sim-rrk"), b = document.getElementById("sim-loss");
    if (a) rrk(a);
    if (b) perdite(b);
  }
  return { rrk, perdite, init };
})();
if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", SIMFRAG.init); else SIMFRAG.init();
