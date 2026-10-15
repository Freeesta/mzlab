// @ts-strict
"use strict";
// The «Sample metadata» section of the «Statistica» panel (first section): the table of CAMPIONI (campioni.js), one row per file, with the TSV export and import.
// Every edit goes through CAMPIONI.apply, the only writer of roles, subtypes, times and their confirmation.
(() => {
  const { files, apply, selVal, kindOf, shownRep, importTsv, gradientLegend, toTsv, rows } = CAMPIONI;
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  const T_UNITS = ["min", "h", "d"];
  const C_UNITS = ["mg/L", "µg/L", "ng/L", "g/L"];
  const isDark = name => files().some(f => f.file === name && kindOf(f) === "dark");        // only a dark takes a signed time

  // ---------------------------------------------------------------- the section
  function paint(box) {
    const fs = files(), u = tunitGet();
    const none = `<p class="sm">${t("meta.none")}</p>`;
    if (!fs.length) { box.innerHTML = none; return; }
    const body = fs.map((f, k) => {
      const r = selVal(f), id = `data-f="${eh(f.file)}"`, lab = t("meta.of", { f: f.label || f.file }), guess = f.time != null && f.tconf === false;
      return `<tr><td><span class="qq-n">${k + 1}</span> ${eh(f.label || f.file)}</td>
<td><select ${id} data-m="type" aria-label="${eh(lab)} · ${eh(t("qaqc.col.type"))}">${RUOLI.OPTS.map(x => `<option value="${x}"${x === r ? " selected" : ""}>${eh(RUOLI.label(x))}</option>`).join("")}</select></td>
<td class="num"><input ${id} data-m="time" inputmode="decimal" size="5" value="${eh(timeShown(f.time, u) ?? "")}"${guess ? ` class="guess" title="${eh(t("load.guess.title"))}"` : ""} aria-label="${eh(lab)} · ${eh(t("meta.time"))}"></td>
<td class="num"><input ${id} data-m="conc" inputmode="decimal" size="6" value="${eh(f.conc ?? "")}" aria-label="${eh(lab)} · ${eh(t("meta.conc"))}"><select ${id} data-m="cunit" aria-label="${eh(lab)} · ${eh(t("meta.cunit"))}">${C_UNITS.map(x => `<option${x === (f.cunit || "mg/L") ? " selected" : ""}>${x}</option>`).join("")}</select></td>
<td><input ${id} data-m="rep" size="8" value="${eh(f.rep || "")}" placeholder="${eh(shownRep(f))}" aria-label="${eh(lab)} · ${eh(t("meta.rep"))}"></td></tr>`;
    }).join("");
    const html = `<p class="sm">${t("meta.intro")}</p>
<div class="qq-scroll"><table class="qq-tab" id="cmp-tab"><thead><tr><th scope="col">${t("qaqc.col.file")}</th><th scope="col">${t("qaqc.col.type")}</th>
<th scope="col">${t("meta.time")} <select id="cmp-tu" aria-label="${eh(t("load.table.tunit.title"))}">${T_UNITS.map(x => `<option value="${x}"${x === u ? " selected" : ""}>${eh(tunitLabel(x))}</option>`).join("")}</select></th>
<th scope="col">${t("meta.conc")}</th><th scope="col">${t("meta.rep")}</th></tr></thead><tbody>${body}</tbody></table></div>
<div class="qq-form"><button type="button" id="cmp-exp">${t("meta.export")}</button><button type="button" id="cmp-copy">${t("meta.copy")}</button><label class="cmp-imp"><span>${t("meta.import")}</span><input type="file" id="cmp-imp" accept=".tsv,.csv,.txt"></label></div>
<div id="cmp-msg" class="sm" role="status"></div>${gradientLegend(fs)}`;
    box.innerHTML = html;
  }
  function msg(box, text, bad) { const m = box.querySelector("#cmp-msg"); if (m) { m.textContent = text; m.classList.toggle("bad", !!bad); } }
  function tsvText() { return toTsv(rows(), tunitGet()); }
  function mount(box) {
    paint(box);
    box.onchange = e => {
      const el = e.target, f = el.dataset && el.dataset.f;
      if (el.id === "cmp-tu") { tunitSet(el.value); paint(box); if (typeof renderFileList === "function") renderFileList(); return; }
      if (el.id === "cmp-imp") {
        const file = el.files && el.files[0]; if (!file) return;
        FILESICURI.readText(file).then(tx => {
          const r = importTsv(tx);
          paint(box);
          msg(box, r.changed ? t("meta.imported", { n: r.changed }) + (r.unknown.length ? " " + t("meta.unknown", { list: r.unknown.join(", ") }) : "") : t("meta.imported.none"), !r.changed);
        }, x => msg(box, I18N.t(x.key || "workflow.err.invalidFormat"), true));
        return;
      }
      if (!f) return;
      const m = el.dataset.m, ent = { file: f };
      if (m === "type") ent.role = el.value;
      else if (m === "rep") ent.rep = el.value.trim();
      else if (m === "cunit") ent.cunit = el.value;
      else {
        const v = parseDecimal(el.value, m === "time" && isDark(f));
        if (Number.isNaN(v)) { paint(box); msg(box, t("meta.err.num"), true); return; }
        if (m === "time") ent.time = timeStored(v, tunitGet()); else { ent.conc = v; if (v != null) ent.cunit = (box.querySelector(`select[data-f="${CSS.escape(f)}"][data-m=cunit]`) || {}).value || "mg/L"; }
      }
      apply([ent]); paint(box);
    };
    box.onkeydown = e => {
      const el = e.target;
      if (e.key !== "Enter" || !el.dataset || el.dataset.m !== "time" || el.disabled) return;
      const v = parseDecimal(el.value, isDark(el.dataset.f));
      if (v != null && !Number.isNaN(v)) { apply([{ file: el.dataset.f, time: timeStored(v, tunitGet()), tconf: true }]); paint(box); }
    };
    box.onclick = e => {
      if (e.target.id === "cmp-exp") {
        dl("metadata.tsv", tsvText(), "text/tab-separated-values", { table: "metadata" });
      } else if (e.target.id === "cmp-copy") {
        PROV.tsv(tsvText(), { table: "metadata" }).then(tx => navigator.clipboard ? navigator.clipboard.writeText(tx) : Promise.reject()).then(() => msg(box, t("meta.copied")), () => msg(box, t("meta.copy.fail"), true));
      }
    };
  }

  if (typeof STAT !== "undefined") STAT.add({ id: "meta", title: "meta.title", visible: () => true, mount });
  Object.assign(CAMPIONI, { mount, paint });
})();
