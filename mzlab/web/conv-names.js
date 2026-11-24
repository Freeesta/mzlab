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
    // a maker's file says its file and sample number and the engine settles its default name (`file`); a file with no reader (a plain mzML) is written as it is listed (`plain`)
    items.push(m ? { who: id == null ? who : who + "@" + id, file: who, id, sample } : { who, plain: name, id, sample });
  }
  let r = null;
  try { r = await fetch("api/samples/mzml_names", { method: "POST", body: JSON.stringify({ files: items }) }).then(x => x.json()); } catch (_) { r = null; }
  if (!r || !Array.isArray(r.files) || r.files.length !== list.length) { conv.notes = [I18N.t("conv.nameby.fail")]; return false; }
  list.forEach((n, i) => { conv.renames.set(n, r.files[i]); if (r.files[i].note) conv.notes.push(r.files[i].note); });
  return true;
}

// the notes under the choice (in place: the table is not drawn again)
function paint(/** @type {any} */ conv) {
  const u = document.getElementById("cvnotes"); if (!u) return;
  u.replaceChildren(...conv.notes.map((/** @type {string} */ t) => { const li = document.createElement("li"); li.textContent = t; return li; }));
  u.hidden = !conv.notes.length;
}

/** @type {any} */ (window).CONVNAMES = { ask, paint };
