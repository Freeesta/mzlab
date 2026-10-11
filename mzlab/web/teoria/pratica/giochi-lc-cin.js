/* mzLab - Pratica: the game "lc-cin" of pratica-tp.html (kinetic profiles of transformation products).
   Classic script after giochi-lc.js (LCUI: frame, aids, feedback helpers), cinetica-lcms.js (LCCIN: data and generator) and teoria.js (TP: DASH).
   Synthetic data, declared as such. Three steps: discard the false candidates, primary or secondary, time of the MS2 experiment.
   The coherence feedback gives numbers; the aids (3 levels: observation, family, zone) never say which candidate is which. */
"use strict";
const LCCINVIEW = (() => {
  const { sgn } = LCUI;
  const fx = LCCIN.fx, T = LCCIN.TIMES;
  const LVC = `<option value="1">1 · quattro candidati, rumore basso</option><option value="2" selected>2 · cinque candidati, più tipi di falsi</option><option value="3">3 · sei candidati, isomeri, rumore alto</option>`;
  const CLS = { tengo: "prodotto vero", t0bianco: "già presente a t0 o nel bianco", rt: "RT incoerente con il Δm", piatto: "profilo piatto, senza cinetica" };
  const CHOICES = [["tengo", "Tengo: prodotto di trasformazione"], ["t0bianco", "Scarto: già a t0 o nel bianco"], ["rt", "Scarto: RT incoerente con il Δm"], ["piatto", "Scarto: profilo piatto"]];
  const SERIES = ["var(--c1)", "var(--c2)", "var(--c3)", "var(--c4)", "var(--c5)", "var(--c6)"];
  const DASHES = ["", "7 4", "2 3", "8 3 2 3", "12 4", "1 4"];
  const MARKS = [(x, y) => `<circle cx="${x}" cy="${y}" r="4.5"/>`, (x, y) => `<rect x="${x - 4}" y="${y - 4}" width="8" height="8"/>`,
    (x, y) => `<path d="M${x} ${y - 5.5}L${x + 5} ${y + 4}L${x - 5} ${y + 4}Z"/>`, (x, y) => `<path d="M${x} ${y - 6}L${x + 6} ${y}L${x} ${y + 6}L${x - 6} ${y}Z"/>`,
    (x, y) => `<path d="M${x - 5} ${y - 5}L${x + 5} ${y + 5}M${x + 5} ${y - 5}L${x - 5} ${y + 5}" fill="none" stroke-width="2.4"/>`, (x, y) => `<path d="M${x - 5.5} ${y}H${x + 5.5}M${x} ${y - 5.5}V${y + 5.5}" fill="none" stroke-width="2.4"/>`];

  /** the profiles as an SVG line chart: series differ by colour, dash and marker; the same numbers are in the table beside it */
  function chart(e, tabId) {
    const W = 600, H = 300, m = { l: 52, r: 16, t: 14, b: 40 }, ymax = Math.max(10, Math.ceil(Math.max(...e.features.map(f => f.max)) / 100) * 100);
    const X = t => m.l + (W - m.l - m.r) * t / T[T.length - 1], Y = v => H - m.b - (H - m.t - m.b) * v / ymax;
    let s = `<svg class="cin-svg" viewBox="0 0 ${W} ${H}" role="img" aria-describedby="${tabId}" aria-label="Grafico: area XIC dei candidati in funzione del tempo di irraggiamento, da 0 a ${T[T.length - 1]} minuti, ${e.features.length} candidati; gli stessi valori sono nella tabella."><g class="cin-ax">`;
    for (let k = 0; k <= 4; k++) { const v = ymax * k / 4; s += `<line x1="${m.l}" x2="${W - m.r}" y1="${Y(v)}" y2="${Y(v)}" class="cin-grid"/><text x="${m.l - 6}" y="${Y(v) + 4}" text-anchor="end">${v}</text>`; }
    T.forEach(t => { s += `<line x1="${X(t)}" x2="${X(t)}" y1="${H - m.b}" y2="${H - m.b + 4}" class="cin-tick"/><text x="${X(t)}" y="${H - m.b + 17}" text-anchor="middle">${t}</text>`; });
    s += `<text x="${(m.l + W - m.r) / 2}" y="${H - 4}" text-anchor="middle">tempo di irraggiamento (min)</text><text transform="translate(13 ${(m.t + H - m.b) / 2}) rotate(-90)" text-anchor="middle">area XIC (migliaia di conteggi)</text>`;
    s += `<line x1="${m.l}" x2="${W - m.r}" y1="${Y(e.loq)}" y2="${Y(e.loq)}" class="cin-loq"/></g>`;
    e.features.forEach((f, i) => {
      const col = SERIES[i % 6], pts = f.areas.map((a, j) => [X(T[j]), Y(a)]);
      s += `<g class="cin-s" style="--s:${col}"><polyline points="${pts.map(p => p.join(",")).join(" ")}" fill="none" stroke="${col}" stroke-width="2" ${DASHES[i % 6] ? `stroke-dasharray="${DASHES[i % 6]}"` : ""}/><g fill="${col}" stroke="${col}">${pts.map(p => MARKS[i % 6](p[0], p[1])).join("")}</g></g>`;
    });
    s += "</svg>";
    const leg = e.features.map((f, i) => `<li><svg width="38" height="14" viewBox="0 0 38 14" aria-hidden="true"><line x1="1" x2="37" y1="7" y2="7" stroke="${SERIES[i % 6]}" stroke-width="2" ${DASHES[i % 6] ? `stroke-dasharray="${DASHES[i % 6]}"` : ""}/><g fill="${SERIES[i % 6]}" stroke="${SERIES[i % 6]}">${MARKS[i % 6](19, 7)}</g></svg> ${f.id}</li>`).join("");
    return `${s}<ul class="cin-leg" aria-label="Legenda">${leg}</ul><p class="note">La linea orizzontale tratteggiata sul grafico è il limite di quantificazione (${e.loq}).</p>`;
  }

  function table(e, tabId) {
    const head = `<tr><th scope="col">Candidato</th><th scope="col">m/z</th><th scope="col">Δm (u)</th><th scope="col">RT (min)</th><th scope="col">Bianco</th>${T.map((t, i) => `<th scope="col">${i ? t + " min" : "t0"}</th>`).join("")}</tr>`;
    const row = (name, f, pre) => `<tr><th scope="row">${name}</th><td class="n">${fx(f.mz, 4)}</td><td class="n">${pre ? "—" : sgn(f.dm, 4)}</td><td class="n">${fx(f.rt, 2)}</td><td class="n">${f.blank}</td>${f.areas.map(a => `<td class="n">${a}</td>`).join("")}</tr>`;
    return `<div class="cin-tw"><table class="eitab cin-tab" id="${tabId}"><caption class="note">Aree XIC (migliaia di conteggi), un campione per colonna. Il precursore è dato come riferimento.</caption>${head}${row("Precursore", e.precursor, true)}${e.features.map(f => row(f.id, f)).join("")}</table></div>`;
  }

  function markup(e, tabId, kept) {
    const opts = `<option value="">—</option>`;
    return `<h3>Profili di un prodotto di trasformazione</h3>
        <p>Il precursore <b>${LCUI.sub(e.precursor.f)}</b> (${e.desc}) è stato irraggiato con TiO<sub>2</sub>; i campioni sono stati prelevati a ${T.slice(1).join(", ")} min, più un campione a t0 (senza luce) e un bianco. Le aree XIC dei candidati sono in tabella e nel grafico.</p>
        <p class="note">Dati sintetici costruiti a scopo didattico: cinetica del primo ordine più rumore, formule inventate, non misure. Regola didattica per la fase inversa: un prodotto più polare (per esempio ossidato) eluisce prima del precursore, uno meno polare dopo.</p>
        <div class="hpbox noprint"></div>${chart(e, tabId)}${table(e, tabId)}
        <div class="st"><h4 id="c1">1. Quali candidati tieni come prodotti di trasformazione?</h4>
        <table class="eitab"><tr><th scope="col">Candidato</th><th scope="col">Scelta</th></tr>${e.features.map(f => `<tr><th scope="row">${f.id}</th><td><select class="c1s" data-id="${f.id}" aria-label="Scelta per ${f.id}">${opts}${CHOICES.map(c => `<option value="${c[0]}">${c[1]}</option>`).join("")}</select></td></tr>`).join("")}</table>
        <div class="row"><button type="button" class="pri ck1">Controlla</button></div><div class="fb"></div></div>
        <div class="st off" inert><h4 id="c2">2. Primario o secondario?</h4><p class="note">Un primario si forma direttamente dal precursore; un secondario si forma da un altro prodotto.</p>
        <table class="eitab"><tr><th scope="col">Candidato</th><th scope="col">Tipo</th></tr>${kept.map(f => `<tr><th scope="row">${f.id}</th><td><select class="c2s" data-id="${f.id}" aria-label="Tipo di ${f.id}"><option value="">—</option><option value="primario">primario</option><option value="secondario">secondario</option></select></td></tr>`).join("")}</table>
        <div class="row"><button type="button" class="pri ck2">Controlla</button></div><div class="fb"></div></div>
        <div class="st off" inert><h4 id="c3">3. Il campione per l'MS<sup>2</sup></h4><p class="note">Scegli un prodotto da frammentare e il tempo di irraggiamento a cui prelevare il campione.</p>
        <div class="row"><label>Prodotto <select class="c3p"><option value="">—</option>${kept.map(f => `<option value="${f.id}">${f.id}</option>`).join("")}</select></label><label>Tempo <select class="c3t"><option value="">—</option>${T.slice(1).map((t, i) => `<option value="${i + 1}">${t} min</option>`).join("")}</select></label><button type="button" class="pri ck3">Controlla</button></div><div class="fb"></div></div>`;
  }
  return { CLS, T, LVC, markup };
})();

