// @ts-check
// the panel of a conversion asked from the table («Download .mzML» of a maker's sample, CONV.make), inside `box`: "File 1 of 2: name", a button to cancel, the phase as text
// ("Converting to mzML", then "Indexing": g.index), ONE progress bar (role=progressbar) over the whole job (the conversion counts 3/4 of it, the indexing 1/4), and the numbers (scans,
// MB written, time elapsed: no estimate of the time left).
// Each file has its own stop in `stops` (ConvUI.stop, × on its side row). The digits are tabular and padded; after 2 s without news it says it is still
// working (animated dot, still under reduced motion), after 30 s it adds advice; the panel goes when the job ends.
const CONV_W = 0.75;         // share of the bar taken by the conversion; the rest is the indexing
const FIG = "\u2007";        // figure space: as wide as a digit
const convPad = (/** @type {string | number | null} */ s, /** @type {number} */ w) => String(s).padStart(w, FIG);
const CONV_CARD = `<div class="cp-head"><b data-r="file"></b> <button type="button" class="sec sm" data-r="cancel"></button></div>
      <div class="cp-phase"><span data-r="phase" role="status" aria-live="polite"></span> <span class="cp-beat muted" data-r="beat" role="status" aria-live="polite"></span></div>
      <div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax="100" data-r="bar"><i></i></div><div class="muted sm cp-num" data-r="detail"></div>`;
