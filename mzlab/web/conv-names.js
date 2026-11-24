// @ts-check
// «Name files by: sample name» of the mzML to save, loaded on its first use (CONV.syncNames in conversione.js). The page only tells the engine, for each file, the file it came from, the sample
// name the reader reports and the sample number; the cleaning, the clashes and the notes are the Rust of `mzlab_core::samples::mzml_names` (route `api/samples/mzml_names`, the function of
// `mzlab-cli file convert --name sample`). Each automatic rename comes back as a note and is shown as it is.
async function ask(/** @type {any} */ conv, /** @type {string[]} */ list) {
  const rb = window.MZLAB_RUST, sets = new Map(), items = [];
  for (const name of list) {
    const m = conv.meta.get(name);
    let who = name, sample = null, id = null;
    if (m && rb && rb.nativeSamples) {
      const key = m.members.map(x => x.name + ":" + x.blob.size).join("|");
      if (!sets.has(key)) sets.set(key, rb.nativeSamples(m.members, true).then((/** @type {Bag} */ j) => j.samples || []).catch(() => []));
      const all = await sets.get(key), s = all.find((/** @type {Bag} */ x) => x.id === m.id);
      who = m.members[0].name.split("/")[0];
      if (s) { sample = s.name; if (all.length > 1) id = m.id; }
    }
    items.push({ who, id, sample, plain: name });
  }
  // the default name of the CLI for a maker's file (NAME.mzML, NAME-ID.mzML, the extension kept when two would be equal): what a file with no usable sample name is written as
  const cut = (/** @type {string} */ n) => n.replace(/\.gz$/i, ""), stem = (/** @type {string} */ n) => { const g = cut(n), i = g.lastIndexOf("."); return i < 0 ? g : g.slice(0, i); };
  const short = (/** @type {Bag} */ x) => stem(x.who) + (x.id == null ? "" : "-" + x.id) + ".mzML", own = items.filter(x => conv.meta.has(x.plain));
  for (const x of own) x.plain = (own.filter(y => short(y) === short(x)).length > 1 ? cut(x.who) : stem(x.who)) + (x.id == null ? "" : "-" + x.id) + ".mzML";
  let r = null;
  try { r = await fetch("api/samples/mzml_names", { method: "POST", body: JSON.stringify({ files: items.map(x => ({ who: x.id == null ? x.who : x.who + "@" + x.id, plain: x.plain, sample: x.sample, id: x.id })) }) }).then(x => x.json()); } catch (_) { r = null; }
  if (!r || !Array.isArray(r.files) || r.files.length !== list.length) { conv.notes = [I18N.t("conv.nameby.fail")]; return; }
  list.forEach((n, i) => { conv.renames.set(n, r.files[i]); if (r.files[i].note) conv.notes.push(r.files[i].note); });
}

// the notes under the choice (in place: the table is not drawn again)
function paint(/** @type {any} */ conv) {
  const u = document.getElementById("cvnotes"); if (!u) return;
  u.replaceChildren(...conv.notes.map((/** @type {string} */ t) => { const li = document.createElement("li"); li.textContent = t; return li; }));
  u.hidden = !conv.notes.length;
}

/** @type {any} */ (window).CONVNAMES = { ask, paint };