const LCCINUI = (() => {
  const { sgn, frame, seed, fbSet, lock, open, aids, last } = LCUI;
  const { CLS, T, LVC } = LCCINVIEW;
  function cin(root) {
    const fr = frame(root, LVC, next);
    function next() {
      const e = last.cin = LCCIN.genCin(fr.level(), seed()), tabId = "cin-t" + e.seed;
      const kept = e.features.filter(f => f.truth === "tengo");
      fr.wk.innerHTML = LCCINVIEW.markup(e, tabId, kept);
      aids(fr.wk.querySelector(".hpbox"), e.help);
      const st = fr.wk.querySelectorAll(".st");
      let s1 = 0, s2 = 0;
      st[0].querySelector(".ck1").onclick = () => {
        const sel = [...st[0].querySelectorAll(".c1s")];
        if (sel.some(x => !x.value)) { fbSet(st[0].querySelector(".fb"), false, "Scegliete una risposta per ogni candidato."); return; }
        let good = 0;
        const lines = sel.map(x => {
          const f = e.features.find(g => g.id === x.dataset.id), c = LCCIN.coherenceCin(e, f.id), ok = x.value === f.truth; good += ok ? 1 : 0;
          return `<li class="${ok ? "ok" : "no"}"><b>${f.id}</b>: ${ok ? "giusto" : "no"}, ${CLS[f.truth]}. Bianco ${c.blankPct} % del massimo, t0 ${c.t0Pct} %, area massima ${c.max} a ${c.tmax} min, RT ${sgn(c.dRT, 2)} min dal precursore (Δm ${sgn(c.dm, 4)} u).</li>`;
        });
        s1 = good / sel.length; lock(st[0]);
        fbSet(st[0].querySelector(".fb"), s1 === 1, `${s1 === 1 ? "Tutto giusto." : `${good} su ${sel.length}.`}<ul class="hnote">${lines.join("")}</ul>`);
        if (kept.length) open(st[1]); else { s2 = 1; open(st[2]); }
      };
      st[1].querySelector(".ck2").onclick = () => {
        const sel = [...st[1].querySelectorAll(".c2s")];
        if (sel.some(x => !x.value)) { fbSet(st[1].querySelector(".fb"), false, "Scegliete una risposta per ogni prodotto."); return; }
        let good = 0;
        const lines = sel.map(x => {
          const f = e.features.find(g => g.id === x.dataset.id), c = LCCIN.coherenceCin(e, f.id), ok = x.value === f.role; good += ok ? 1 : 0;
          return `<li class="${ok ? "ok" : "no"}"><b>${f.id}</b>: ${ok ? "giusto" : "no"}, ${f.role}. Al primo prelievo (${T[1]} min) l'area è il ${c.r1Pct} % del massimo; il massimo è a ${c.tmax} min.</li>`;
        });
        s2 = good / sel.length; lock(st[1]);
        fbSet(st[1].querySelector(".fb"), s2 === 1, `${s2 === 1 ? "Tutto giusto." : `${good} su ${sel.length}.`}<ul class="hnote">${lines.join("")}</ul>`);
        open(st[2]);
      };
      const check3 = () => {
        const id = st[2].querySelector(".c3p").value, ti = +st[2].querySelector(".c3t").value;
        if (!id || !ti) { fbSet(st[2].querySelector(".fb"), false, "Scegliete il prodotto e il tempo."); return; }
        const f = e.features.find(g => g.id === id), pct = LCCIN.pctAtTime(f, ti), ok = f.ms2Times.includes(ti); lock(st[2]);
        fbSet(st[2].querySelector(".fb"), ok, `${ok ? "Giusto." : "No."} A ${T[ti]} min l'area di ${id} è il ${pct} % del suo massimo (${f.max}, a ${T[f.m.tmaxIdx]} min). Per un MS<sup>2</sup> con il segnale più alto servono i tempi in cui l'area supera l'${Math.round(LCCIN.FRAC * 100)} % del massimo: ${f.ms2Times.map(i => T[i] + " min").join(", ")}.`);
        const s = (s1 + s2 + (ok ? 1 : 0)) / 3;
        PAL.record("lc-cin", e.id, e.d0, { tp: s, dati: s1 }, s * 100); fr.pts("lc-cin");
      };
      st[2].querySelector(".ck3").onclick = check3;
    }
    next(); fr.pts("lc-cin");
  }
  return cin;
})();
GIOCHI_LC.cin = LCCINUI;