function group(/** @type {HTMLElement} */ box, /** @type {number[]} */ sizes, /** @type {Map<string, AbortController>} */ stops) {
  /** @param {string} k @param {Bag} [p] */ const T = (k, p) => I18N.t(k, p), n = sizes.length, ctl = new AbortController(), t0 = Date.now();
  const el = document.createElement("div"); el.className = "convprog"; el.setAttribute("role", "group"); el.setAttribute("aria-label", T("conv.aria"));
  el.innerHTML = CONV_CARD;
  const q = (/** @type {string} */ s) => el.querySelector(`[data-r=${s}]`);
  const cancel = q("cancel"); cancel.textContent = T("conv.cancel"); cancel.title = cancel.ariaLabel = T("conv.cancel.title");
  cancel.onclick = () => { cancel.disabled = true; q("phase").textContent = T("conv.cancelling"); ctl.abort(); };
  box.appendChild(el);
  const bar = q("bar"), fill = bar.querySelector("i");
  const setBar = (/** @type {number | null | undefined} */ pc) => { if (pc == null) { bar.removeAttribute("aria-valuenow"); fill.style.transform = "scaleX(0)"; } else { bar.setAttribute("aria-valuenow", String(pc)); fill.style.transform = `scaleX(${pc / 100})`; } };
  let beatStage = "", cur = -1, /** @type {Bag | null} */ last = null, doneFrac = 0, stage = "converting", beat = "", changed = t0, key = "", phaseKey = "", indexed = new Map(), nIdx = 0, /** @type {any} */ timer = null, maxPc = 0;
  const idxMean = () => nIdx ? [...indexed.values()].reduce((a, b) => a + b, 0) / (100 * nIdx) : 0;
  const show = () => {
    const now = Date.now(), elapsed = convElapsed(now - t0), conv = stage === "indexing" ? 100 : convTotalPercent(sizes, Math.max(0, cur), convFraction(last));
    const known = last && (last.copied != null || last.scansTotal); let pc = (conv == null || !known && cur <= 0) && stage === "converting" ? null : Math.min(100, Math.floor(CONV_W * (conv || 0) + (1 - CONV_W) * 100 * idxMean()));
    // forward only and never 100 % before `end()`: the same rules as PROGRESSO
    if (pc != null) { maxPc = Math.max(maxPc, Math.min(99, pc)); pc = maxPc; }
    const lab = T(stage === "indexing" ? "conv.phase.indexing" : "conv.phase.converting");
    if (lab !== phaseKey && !cancel.disabled) { phaseKey = lab; q("phase").textContent = lab; }
    bar.setAttribute("aria-label", lab); setBar(pc);
    let d;
    // one count and one clock for the whole loading: the same line as the group over the table
    if (stage === "indexing") d = CONVROWS.rows.length ? T("conv.detail.elapsed", { elapsed }) : T("conv.detail.indexed", { done: [...indexed.values()].filter(v => v >= 100).length, total: nIdx, elapsed });
    else if (last && last.copied != null) d = T("conv.detail.handover", { done: convMB(last.copied), total: convMB(last.size), elapsed });
    else if (last && last.scansTotal) {
      const w = String(Math.round(last.scansTotal)).length, fp = convPercent(last);
      const parts = [convPad(fp, 3) + "%", ...(last.several ? [] : [T("conv.detail.scans", { done: convPad(I18N.fix(last.scans, 0), w + Math.floor((w - 1) / 3)), total: I18N.fix(last.scansTotal, 0) })]),
        T("conv.detail.written", { mb: convPad(convMB(last.written), 6) }), T("conv.detail.elapsed", { elapsed: convPad(elapsed, 5) })];
      d = parts.join(" · ");
    } else d = T("conv.detail.wait", { elapsed });
    q("detail").textContent = d;
    const quiet = now - changed, b = quiet >= 30000 ? "stall" : quiet >= 2000 ? "beat" : "";
    if (b !== beat || b === "beat" && beatStage !== stage) { beatStage = stage; beat = b; q("beat").innerHTML = b ? `<span class="cp-spin" aria-hidden="true"></span> <span data-r="beattxt"></span>` : ""; if (b) q("beattxt").textContent = T(b === "stall" ? "conv.stall" : stage === "indexing" ? "conv.beat.indexing" : "conv.beat"); }
  };
  // a late index() or update() after end() starts no clock again
  let ended = false;
  const tick = () => { if (!timer && !ended) timer = setInterval(show, 1000); };
  const touch = (/** @type {string} */ k) => { if (k !== key) { key = k; changed = Date.now(); } };
  show();
  return {
    signal: ctl.signal,
    start(/** @type {number} */ i, /** @type {string} */ name) {
      cur = i - 1; last = null; stage = "converting";
      q("file").textContent = T("conv.file", { i, n, name });
      tick(); show();
      const c = new AbortController(), off = () => { if (stops.get(name) === c) stops.delete(name); };
      ctl.signal.addEventListener("abort", () => c.abort()); stops.set(name, c);
      return {
        signal: c.signal,
        update(/** @type {Bag} */ p) { touch([p.scans, p.read, p.written, p.copied].join(",")); last = p; show(); },
        // replaces "File i of n: name" (conversions running together: "Samples converted: 3 of 23")
        title(/** @type {string} */ text) { q("file").textContent = text; },
        done() { off(); doneFrac = (cur + 1) / n; },
        fail(/** @type {string} */ msg) { off(); clearInterval(timer); timer = null; el.remove(); const f = document.createElement("div"); f.className = "fail"; f.textContent = name + ": " + msg; box.appendChild(f); },
      };
    },
    // the page builds the index of each converted file (index.html upload): `pc` 0..100 of `name`; the bar goes on from where the conversion left it
    index(/** @type {string} */ name, /** @type {number} */ pc) {
      if (!indexed.has(name)) nIdx++;
      // never backwards: the builder reads the file more than once
      indexed.set(name, Math.max(indexed.get(name) || 0, pc));
      if (stage !== "indexing") { stage = "indexing"; q("file").textContent = ""; cancel.style.visibility = "hidden"; }
      touch("idx" + [...indexed.values()].join(","));
      tick(); show();
    },
    // nothing stays on screen when the job is over
    end() { ended = true; clearInterval(timer); timer = null; el.remove(); },
  };
}

/** @type {any} */ (window).CONVCARD = group;
