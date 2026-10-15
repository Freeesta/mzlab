// @ts-check
// The marked points of the RT x m/z map: the shapes drawn on the map, the Excel / copy export and the table under it (sort, groups, reference).
// Classic script, loaded before mappa.js, which calls MAPFORME(ctx) and MAPTAB(ctx, forms) once inside its closure: ctx carries the helpers that live there.
const MAPFORME = c => {
  const { st, is2d, pts, ptsSave, overlay, margins, follow, grow, ppmOf, diffOpt, isHrF, PREF, grpTol, prefSave, SHAPES, GLYPH, SHAPE_KEY, spectrumAt } = c;
  /** @param {string} k @param {any} [o] */ const T = (k, o) => I18N.t(k, o);
  // the points of one group of RT share a shape (circle, square, triangle, diamond, by turns); with no groups (LR, table not ready) a circle
  const grpShape = (p, i) => { const r = st(p).rows; return r && r.length === pts(p).length && r[i] && r[i].g ? SHAPES[(r[i].g - 1) % SHAPES.length] : "circle"; };
  function shapePath(g, kind, x, y, r) {
    g.beginPath();
    if (kind === "square") g.rect(x - r * .88, y - r * .88, r * 1.76, r * 1.76);
    else if (kind === "triangle") { g.moveTo(x, y - r * 1.15); g.lineTo(x + r * 1.1, y + r * .85); g.lineTo(x - r * 1.1, y + r * .85); g.closePath(); }
    else if (kind === "diamond") { g.moveTo(x, y - r * 1.2); g.lineTo(x + r * 1.2, y); g.lineTo(x, y + r * 1.2); g.lineTo(x - r * 1.2, y); g.closePath(); }
    else g.arc(x, y, r, 0, 2 * Math.PI);
  }
  function drawPoints(p, g) {
    const a = p._a, s = st(p); g.save(); g.font = fpx(11, "bold "); g.textAlign = "center";
    pts(p).forEach((q, i) => {
      if (q.rt < a.x0 || q.rt > a.x1 || q.mz < a.y0 || q.mz > a.y1) return;
      const x = a.X(q.rt), y = a.Y(q.mz), on = s.psel === i;
      shapePath(g, grpShape(p, i), x, y, on ? 9 : 7.5); g.fillStyle = css("--panel"); g.globalAlpha = .9; g.fill(); g.globalAlpha = 1;
      g.lineWidth = on ? 2.4 : 1.4; g.strokeStyle = on ? css("--accent") : css("--ink"); g.stroke();
      g.fillStyle = css("--ink"); g.fillText(String(i + 1), x, y + 4);
    });
    g.restore();
  }
  const COLS = ["n", "rt", "mz", "ia", "ib", "d", "ratio", "sn", "g", "dm", "note"];
  const COLK = { n: ["mappa.col.n", "mappa.col.n.title"], rt: ["mappa.col.rt", "mappa.col.rt.title"], mz: ["mappa.col.mz", "mappa.col.mz.title"], ia: ["mappa.col.ia", "mappa.col.ia.title"], ib: ["mappa.col.ib", "mappa.col.ib.title"], d: ["mappa.col.d", "mappa.col.d.title"], ratio: ["mappa.col.ratio", "mappa.col.ratio.title"], sn: ["mappa.col.sn", "mappa.col.sn.title"], g: ["mappa.col.g", "mappa.col.g.title"], dm: ["mappa.col.dm", "mappa.col.dm.title"], note: ["mappa.col.note", "mappa.col.note.title"] };     // literal keys: the i18n check finds them
  function sheetRows(p) {
    const s = st(p), hr = isHrF(p._a.f);
    return (s.rows || []).map(r => [r.n, r.rt != null ? +r.rt.toFixed(3) : "", r.mz != null ? +r.mz.toFixed(hr ? 5 : 2) : "", r.ia ?? "", r.ib ?? "", r.d ?? "", r.ratio != null ? +r.ratio.toFixed(3) : "", r.sn != null ? +r.sn.toFixed(1) : "", r.g ?? "", r.dm != null ? +r.dm.toFixed(hr ? 5 : 2) : "", r.note || ""]);
  }
  const headRow = () => COLS.map(c => T(COLK[c][0]));
  function excel(p) {
    const a = p._a, f = a.f, meta = [[T("mappa.meta.fileA"), f.label], [T("mappa.meta.fileB"), a.ref ? a.ref.label : "–"], [T("mappa.meta.level"), f.lv === 2 ? "MS2" : "MS1"],
      [T("mappa.meta.xic"), isHrF(f) ? `± ${ppmOf(f)} ppm` : "[n − 0,2; n + 0,8]"], [T("mappa.meta.rttol"), `± ${diffOpt().rttol} min`], [T("mappa.meta.grp"), `± ${grpTol()} min`],
      [T("mappa.meta.snap"), "± 0.2 min · ± 1 bin"], [T("mappa.meta.date"), new Date().toLocaleString(LOCALE)]];
    dlx(`${f.label}${a.ref ? "-" + a.ref.label : ""}_punti.xlsx`, [{ name: T("mappa.tab.sheet"), head: headRow(), rows: sheetRows(p), widths: [5, 9, 11, 12, 12, 12, 9, 8, 7, 10, 30] },
      { name: T("mappa.meta.sheet"), head: [T("mappa.meta.what"), T("mappa.meta.value")], rows: meta, widths: [30, 40] }], { p, table: "map-points" });
  }
  async function copyRows(p) {
    const txt = await PROV.tsv([headRow(), ...sheetRows(p)].map(r => r.join("\t")).join("\n"), { p, table: "map-points" });
    try { await navigator.clipboard.writeText(txt); nearMsg(T("mappa.tab.copied")); } catch (e) { nearMsg(T("mappa.tab.copyFail")); }
  }
  return { grpShape, drawPoints, shapePath, COLS, COLK, excel, copyRows };
};
const MAPTAB = (c, f) => {
  const { st, is2d, pts, ptsSave, overlay, margins, follow, grow, ppmOf, diffOpt, isHrF, PREF, grpTol, prefSave, SHAPES, GLYPH, SHAPE_KEY, spectrumAt } = c;
  /** @param {string} k @param {any} [o] */ const T = (k, o) => I18N.t(k, o);
  const { COLS, COLK, excel, copyRows } = f;
  const openDm = (dm, hr, pol) => dmOpen(dm, hr, pol);      // the same recognition and destination as the ruler of the spectrum (spettro.js)
  const xicAB = (p, mz) => xicDirect(mz, p);                // XIC of A and B overlaid: a new XIC panel with the m/z
  let tT = 0;
  function table(p) {
    clearTimeout(tT); tT = setTimeout(() => tableNow(p), 30);
  }
  async function tableNow(p) {
    const s = st(p), a = p._a; if (!a || !s.grid) return;
    let box = s.tab;
    const list = is2d(p) ? pts(p) : [];
    if (!list.length) { if (box) { box.remove(); s.tab = null; grow(p, "tab", 0); } return; }
    if (!box) { box = s.tab = document.createElement("div"); box.className = "mztab"; s.grid.after(box); }
    const f = a.f, ref = a.ref, mzq = list.map(q => `${(+q.rt).toFixed(4)},${(+q.mz).toFixed(5)}`).join(";");
    let rows = [];
    try { rows = (await J(`api/mappunti?k=${f.k}&level=${f.lv || 1}${ref ? "&ref=" + ref.k : ""}&ppm=${ppmOf(f)}&grp=${grpTol()}&pts=${mzq}`)).rows; } catch (e) { rows = list.map(q => ({ rt: q.rt, mz: q.mz })); }
    rows = rows.map((r, i) => ({ ...r, n: i + 1, note: list[i] ? list[i].note || "" : "" }));
    if (PREF.align && !s.aligning) {                              // «align»: every point of a group takes the RT of its first marked point, so they stand on one vertical line (measures are asked again there)
      const first = new Map(); let moved = false;
      rows.forEach((r, i) => { if (r.g && list[i]) { if (!first.has(r.g)) first.set(r.g, list[i].rt); else if (list[i].rt !== first.get(r.g)) { list[i].rt = first.get(r.g); moved = true; } } });
      if (moved) { ptsSave(p); s.aligning = true; try { return await tableNow(p); } finally { s.aligning = false; } }
    }
    const ri = list.findIndex(q => q.ref);                      // the point the student chose as the reference of Δm (none: the most intense of each group)
    if (ri >= 0 && rows[ri] && rows[ri].mz != null) rows.forEach(r => { r.dm = r.mz == null ? null : r.mz - rows[ri].mz; r.isref = r.n - 1 === ri; });
    s.rows = rows; overlay(p);                                          // the shapes of the points come from the groups of the table
    const so = s.sort || { c: "n", d: 1 }, val = (r, c) => r[c] == null ? -Infinity : r[c];
    const view = rows.slice().sort((u, v) => typeof val(u, so.c) === "string" ? so.d * String(val(u, so.c)).localeCompare(String(val(v, so.c))) : so.d * (val(u, so.c) - val(v, so.c)));
    const dec = mzd(p), hr = isHrF(f), num = (v, d) => v == null || !Number.isFinite(v) ? "–" : v.toFixed(d);
    const head = COLS.map(c => `<th data-c="${c}" title="${T(COLK[c][1])}">${T(COLK[c][0])}${so.c === c ? (so.d > 0 ? " ▲" : " ▼") : ""}</th>`).join("") + `<th title="${T("mappa.tab.ref.title")}">${T("mappa.tab.ref")}</th><th></th>`;
    const rowHtml = r => `<tr data-i="${r.n - 1}" class="${s.psel === r.n - 1 ? "on" : ""}">` +
      `<td>${r.g ? `<span aria-hidden="true">${GLYPH[(r.g - 1) % GLYPH.length]}</span> ` : ""}${r.n}</td><td>${num(r.rt, 2)}</td><td>${num(r.mz, hr ? 4 : 1)}</td><td>${r.ia == null ? "–" : fmt(r.ia)}</td><td>${r.ib == null ? "–" : fmt(r.ib)}</td><td>${r.d == null ? "–" : fmt(r.d)}</td>` +
      `<td>${num(r.ratio, 2)}</td><td>${num(r.sn, 1)}</td><td>${r.g ?? "–"}</td><td>${r.dm == null ? "–" : r.dm === 0 ? "0" : `<a href="#" data-dm="${r.dm}">${(r.dm > 0 ? "+" : "") + r.dm.toFixed(hr ? 4 : 1)}</a>`}</td>` +
      `<td><input data-note="${r.n - 1}" value="${EH(r.note || "")}" placeholder="${T("mappa.tab.note")}"></td><td><button data-ref="${r.n - 1}" aria-pressed="${r.isref ? "true" : "false"}" title="${T("mappa.tab.ref.pick", { n: r.n })}" aria-label="${T("mappa.tab.ref.pick", { n: r.n })}">${r.isref ? "●" : "○"}</button></td><td><button data-xic="${r.n - 1}" title="${T("mappa.tab.xic.title")}">XIC</button></td></tr>`;
    const html = `<div class="mzt-bar"><b>${T("mappa.tab.title", { n: rows.length })}</b><label class="muted" title="${T("mappa.tab.grp.title")}">${T("mappa.tab.grp")} ± <input data-t="grp" class="mzf" inputmode="decimal" value="${grpTol()}"> min</label><label class="muted" title="${T("mappa.tab.align.title")}"><input type="checkbox" data-t="align"${PREF.align ? " checked" : ""}> ${T("mappa.tab.align")}</label>` +
      `<span class="sp"></span><button data-t="xl" title="${T("mappa.tab.xl.title")}">Excel</button><button data-t="cp" title="${T("mappa.tab.cp.title")}">${T("mappa.tab.cp")}</button><button data-t="clear">${T("mappa.tab.clear")}</button></div>` +
      `<div class="mzt-sc"><table><thead><tr>${head}</tr></thead><tbody>${groupedRows(s, view, rowHtml, !s.sort)}</tbody></table></div>`;
    box.innerHTML = h`${raw(html)}`;
    box.querySelectorAll("[data-gtog]").forEach(b => b.onclick = () => { const g = +b.dataset.gtog; if (s.gcol.has(g)) s.gcol.delete(g); else s.gcol.add(g); tableNow(p); });
    box.querySelectorAll("th[data-c]").forEach(th => th.onclick = () => { const c = th.dataset.c; s.sort = { c, d: so.c === c ? -so.d : 1 }; tableNow(p); });
    box.querySelector('[data-t="grp"]').onchange = ev => { const v = numIt(ev.target.value); if (v > 0) { PREF.grp = v; prefSave(); } tableNow(p); };
    box.querySelector('[data-t="align"]').onchange = ev => { PREF.align = ev.target.checked; prefSave(); tableNow(p); };
    box.querySelector('[data-t="xl"]').onclick = () => excel(p);
    box.querySelector('[data-t="cp"]').onclick = () => copyRows(p);
    box.querySelector('[data-t="clear"]').onclick = async () => { if (await yesno(T("mappa.tab.clear.ask"))) { pts(p).splice(0); ptsSave(p); overlay(p); tableNow(p); } };
    box.querySelectorAll("tbody tr[data-i]").forEach(tr => {
      const i = +tr.dataset.i;
      tr.onclick = ev => { if (ev.target.closest("input,button,a")) return; const q = pts(p)[i]; if (!q) return; s.psel = i; s.lock = { rt: q.rt, mz: q.mz }; follow(p, s.lock); overlay(p); margins(p); tableNow(p); };
      tr.ondblclick = ev => { if (ev.target.closest("input,button,a")) return; const q = pts(p)[i]; if (q) spectrumAt(p, q.rt); };
    });
    box.querySelectorAll("[data-note]").forEach(inp => inp.onchange = () => { const q = pts(p)[+inp.dataset.note]; if (q) { q.note = inp.value; ptsSave(p); } });
    box.querySelectorAll("[data-ref]").forEach(b => b.onclick = () => { const i = +b.dataset.ref, was = !!(pts(p)[i] || {}).ref; pts(p).forEach(q => { delete q.ref; }); if (!was && pts(p)[i]) pts(p)[i].ref = true; ptsSave(p); tableNow(p); });
    box.querySelectorAll("[data-xic]").forEach(b => b.onclick = () => { const q = pts(p)[+b.dataset.xic]; if (q) xicAB(p, q.mz); });
    box.querySelectorAll("[data-dm]").forEach(l => l.onclick = ev => { ev.preventDefault(); openDm(+l.dataset.dm, isHrF(f), f.polarity); });
    grow(p, "tab", Math.ceil(box.offsetHeight) + 5);                    // the panel grows by the height of the table (it scrolls inside beyond its maximum): the map keeps its size
  }
  // HR: the rows of one group (more than one point) sit together under a row of the group that can be folded; the other points follow as they are
  function groupedRows(s, view, rowHtml, byRt) {
    const gcol = s.gcol || (s.gcol = new Set()), by = new Map(), out = [];
    view.forEach(r => { if (!by.has(r.g)) by.set(r.g, []); by.get(r.g).push(r); });
    for (const [g, mem] of by) {
      if (mem.length < 2) { out.push(rowHtml(mem[0])); continue; }
      if (byRt) mem.sort((x, y) => x.rt - y.rt || y.ia - x.ia);                  // unsorted table: the members by RT, then the most intense first
      const open = !gcol.has(g), rts = mem.map(r => r.rt).filter(v => v != null), gl = GLYPH[(g - 1) % GLYPH.length], nm = T(SHAPE_KEY[SHAPES[(g - 1) % SHAPES.length]]);
      const lab = T("mappa.grp.row", { g, shape: nm, n: mem.length, members: mem.map(r => r.n).sort((x, y) => x - y).join(", ") });
      out.push(`<tr class="mzgr"><td colspan="${COLS.length + 2}"><button data-gtog="${g}" aria-expanded="${open}" aria-label="${T(open ? "mappa.grp.fold" : "mappa.grp.unfold", { g })}" title="${T(open ? "mappa.grp.fold" : "mappa.grp.unfold", { g })}">${open ? "▾" : "▸"}</button> ` +
        `<b><span aria-hidden="true">${gl}</span> ${EH(lab)}</b> <span class="muted">RT ${Math.min(...rts).toFixed(2)}–${Math.max(...rts).toFixed(2)} min</span></td></tr>`);
      if (open) mem.forEach(r => out.push(rowHtml(r)));
    }
    return out.join("");
  }
  return { table, tableNow };
};
