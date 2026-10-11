/* mzLab - Pratica: game "lc-ms2" (H7 "Quale MSn fare") of pratica-tp.html. Classic script on top of msn-lcms.js (LCMSN) and giochi-lc.js (LCUI).
   Data are CONSTRUCTED for teaching. The program never says which ion to isolate: aids (3 levels) narrow the reasoning, the check gives the numbers. */
"use strict";
const LCMSNVIEW = (() => {
  const { fx } = LCMSN;
  const LV = `<option value="1">1 · quattro gruppi</option><option value="2" selected>2 · cinque gruppi</option><option value="3">3 · cinque gruppi, più doppie perdite</option>`;
  const gname = g => LCMSN.GROUPS[g].name;
  const ionRows = e => e.ions.map(x => `<tr><th scope="row">${x.id}</th><td class="n">${fx(x.mz, 1)}</td><td class="n">${x.abund}</td></tr>`).join("");
  const markup = e => `<h3>Quale MS<sup>n</sup> fare</h3>
    <p>Precursore [M+H]<sup>+</sup> <b>m/z ${fx(e.precMz, 1)}</b>, formula <b>${PAL.fhtml(e.formula)}</b>. Gruppi funzionali noti: ${e.groups.map(gname).join(", ")}. Lo spettro MS<sup>2</sup> è in tabella (risoluzione unitaria, abbondanza in % del picco base). Un MS<sup>3</sup> sotto il ${e.threshold} % è troppo debole.</p>
    ${LCUI.ds}<div class="hpbox noprint"></div>
    <div class="cin-tw"><table class="cin-tab" aria-label="Spettro MS2"><tr><th>ione</th><th>m/z</th><th>abbondanza (%)</th></tr>${ionRows(e)}</table></div>
    <div class="st"><h4 id="q1">1. Quale ione isolate per l'MS<sup>3</sup>?</h4>
    <div class="opts" role="group" aria-labelledby="q1">${e.ions.map(x => `<button type="button" data-id="${x.id}">${x.id} · m/z ${fx(x.mz, 1)}</button>`).join("")}</div><div class="fb" role="status"></div></div>
    <div class="st off" inert><h4 id="q2">2. Quanti picchi nuovi (non già nello spettro MS<sup>2</sup>) vi aspettate dall'MS<sup>3</sup> dello ione <span class="pid"></span>?</h4>
    <div class="row"><label>picchi nuovi <input class="nn" size="3" inputmode="numeric" aria-labelledby="q2"></label><button type="button" class="pri ck2">Controlla</button></div><div class="fb" role="status"></div></div>`;
  const ms3Table = (e, ion) => `<table class="cin-tab" aria-label="MS3 di ${ion.id}"><tr><th>m/z</th><th>perdita</th><th>nuovo?</th></tr>${LCMSN.ms3Of(e, ion).map(d => `<tr><td class="n">${fx(d.mz, 1)}</td><td>${PAL.fhtml(P_(d.loss))}</td><td>${d.novel ? "sì" : "no, già in MS<sup>2</sup>"}</td></tr>`).join("")}</table>`;
  const P_ = s => PAL.parse(s);
  const numbers = e => `<table class="cin-tab"><tr><th>ione</th><th>gruppi rimasti</th><th>doppie perdite già in MS<sup>2</sup></th><th>picchi nuovi</th><th>abbondanza (%)</th></tr>${LCMSN.coherence(e).map(c => `<tr><th scope="row">${c.id}</th><td class="n">${c.remaining}</td><td class="n">${c.seen}</td><td class="n">${c.novel}</td><td class="n">${c.abund}${c.feasible ? "" : " (sotto soglia)"}</td></tr>`).join("")}</table>`;
  return { LV, markup, ms3Table, numbers };
})();

const LCMSNUI = (() => {
  const { frame, seed, fbSet, lock, open, aids, last, num } = LCUI;
  function ms2(root) {
    const fr = frame(root, LCMSNVIEW.LV, next);
    function next() {
      const e = last.ms2 = LCMSN.genMsn(fr.level(), seed());
      fr.wk.innerHTML = LCMSNVIEW.markup(e);
      aids(fr.wk.querySelector(".hpbox"), e.help);
      const st = fr.wk.querySelectorAll(".st"), probe = e.ions.find(x => x.id === e.probeId);
      let s1 = 0;
      st[0].querySelectorAll(".opts button").forEach(b => {
        b.onclick = () => {
          const ion = e.ions.find(x => x.id === b.dataset.id), ok = e.bestIds.includes(ion.id);
          s1 = ok ? 1 : 0; lock(st[0]); b.classList.add(ok ? "right" : "wrong");
          const why = ion.kind !== "single" ? "È uno ione a doppia perdita: nei dati costruiti non si frammenta oltre." : "";
          fbSet(st[0].querySelector(".fb"), ok, `${ok ? "Coerente con i conteggi." : "Non coerente con i conteggi."} ${why} MS<sup>3</sup> di ${ion.id}:${LCMSNVIEW.ms3Table(e, ion)}Conteggi per tutti gli ioni a perdita singola:${LCMSNVIEW.numbers(e)}`);
          st[1].querySelector(".pid").textContent = probe.id + " (m/z " + LCMSN.fx(probe.mz, 1) + ")";
          open(st[1]);
        };
      });
      st[1].querySelector(".ck2").onclick = () => {
        const v = num(st[1].querySelector(".nn").value);
        if (v == null || v < 0) { fbSet(st[1].querySelector(".fb"), false, "Scrivete un numero intero."); return; }
        const n = LCMSN.infoOf(e, probe), ok = v === n; lock(st[1]);
        fbSet(st[1].querySelector(".fb"), ok, `${ok ? "Giusto." : "No."} Per ${probe.id} i picchi nuovi sono ${n}:${LCMSNVIEW.ms3Table(e, probe)}`);
        const s = (s1 + (ok ? 1 : 0)) / 2;
        PAL.record("lc-ms2", e.id, e.d0, { tp: s, ms: s }, s * 100); fr.pts("lc-ms2");
      };
    }
    next(); fr.pts("lc-ms2");
  }
  return ms2;
})();
GIOCHI_LC.ms2 = LCMSNUI;
