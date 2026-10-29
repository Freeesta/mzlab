// Node check of MAPMARGINI (mzlab/web/mappa-margini.js): the margin canvases of the map follow their OWN box. Fake DOM: a canvas whose box changes while the map canvas does not,
// one that goes from hidden (0 x 0) to shown, a panel animated by a FLIP scale (the rect lies, the layout box does not), and a change of the pixel ratio. Exits 1 on the first failure.
const fs = require("fs"), assert = require("assert");
const src = fs.readFileSync(process.argv[2], "utf8").replace("const MAPMARGINI", "var MAPMARGINI");
// fake frame clock: FRAME.ro registers a callback per element; settle() delivers the pending resize notifications like a ResizeObserver does before paint
const subs = new Map(), FRAME = { ro(el, fn) { if (!subs.has(el)) subs.set(el, new Set()); subs.get(el).add(fn); return () => subs.get(el).delete(fn); } };
const settle = () => { for (const [el, fs_] of subs) { const k = el.box.width + "x" + el.box.height; if (el._seen !== k) { el._seen = k; for (const f of [...fs_]) f(el); } } };
// matchMedia: one listener per query; fire() plays the change event of the resolution query
let mqls = [], dpr = 1; const matchMedia = q => { const m = { q, ls: [], addEventListener(_, f) { this.ls.push(f); } }; mqls.push(m); return m; };
const fire = () => { const l = mqls; mqls = []; for (const m of l) for (const f of m.ls) f(); };
const M = new Function("FRAME", "matchMedia", "devicePixelRatio", src + ";return MAPMARGINI")(FRAME, matchMedia, 1);
const canvas = (w, h) => ({ width: 300, height: 150, box: { width: w, height: h }, ctx: { calls: [], setTransform(...a) { this.calls.push(["t", ...a]); }, clearRect(...a) { this.calls.push(["c", ...a]); } },
  scale: 1, get clientWidth() { return Math.round(this.box.width); }, get clientHeight() { return Math.round(this.box.height); }, get offsetWidth() { return this.clientWidth; }, get offsetHeight() { return this.clientHeight; }, clientLeft: 0, clientTop: 0,
  getBoundingClientRect() { return { left: this.left || 0, top: this.top || 0, width: this.box.width * this.scale, height: this.box.height * this.scale }; }, getContext() { return this.ctx; } });

const cv = canvas(600, 300), sp = canvas(180, 300), xc = canvas(0, 0);               // xc is hidden: 0 x 0
let draws = [];
const redraw = () => { for (const c of [sp, xc]) { const r = M.sized(c, 2); draws.push({ c, W: r.W, H: r.H }); } };
const stop = M.watch([sp, xc], redraw);
settle(); draws = [];
// 1) xc is shown: 0 x 0 -> 385 x 140, the map canvas keeps its box
xc.box = { width: 385, height: 140 }; settle();
assert(draws.length >= 1, "no redraw when a hidden canvas is shown");
assert.strictEqual(xc.width, 770); assert.strictEqual(xc.height, 280);               // rect * dpr, no 300 x 150 left
assert.deepStrictEqual(draws.filter(d => d.c === xc).map(d => [d.W, d.H]).pop(), [385, 140]);   // drawing code receives the measured H
// 2) only the box of xc changes (the panel grows): bitmap follows, the drawing H is the new one; cv is untouched
draws = []; xc.box = { width: 385, height: 230 }; settle();
assert.strictEqual(xc.height, 460); assert.strictEqual(xc.width / xc.box.width, xc.height / xc.box.height);
assert.deepStrictEqual(draws.filter(d => d.c === xc).map(d => [d.W, d.H]).pop(), [385, 230]);
assert.strictEqual(cv.width, 300);                                                    // the map canvas was never touched
// 3) the side spectrum alone changes
draws = []; sp.box = { width: 180, height: 410 }; settle();
assert.strictEqual(sp.height, 820); assert.strictEqual(sp.width, 360);
// 4) bitmap only reallocated when the size changed; the context is in CSS pixels (dpr transform) and cleared with the measured box
const before = xc.height; xc.ctx.calls.length = 0; M.sized(xc, 2);
assert.strictEqual(xc.height, before); assert.deepStrictEqual(xc.ctx.calls, [["t", 2, 0, 0, 2, 0, 0], ["c", 0, 0, 385, 230]]);
M.sized(xc, 9); assert.strictEqual(xc.width, 385 * 4);                                // pixel ratio capped at 4
// 5) a FLIP scale on the panel (transizioni.js): the rect is 0.62 x the layout box, the bitmap must still be measured on the layout box and rel() divides the transform out
xc.box = { width: 385, height: 140 }; settle(); xc.scale = .62; sp.scale = .62; cv.scale = .62; cv.left = 10 * .62; cv.top = 40 * .62; xc.left = 0; xc.top = 300 * .62; sp.left = 400 * .62; sp.top = 0; cv.box = { width: 380, height: 300 };
const sc = M.sized(xc, 1); assert.deepStrictEqual([sc.W, sc.H, xc.width, xc.height], [385, 140, 385, 140]);
const o = M.rel(cv, xc); assert(Math.abs(o.x - 10) < 1e-9 && Math.abs(o.y - (40 - 300)) < 1e-9, JSON.stringify(o));
xc.scale = sp.scale = cv.scale = 1;
// 6) the pixel ratio changes (window zoom): redraw, bitmap = layout box x new ratio, and the watcher arms itself again for the next change
draws = []; fire(); assert(draws.length >= 2, "no redraw on a change of the pixel ratio"); fire(); assert(draws.length >= 4, "the resolution watcher was not re-armed");
// 7) after stop() nothing is redrawn
stop(); draws = []; xc.box = { width: 100, height: 100 }; settle(); fire(); assert.strictEqual(draws.length, 0);
console.log("ok");
