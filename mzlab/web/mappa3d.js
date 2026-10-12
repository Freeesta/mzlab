// RT x m/z map in 3D: a full-screen viewer (modal dialog) written by hand in WebGL2, no libraries. Opened by «Apri in 3D» of the 2D map panel
// (mappa.js), with the file and the region of that panel; the panel itself stays 2D. Four representations of the same data: surface, sticks
// (one segment per true point, high resolution centroids), waterfall of spectra stacked along RT, contour lines on the floor.
// The data come from /api/map (grid, max per cell) and /api/mappunti3d (points); the mouse ray meets the height grid on the CPU.
// Only measures are shown here: no automatic labels, no candidates.
const T = (k, o) => I18N.t(k, o);
const PREF_KEY = "d3";                                   // inside MAPPA.prefs ("qqq.mappa")
const REDUCED = () => !!(window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches);
const SMALL = () => !!(window.MZLAB_PHONE || (matchMedia("(pointer:coarse)").matches && Math.min(screen.width, screen.height) < 900) || (navigator.deviceMemory && navigator.deviceMemory <= 4));
const GRID = () => SMALL() ? 512 : 1024;                 // cells per side
const STICKS = () => SMALL() ? 50000 : 200000;           // most points drawn as sticks
const DEF = { mode: "surf", hs: "lin", ex: 1, asp: 1.6, lt: true, fl: true, sig: null, side: true };

// ---------------------------------------------------------------- small maths (column-major 4x4)
const m4 = {
  mul(a, b) { const o = new Float32Array(16); for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) { let s = 0; for (let k = 0; k < 4; k++) s += a[k * 4 + r] * b[c * 4 + k]; o[c * 4 + r] = s; } return o; },
  persp(fov, asp, n, f) { const t = 1 / Math.tan(fov / 2), o = new Float32Array(16); o[0] = t / asp; o[5] = t; o[10] = (f + n) / (n - f); o[11] = -1; o[14] = 2 * f * n / (n - f); return o; },
  look(e, c, u) {
    const f = norm3(sub3(c, e)), s = norm3(cross3(f, u)), v = cross3(s, f), o = new Float32Array(16);
    o[0] = s[0]; o[4] = s[1]; o[8] = s[2]; o[1] = v[0]; o[5] = v[1]; o[9] = v[2]; o[2] = -f[0]; o[6] = -f[1]; o[10] = -f[2];
    o[12] = -(s[0] * e[0] + s[1] * e[1] + s[2] * e[2]); o[13] = -(v[0] * e[0] + v[1] * e[1] + v[2] * e[2]); o[14] = f[0] * e[0] + f[1] * e[1] + f[2] * e[2]; o[15] = 1; return o;
  }
};
function sub3(a, b) { return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]; }
function cross3(a, b) { return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]; }
function norm3(a) { const l = Math.hypot(a[0], a[1], a[2]) || 1; return [a[0] / l, a[1] / l, a[2] / l]; }
const clamp = (v, a, b) => Math.min(b, Math.max(a, v));
const b64f = (s, Cls) => { const bin = atob(s), u = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i); return new Cls(u.buffer); };

// ---------------------------------------------------------------- shaders
const PRE = "#version 300 es\nprecision highp float;precision highp int;precision highp sampler2D;\n";
const SURF_VS = PRE + `
uniform sampler2D uH; uniform ivec2 uN; uniform mat4 uVP; uniform vec3 uBox; uniform float uSigned;
out float vZ; out vec3 vN; out vec3 vW;
float h(int i, int j) { return texelFetch(uH, ivec2(clamp(j, 0, uN.y - 1), clamp(i, 0, uN.x - 1)), 0).r; }
void main() {
  int cell = gl_VertexID / 6, k = gl_VertexID - cell * 6, cw = uN.y - 1;
  int ci = cell / cw, cj = cell - ci * cw;
  int di = (k == 1 || k == 4 || k == 5) ? 1 : 0, dj = (k == 2 || k == 3 || k == 5) ? 1 : 0;
  int i = ci + di, j = cj + dj;
  float z = h(i, j);
  float x = (float(i) / float(uN.x - 1) - 0.5) * uBox.x, zz = (0.5 - float(j) / float(uN.y - 1)) * uBox.z;
  float dx = uBox.x / float(uN.x - 1) * 2.0, dz = uBox.z / float(uN.y - 1) * 2.0;
  float gx = (h(i + 1, j) - h(i - 1, j)) * uBox.y / dx, gz = -(h(i, j + 1) - h(i, j - 1)) * uBox.y / dz;
  vN = normalize(vec3(-gx, 1.0, -gz)); vZ = z; vW = vec3(x, z * uBox.y, zz);
  gl_Position = uVP * vec4(x, z * uBox.y, zz, 1.0);
}`;
const SURF_FS = PRE + `
uniform sampler2D uPal; uniform float uSigned, uLight; uniform vec3 uEye;
in float vZ; in vec3 vN; in vec3 vW; out vec4 o;
void main() {
  float t = uSigned > 0.5 ? 0.5 - 0.5 * clamp(vZ, -1.0, 1.0) : clamp(vZ, 0.0, 1.0);
  vec3 c = texture(uPal, vec2(t, 0.5)).rgb;
  if (uLight > 0.5) { vec3 L = normalize(uEye - vW); float d = max(dot(normalize(vN), L), 0.0); c *= 0.42 + 0.72 * d; }
  o = vec4(c, 1.0);
}`;
const FLOOR_VS = PRE + `
uniform mat4 uVP; uniform vec3 uBox; out vec2 vUV;
void main() {
  int k = gl_VertexID; float a = (k == 1 || k == 4 || k == 5) ? 1.0 : 0.0, b = (k == 2 || k == 3 || k == 5) ? 1.0 : 0.0;
  vUV = vec2(a, b);
  gl_Position = uVP * vec4((a - 0.5) * uBox.x, -0.0008, (0.5 - b) * uBox.z, 1.0);
}`;
const FLOOR_FS = PRE + `
uniform sampler2D uMap; uniform float uUse; uniform vec4 uCol; in vec2 vUV; out vec4 o;
void main() { o = uUse > 0.5 ? vec4(texture(uMap, vUV).rgb, 1.0) : uCol; }`;
const STICK_VS = PRE + `
uniform sampler2D uPal; uniform mat4 uVP; uniform vec3 uBox; uniform float uSigned;
in vec3 aP; out vec3 vC;
void main() {
  float top = float(gl_VertexID & 1);
  vC = texture(uPal, vec2(clamp(aP.z, 0.0, 1.0), 0.5)).rgb;
  gl_Position = uVP * vec4((aP.x - 0.5) * uBox.x, aP.z * uBox.y * top, (0.5 - aP.y) * uBox.z, 1.0);
}`;
const STICK_FS = PRE + `in vec3 vC; out vec4 o; void main() { o = vec4(vC, 1.0); }`;
const ROW_VS = PRE + `
uniform sampler2D uH; uniform sampler2D uPal; uniform ivec2 uN; uniform mat4 uVP; uniform vec3 uBox; uniform int uRow, uMode; uniform float uSigned, uLift;
out vec3 vC;
void main() {
  int j = uMode == 0 ? gl_VertexID / 2 : gl_VertexID; bool top = uMode != 0 || (gl_VertexID & 1) == 0;
  float z = texelFetch(uH, ivec2(j, uRow), 0).r;
  float t = uSigned > 0.5 ? 0.5 - 0.5 * clamp(z, -1.0, 1.0) : clamp(z, 0.0, 1.0);
  vC = texture(uPal, vec2(t, 0.5)).rgb;
  gl_Position = uVP * vec4((float(uRow) / float(uN.x - 1) - 0.5) * uBox.x, (top ? z * uBox.y : 0.0) + uLift, (0.5 - float(j) / float(uN.y - 1)) * uBox.z, 1.0);
}`;
const ROW_FS = PRE + `uniform int uMode; uniform vec3 uBg; uniform vec4 uFlat; in vec3 vC; out vec4 o;
void main() { if (uFlat.a > 0.0) o = uFlat; else if (uMode == 0) o = vec4(mix(uBg, vC, 0.16), 1.0); else o = vec4(vC, 1.0); }`;
const LINE_VS = PRE + `uniform mat4 uVP; in vec3 aP; in vec3 aC; out vec3 vC; void main() { vC = aC; gl_Position = uVP * vec4(aP, 1.0); }`;
const LINE_FS = PRE + `uniform float uA; in vec3 vC; out vec4 o; void main() { o = vec4(vC, uA); }`;

function prog(gl, vs, fs) {
  const mk = (t, s) => { const x = gl.createShader(t); gl.shaderSource(x, s); gl.compileShader(x); if (!gl.getShaderParameter(x, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(x)); return x; };
  const p = gl.createProgram(); gl.attachShader(p, mk(gl.VERTEX_SHADER, vs)); gl.attachShader(p, mk(gl.FRAGMENT_SHADER, fs)); gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
  const u = {}, n = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS);
  for (let i = 0; i < n; i++) { const a = gl.getActiveUniform(p, i); u[a.name] = gl.getUniformLocation(p, a.name); }
  return { p, u };
}

// ---------------------------------------------------------------- the viewer
const CSS = `
dialog.m3d{position:fixed;inset:0;width:100vw;height:100vh;max-width:none;max-height:none;margin:0;padding:0;border:0;background:var(--bg);color:var(--ink);overflow:hidden}
dialog.m3d[open]{display:flex}dialog.m3d::backdrop{background:rgba(0,0,0,.5)}
.m3d-main{position:relative;flex:1 1 0;min-width:0;height:100%;background:var(--panel)}
.m3d-main canvas.m3d-gl,.m3d-main canvas.m3d-ov{position:absolute;inset:0;width:100%;height:100%;display:block}.m3d-ov{pointer-events:none}.m3d-gl{touch-action:none;cursor:grab}.m3d-gl:active{cursor:grabbing}
.m3d-gl:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}
.m3d-read{position:absolute;left:12px;top:8px;right:12px;font-size:12px;font-variant-numeric:tabular-nums;color:var(--ink);pointer-events:none;text-shadow:0 0 3px var(--panel),0 0 3px var(--panel)}
.m3d-side>.mznav{display:flex;gap:2px;margin:0 0 6px}.m3d-help{position:absolute;left:12px;bottom:6px;font-size:11px;color:var(--muted);max-width:55%;pointer-events:none;text-shadow:0 0 3px var(--panel)}
.m3d-mini{position:absolute;right:10px;bottom:46px;display:flex;flex-direction:column;gap:6px;pointer-events:none}
.m3d-mini canvas{width:260px;height:104px;background:color-mix(in srgb,var(--panel) 88%,transparent);border:1px solid var(--line);border-radius:6px}
.m3d-time{position:absolute;left:12px;right:290px;bottom:26px;display:flex;align-items:center;gap:8px;font-size:12px;color:var(--muted)}.m3d-time input[type=range]{flex:1 1 0}
.m3d-side{flex:none;width:320px;height:100%;overflow:auto;border-left:1px solid var(--line);background:var(--bg);padding:8px 10px;font-size:12px;box-sizing:border-box}
dialog.m3d.hid .m3d-side{display:none}
.m3d-side details{border-bottom:1px solid var(--line);padding:4px 0}.m3d-side summary{cursor:pointer;font-weight:700;padding:4px 0}
.m3d-side .rw{display:flex;gap:6px;align-items:center;flex-wrap:wrap;margin:4px 0}.m3d-side .rw>label{display:inline-flex;gap:4px;align-items:center;white-space:nowrap}
.m3d-side select{max-width:170px}.m3d-side input.mzf{width:70px}.m3d-side input[type=range]{width:120px}
.m3d-top{display:flex;gap:6px;align-items:center;justify-content:space-between;margin-bottom:6px}
.m3d-hist{width:100%;height:46px;border:1px solid var(--line);border-radius:4px;background:var(--panel);cursor:crosshair}
.m3d-tog{position:absolute;right:8px;top:6px;z-index:3}
.m3d-msg{position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);padding:10px 14px;border:1px solid var(--line);background:var(--panel);border-radius:8px;text-align:center;max-width:70%}
@media (prefers-reduced-motion:reduce){.m3d *{transition:none!important;animation:none!important}}
`;

// Why the viewer could not open, said as it is: no WebGL2 at all | the shaders did not compile | a JavaScript error | the context was lost. `diag` collects what the browser says
// (version, renderer, attributes, the first error) for the dialog and the console; nothing here is guessed.
export class Fail extends Error { constructor(kind, detail, diag) { super(`${kind}: ${detail}`); this.kind = kind; this.detail = detail; this.diag = diag || {}; } }
const WEBGL_TRIES = [{ antialias: true, alpha: false }, { antialias: false, alpha: false }, {}, { powerPreference: "low-power" }];
// getContext("webgl2") with the options that the viewer prefers, then without them; the canvas is never one that already has another kind of context
export function makeContext(canvas, tries = WEBGL_TRIES) {
  const errs = [];
  for (const o of tries) {
    try { const g = canvas.getContext("webgl2", o); if (g) return { gl: g, opts: o, errs }; } catch (e) { errs.push(String(e && e.message || e)); }
  }
  return { gl: null, errs };
}
export function diagnose(gl, extra) {
  const d = { ...extra };
  try { d.version = gl.getParameter(gl.VERSION); d.renderer = gl.getParameter(gl.RENDERER); } catch (e) { d.version = "?"; }
  try { const x = gl.getExtension("WEBGL_debug_renderer_info"); if (x) d.gpu = gl.getParameter(x.UNMASKED_RENDERER_WEBGL); } catch (e) { /* not given */ }
  try { d.attributes = gl.getContextAttributes(); d.error = gl.getError(); d.lost = gl.isContextLost(); } catch (e) { /* context gone */ }
  return d;
}
// the markup of the dialog: the viewer on the left, the sidebar with the groups of controls on the right (the handlers are bound in open())
function dialogHtml({ f, lv, hr, files, others, V, P, prefs, opt }) {
  return `<div class="m3d-main"><canvas class="m3d-gl" tabindex="0" aria-label="${T("m3d.canvas")}"></canvas><canvas class="m3d-ov" aria-hidden="true"></canvas>
<div class="m3d-read" aria-live="polite"></div><div class="m3d-help">${T("m3d.help")}</div>
<div class="m3d-mini"><canvas class="m3d-xic" aria-label="${T("m3d.xic.aria")}"></canvas><canvas class="m3d-spc" aria-label="${T("m3d.spc.aria")}"></canvas></div>
<div class="m3d-time"><button data-m="play" aria-label="${T("m3d.play")}" title="${T("m3d.play")}">&#9654;</button><input type="range" data-m="ti" min="0" max="0" value="0" aria-label="${T("m3d.time.aria")}"><span data-m="tt"></span></div>
<button class="m3d-tog" data-m="tog" aria-label="${T("m3d.side.toggle")}" title="${T("m3d.side.toggle")}">&#9776;</button></div>
<aside class="m3d-side" aria-label="${T("m3d.side.aria")}"><div class="m3d-top"><b>${T("m3d.title")}</b><button data-m="close" aria-label="${T("m3d.close")}">${T("m3d.close")} (Esc)</button></div>
<details open><summary>${T("m3d.g.data")}</summary>
<div class="rw"><label>${T("m3d.file")} <select data-m="file" aria-label="${T("m3d.file")}">${files.map(x => opt(x.k, f.k, x.label)).join("")}</select></label><span class="muted">MS<sup>${lv}</sup></span></div>
<div class="rw"><label>${T("m3d.diff")} <select data-m="ref" aria-label="${T("m3d.diff")}"><option value="">${T("ctl.map.diff.none")}</option>${others.map(x => opt(x.k, V.ref, x.label)).join("")}</select></label>
<select data-m="norm" aria-label="${T("ctl.map.norm.title")}">${opt("abs", V.norm, T("ctl.map.norm.abs"))}${opt("max", V.norm, T("ctl.map.norm.max"))}${opt("tic", V.norm, T("ctl.map.norm.tic"))}</select></div>
<div class="rw"><label>RT <input class="mzf" data-m="rt0" inputmode="decimal" aria-label="RT ${T("ctl.range.from")}"> ${T("ctl.range.to")} <input class="mzf" data-m="rt1" inputmode="decimal" aria-label="RT ${T("ctl.range.to")}"> min</label></div>
<div class="rw"><label><i>m/z</i> <input class="mzf" data-m="mz0" inputmode="decimal" aria-label="m/z ${T("ctl.range.from")}"> ${T("ctl.range.to")} <input class="mzf" data-m="mz1" inputmode="decimal" aria-label="m/z ${T("ctl.range.to")}"></label></div>
<div class="rw"><button data-m="apply">${T("m3d.apply")}</button><button data-m="sync" title="${T("m3d.sync.title")}">${T("m3d.sync")}</button></div>
<div class="rw muted" data-m="res" aria-live="polite"></div></details>
<details open><summary>${T("m3d.g.rep")}</summary>
<div class="rw"><label>${T("m3d.mode")} <select data-m="mode" aria-label="${T("m3d.mode")}">${opt("surf", P.mode, T("m3d.mode.surf"))}${opt("sticks", P.mode, T("m3d.mode.sticks"), !hr)}${opt("fall", P.mode, T("m3d.mode.fall"))}${opt("cont", P.mode, T("m3d.mode.cont"))}</select></label></div>
<div class="rw"><label>${T("m3d.h")} <select data-m="hs" aria-label="${T("m3d.h")}">${opt("lin", P.hs, T("m3d.h.lin"))}${opt("sqrt", P.hs, T("m3d.h.sqrt"))}${opt("log", P.hs, T("m3d.h.log"))}</select></label></div>
<div class="rw"><label>${T("m3d.ex")} <input type="range" data-m="ex" min="0.2" max="4" step="0.1" value="${P.ex}" aria-label="${T("m3d.ex")}"></label></div>
<div class="rw"><label>${T("m3d.asp")} <input type="range" data-m="asp" min="0.5" max="3" step="0.1" value="${P.asp}" aria-label="${T("m3d.asp")}"></label></div>
<div class="rw"><label>${T("m3d.pal")} <select data-m="pal" aria-label="${T("m3d.pal")}">${opt("viridis", mapPal(), T("mappa.pal.viridis"))}${opt("cividis", mapPal(), T("mappa.pal.cividis"))}</select></label></div>
<div class="rw"><label><input type="checkbox" data-m="lt"${P.lt ? " checked" : ""}> ${T("m3d.light")}</label><label><input type="checkbox" data-m="fl"${P.fl ? " checked" : ""}> ${T("m3d.floor")}</label><label><input type="checkbox" data-m="pl"> ${T("m3d.plane")}</label></div></details>
<details open><summary>${T("m3d.g.bg")}</summary>
<div class="rw"><label>${T("mappa.thr")} <input class="mzf" data-m="thr" inputmode="decimal" aria-label="${T("mappa.thr.title")}"> %</label><label><input type="checkbox" data-m="bg"${prefs.bg ? " checked" : ""}> ${T("mappa.bg")}</label></div>
<canvas class="m3d-hist" data-m="hist" aria-label="${T("m3d.hist.aria")}" title="${T("m3d.hist.title")}"></canvas>
<div class="rw"><label>${T("m3d.sig")} <input type="range" data-m="sig" min="0" max="4" step="0.5" value="${P.sig}" aria-label="${T("m3d.sig")}"></label></div></details>
<details open><summary>${T("m3d.g.views")}</summary>
<div class="rw"><button data-m="v1" title="${T("m3d.v.top.title")}">1 ${T("m3d.v.top")}</button><button data-m="v2" title="${T("m3d.v.rt.title")}">2 ${T("m3d.v.rt")}</button><button data-m="v3" title="${T("m3d.v.mz.title")}">3 ${T("m3d.v.mz")}</button><button data-m="v4">4 ${T("m3d.v.iso")}</button></div>
<div class="rw"><button data-m="reset" title="R">${T("m3d.reset")} (R)</button><button data-m="png">${T("m3d.png")}</button></div></details></aside>`;
}

function blur(Z, nx, ny, s) {
  const r = Math.ceil(3 * s), k = new Float32Array(2 * r + 1); let sum = 0;
  for (let i = -r; i <= r; i++) { k[i + r] = Math.exp(-i * i / (2 * s * s)); sum += k[i + r]; } for (let i = 0; i < k.length; i++) k[i] /= sum;
  const t = new Float32Array(Z.length), o = new Float32Array(Z.length);
  for (let i = 0; i < nx; i++) for (let j = 0; j < ny; j++) { let acc = 0; for (let d = -r; d <= r; d++) acc += k[d + r] * Z[i * ny + clamp(j + d, 0, ny - 1)]; t[i * ny + j] = acc; }
  for (let i = 0; i < nx; i++) for (let j = 0; j < ny; j++) { let acc = 0; for (let d = -r; d <= r; d++) acc += k[d + r] * t[clamp(i + d, 0, nx - 1) * ny + j]; o[i * ny + j] = acc; }
  return o;
}

function plot(c, xs, ys, cx, label, xlab, signed) {
  const r = c.getBoundingClientRect(), d = Math.min(2, devicePixelRatio || 1), W = Math.round((r.width || 260) * d), H = Math.round((r.height || 104) * d);
  if (c.width !== W) c.width = W; if (c.height !== H) c.height = H; const g = c.getContext("2d"); g.setTransform(d, 0, 0, d, 0, 0);
  const w = W / d, h = H / d; g.clearRect(0, 0, w, h); g.fillStyle = css("--muted"); g.strokeStyle = css("--muted"); g.font = "10px system-ui, sans-serif"; g.fillText(label, 6, 11);
  const n = ys.length; let mn = 0, mx = 0; for (let i = 0; i < n; i++) { if (ys[i] > mx) mx = ys[i]; if (ys[i] < mn) mn = ys[i]; } if (mx === mn) mx = mn + 1;
  const X = i => 4 + i / (n - 1) * (w - 8), Y = v => h - 14 - (v - mn) / (mx - mn) * (h - 30);
  g.strokeStyle = css("--ink"); g.lineWidth = 1; g.beginPath(); for (let i = 0; i < n; i++) { const x = X(i), y = Y(ys[i]); i ? g.lineTo(x, y) : g.moveTo(x, y); } g.stroke();
  g.strokeStyle = css("--accent"); g.beginPath(); g.moveTo(X(cx), 14); g.lineTo(X(cx), h - 14); g.stroke();
  g.fillStyle = css("--muted"); g.textAlign = "left"; g.fillText(xlab[0], 4, h - 3); g.textAlign = "right"; g.fillText(xlab[1], w - 4, h - 3); g.textAlign = "right"; g.fillText(fmt(mx), w - 4, 11);
}

export function open(p, opener) {
  const a = p._a; if (!a || !a.f) return;
  const probe = document.createElement("canvas"), c0 = makeContext(probe);                      // a throw-away canvas: is there WebGL2 at all? (the context is let go at once)
  if (!c0.gl) { let w1 = false; try { w1 = !!document.createElement("canvas").getContext("webgl"); } catch (e) { /* none */ } throw new Fail("nowebgl2", c0.errs.join("; "), { webgl1: w1 }); }
  try { const x = c0.gl.getExtension("WEBGL_lose_context"); if (x) x.loseContext(); } catch (e) { /* the browser will collect it */ }
  if (!document.getElementById("m3d-css")) { const s = document.createElement("style"); s.id = "m3d-css"; s.textContent = CSS; document.head.appendChild(s); }
  const prefs = MAPPA.prefs, P = Object.assign({}, DEF, prefs[PREF_KEY] || {});
  prefs.bg = p.corr === "auto";                                  // the 3D takes the background of the 2D panel it was opened from (its own switch stays in the sidebar)
  const savePrefs = () => { prefs[PREF_KEY] = { mode: P.mode, hs: P.hs, ex: P.ex, asp: P.asp, lt: P.lt, fl: P.fl, sig: P.sig, side: P.side }; MAPPA.prefSave(); };
  const f = a.f, lv = f.lv || 1, hr = !!(window.HR && HR.isHr(f, lv));
  const V = {                                               // the state of the viewer
    k: f.k, ref: a.ref ? a.ref.k : null, norm: p.norm || "abs", lv, hr, rt: [a.x0, a.x1], mz: [a.y0, a.y1],
    g: null, raw: null, Z: null, zmin: 0, zmax: 1, refv: 1, signed: false, pts: null, cur: null, hover: false, plane: false, playing: 0,
    cam: { az: 35, el: 32, d: 3, t: [0, 0, 0] }, dirty: true, W: 1, H: 1, gen: 0
  };
  if (P.sig == null) P.sig = hr ? 0 : 1;
  const dlg = document.createElement("dialog"); dlg.className = "m3d" + (P.side ? "" : " hid"); dlg.setAttribute("aria-label", T("m3d.title"));
  const opt = (v, cur, lab, dis) => `<option value="${v}"${String(cur) === String(v) ? " selected" : ""}${dis ? " disabled" : ""}>${EH(lab)}</option>`;
  const files = scanFiles(tabFiles(p.tab)).filter(x => x.lv === lv);
  const others = files.filter(x => x.k !== f.k);
  dlg.innerHTML = dialogHtml({ f, lv, hr, files, others, V, P, prefs, opt });
  document.body.appendChild(dlg);
  const Q = k => dlg.querySelector(`[data-m="${k}"]`), glc = dlg.querySelector(".m3d-gl"), ovc = dlg.querySelector(".m3d-ov"), rd = dlg.querySelector(".m3d-read");
  const xicc = dlg.querySelector(".m3d-xic"), spcc = dlg.querySelector(".m3d-spc"), main = dlg.querySelector(".m3d-main"), ov = ovc.getContext("2d");
  const num = el => { const v = numIt(el.value); return Number.isFinite(v) ? v : null; };

  // the most intense cells of the grid, for the «Dati» table (tdati.js)
  if (window.TDATI) TDATI.addButton(glc, () => {
    if (!V.g || !V.raw) return null;
    const { nrt, nmz } = V.g, cells = [];
    for (let i = 0; i < nrt; i++) for (let j = 0; j < nmz; j++) { const v = V.raw[i * nmz + j]; if (v > 0) cells.push([i, j, v]); }
    cells.sort((a, b) => b[2] - a[2]).splice(100);
    return { title: T("m3d.canvas"), head: [T("tdati.col.rt"), "m/z", T("tdati.col.int")], rows: cells.map(c => [TDATI.num(V.rt[0] + (c[0] + 0.5) * (V.rt[1] - V.rt[0]) / nrt, 3), TDATI.num(V.g.log ? V.g.mz0 * Math.exp((c[1] + 0.5) * V.g.dlog) : V.g.mz0 + (c[1] + 0.5) * V.g.dmz, 3), TDATI.num(c[2], 1)]), note: T("tdati.note.map", { n: cells.length }) };
  });
  // ---- WebGL
  const cx = makeContext(glc), gl = cx.gl;
  if (!gl) { dlg.remove(); throw new Fail("nowebgl2", cx.errs.join("; "), {}); }
  let PR;
  try {
    PR = { surf: prog(gl, SURF_VS, SURF_FS), floor: prog(gl, FLOOR_VS, FLOOR_FS), stick: prog(gl, STICK_VS, STICK_FS), row: prog(gl, ROW_VS, ROW_FS), line: prog(gl, LINE_VS, LINE_FS) };
  } catch (e) { const d = diagnose(gl, { opts: cx.opts }); dlg.remove(); throw new Fail("shader", String(e && e.message || e), d); }
  glc.addEventListener("webglcontextlost", e => { e.preventDefault(); if (!alive) return; V.lost = true; msg(T("m3d.err.lost")); console.warn("mzLab 3D: context lost", diagnose(gl, { opts: cx.opts })); });
  const tex = {}; const vao = gl.createVertexArray();
  const mkTex = (id, w, h, fmt, type, data, filter) => {
    if (!tex[id]) tex[id] = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex[id]); gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, fmt.internal, w, h, 0, fmt.format, type, data);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  };
  const bufs = { stick: gl.createBuffer(), cont: gl.createBuffer(), plane: gl.createBuffer(), box: gl.createBuffer() };
  let nStick = 0, nCont = 0;
  const bgCol = () => { const c = css("--panel"); const m = /#([0-9a-f]{6})/i.exec(c); if (m) return [parseInt(m[1].slice(0, 2), 16) / 255, parseInt(m[1].slice(2, 4), 16) / 255, parseInt(m[1].slice(4), 16) / 255]; const r = /rgba?\((\d+)[ ,]+(\d+)[ ,]+(\d+)/.exec(c); return r ? [r[1] / 255, r[2] / 255, r[3] / 255] : [1, 1, 1]; };

  // ---- geometry of the box: RT along x, m/z along -z (high m/z at the back, like the 2D map seen from above), height along y
  const box = () => ({ x: P.asp, y: 0.55 * P.ex, z: 1 });
  const rtAt = i => V.rt[0] + (i + 0.5) * (V.rt[1] - V.rt[0]) / V.g.nrt;
  const iOf = rt => (rt - V.rt[0]) / (V.rt[1] - V.rt[0]) * V.g.nrt - 0.5;
  const mzAt = j => V.g.log ? V.g.mz0 * Math.exp((j + 0.5) * V.g.dlog) : V.g.mz0 + (j + 0.5) * V.g.dmz;
  const jOf = mz => (V.g.log ? Math.log(mz / V.g.mz0) / V.g.dlog : (mz - V.g.mz0) / V.g.dmz) - 0.5;
  const px = i => (i / (V.g.nrt - 1) - 0.5) * box().x, pz = j => (0.5 - j / (V.g.nmz - 1)) * box().z;

  // ---- camera
  const eyeOf = () => { const c = V.cam, az = c.az * Math.PI / 180, el = c.el * Math.PI / 180; return [c.t[0] + c.d * Math.sin(az) * Math.cos(el), c.t[1] + c.d * Math.sin(el), c.t[2] + c.d * Math.cos(az) * Math.cos(el)]; };
  const FOV = 35 * Math.PI / 180;
  const matVP = () => m4.mul(m4.persp(FOV, V.W / V.H, 0.02, 60), m4.look(eyeOf(), V.cam.t, [0, 1, 0]));
  let VP = null;
  const project = (x, y, z) => { const m = VP, w = m[3] * x + m[7] * y + m[11] * z + m[15], cx = (m[0] * x + m[4] * y + m[8] * z + m[12]) / w, cy = (m[1] * x + m[5] * y + m[9] * z + m[13]) / w; return [(cx * 0.5 + 0.5) * V.W, (0.5 - cy * 0.5) * V.H, w]; };
  const resetCam = (az = 35, el = 32) => { const b = box(), r = Math.hypot(b.x, b.z, b.y) / 2; V.cam = { az, el, d: Math.max(1.5, r * 3.6), t: [0, b.y * (V.signed ? 0 : 0.25), 0] }; V.dirty = true; };
  const setView = n => { if (n === 1) resetCam(0, 89); else if (n === 2) resetCam(0, 0); else if (n === 3) resetCam(90, 0); else resetCam(35, 32); if (n !== 4) { const b = box(); V.cam.d = Math.max(1.5, Math.hypot(b.x, b.z) * 2.2); } V.dirty = true; };

  // ---- data
  const decode = j => ({ ...j, m: b64f(j.data, Float32Array) });
  async function load() {
    const gen = ++V.gen, N = GRID(), [rt0, rt1] = V.rt, [mz0, mz1] = V.mz;
    if (!(rt1 > rt0 && mz1 > mz0 && mz0 > 0)) { msg(T("m3d.err.range")); return; }
    msg(T("m3d.loading"));
    const nrt = Math.min(N, Math.max(2, Math.ceil((rt1 - rt0) / Math.max(scanStep(), 1e-4))));
    const q = `api/map?k=${V.k}&level=${V.lv}&rt0=${rt0}&rt1=${rt1}&mz0=${mz0}&mz1=${mz1}&nrt=${nrt}&nmz=${hr ? N : Math.min(N, Math.max(2, Math.ceil(mz1 - mz0)))}&mzscale=${hr ? "log" : "lin"}${prefs.bg ? "&bg=1" : ""}` +
      (V.ref != null ? `&ref=${V.ref}&norm=${V.norm}&rttol=${MAPPA.diffOpt().rttol}` : "&agg=max");
    let j; try { j = decode(await J(q)); } catch (e) { msg(e && e.message ? e.message : T("m3d.err.load")); return; }
    if (gen !== V.gen) return;
    if (j.nrt < 2 || j.nmz < 2) { msg(T("m3d.err.small")); return; }
    V.g = j; V.raw = j.m; V.signed = V.ref != null; V.pts = null;
    Q("res").textContent = T("m3d.res", { nrt: j.nrt, nmz: j.nmz }) + (j.log ? " · " + T("m3d.res.log") : "");
    const ti = Q("ti"); ti.max = String(j.nrt - 1); if (V.cur == null || V.cur.i >= j.nrt || V.cur.j >= j.nmz) V.cur = { i: Math.floor(j.nrt / 2), j: Math.floor(j.nmz / 2) };
    Q("rt0").value = +rt0.toFixed(3); Q("rt1").value = +rt1.toFixed(3); Q("mz0").value = +mz0.toFixed(hr ? 4 : 1); Q("mz1").value = +mz1.toFixed(hr ? 4 : 1);
    msg("");
    if (P.mode === "sticks" && (!hr || V.signed)) { P.mode = "surf"; Q("mode").value = "surf"; }
    Q("mode").querySelector('[value="sticks"]').disabled = !hr || V.signed;
    process(); resetCam(V.cam.az, V.cam.el); readout(); await loadSticks(gen);
  }
  async function loadSticks(gen) {
    if (P.mode !== "sticks" || !hr || V.signed) return;
    try {
      const j = await J(`api/mappunti3d?k=${V.k}&level=${V.lv}&rt0=${V.rt[0]}&rt1=${V.rt[1]}&mz0=${V.mz[0]}&mz1=${V.mz[1]}&max=${STICKS()}`);
      if (gen !== V.gen) return;
      V.pts = { n: j.n, total: j.total, rt: b64f(j.rt, Float32Array), mz: b64f(j.mz, Float64Array), y: b64f(j.inten, Float32Array) };
      uploadSticks();
    } catch (e) { msg(e && e.message ? e.message : T("m3d.err.load")); }
  }
  function msg(t) { let m = main.querySelector(".m3d-msg"); if (!t) { if (m) m.remove(); return; } if (!m) { m = document.createElement("div"); m.className = "m3d-msg"; m.setAttribute("role", "status"); main.appendChild(m); } m.textContent = t; }

  const scaleT = r => P.hs === "lin" ? r : P.hs === "log" ? Math.log10(1 + 1000 * r) / 3 : Math.sqrt(r);
  const thr = () => (prefs.thr ?? (V.signed ? 0.02 : 0.004));
  function process() {
    const g = V.g, raw = V.raw, n = raw.length, nz = [];
    for (let i = 0; i < n; i += Math.max(1, n >> 17)) { const x = Math.abs(raw[i]); if (x > 0) nz.push(x); }
    nz.sort((u, v) => u - v);
    V.refv = nz.length ? nz[Math.min(nz.length - 1, Math.floor(nz.length * 0.995))] : 1;
    const Z = new Float32Array(n), th = thr();
    for (let i = 0; i < n; i++) { const x = raw[i], r = Math.min(1, Math.abs(x) / V.refv); Z[i] = r < th ? 0 : (x < 0 ? -1 : 1) * scaleT(r); }
    V.Z = P.sig > 0.05 ? blur(Z, g.nrt, g.nmz, P.sig) : Z;
    let lo = 0, hi = 0; for (let i = 0; i < n; i++) { const z = V.Z[i]; if (z < lo) lo = z; if (z > hi) hi = z; }
    V.zmin = lo; V.zmax = hi || 1;
    Q("thr").value = +(th * 100).toFixed(2);
    hist(); upload(); V.dirty = true;
  }
  const palData = () => V.signed ? LUT_DIV : lutSeq(mapPal());
  function upload() {
    const g = V.g, F = { internal: gl.R32F, format: gl.RED };
    mkTex("h", g.nmz, g.nrt, F, gl.FLOAT, V.Z, gl.NEAREST);
    mkTex("pal", 256, 1, { internal: gl.RGBA8, format: gl.RGBA }, gl.UNSIGNED_BYTE, new Uint8Array(palData().buffer.slice(0)), gl.LINEAR);
    const px4 = new Uint8Array(g.nrt * g.nmz * 4), tab = palData();       // the colour image of the floor: j upwards = m/z upwards
    for (let i = 0; i < g.nrt; i++) for (let j = 0; j < g.nmz; j++) {
      const z = V.Z[i * g.nmz + j], q = V.signed ? Math.round(127.5 - 127.5 * clamp(z, -1, 1)) : Math.round(clamp(z, 0, 1) * 255), o = (j * g.nrt + i) * 4;
      px4[o] = tab[q * 4]; px4[o + 1] = tab[q * 4 + 1]; px4[o + 2] = tab[q * 4 + 2]; px4[o + 3] = 255;
    }
    mkTex("map", g.nrt, g.nmz, { internal: gl.RGBA8, format: gl.RGBA }, gl.UNSIGNED_BYTE, px4, gl.NEAREST);
    contours();
    if (V.pts) uploadSticks();
    const ti = Q("ti"); ti.max = String(g.nrt - 1);
  }
  function uploadSticks() {
    const pt = V.pts, g = V.g, th = thr(), a = new Float32Array(pt.n * 3); let n = 0;
    const lo = Math.log(V.mz[0]), span = Math.log(V.mz[1] / V.mz[0]);
    for (let i = 0; i < pt.n; i++) {
      const r = Math.min(1, pt.y[i] / V.refv); if (r < th) continue;
      a[n * 3] = (pt.rt[i] - V.rt[0]) / (V.rt[1] - V.rt[0]); a[n * 3 + 1] = (Math.log(pt.mz[i]) - lo) / span; a[n * 3 + 2] = scaleT(r); n++;
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, bufs.stick); gl.bufferData(gl.ARRAY_BUFFER, a.subarray(0, n * 3), gl.STATIC_DRAW); nStick = n; V.dirty = true;
  }
  function contours() {                                       // marching squares on a coarser grid, 8 levels, drawn flat on the floor
    const g = V.g, st = Math.max(1, Math.ceil(Math.max(g.nrt, g.nmz) / 256)), nx = Math.ceil(g.nrt / st), ny = Math.ceil(g.nmz / st), seg = [];
    const val = (i, j) => Math.abs(V.Z[Math.min(g.nrt - 1, i * st) * g.nmz + Math.min(g.nmz - 1, j * st)]), tab = palData();
    const L = V.signed ? 6 : 8;
    for (let l = 1; l <= L; l++) {
      const lev = Math.max(l / (L + 1) * Math.max(Math.abs(V.zmin), V.zmax), 1e-6), t = clamp(l / (L + 1), 0, 1), q = Math.round(t * 255), col = V.signed ? [0.1, 0.1, 0.1] : [tab[q * 4] / 255, tab[q * 4 + 1] / 255, tab[q * 4 + 2] / 255];
      const X = i => (Math.min(i * st, g.nrt - 1) / (g.nrt - 1) - 0.5), Zc = j => (0.5 - Math.min(j * st, g.nmz - 1) / (g.nmz - 1));
      const lerp = (a0, a1) => (lev - a0) / (a1 - a0 || 1e-9);
      for (let i = 0; i < nx - 1; i++) for (let j = 0; j < ny - 1; j++) {
        const v0 = val(i, j), v1 = val(i + 1, j), v2 = val(i + 1, j + 1), v3 = val(i, j + 1), c = (v0 > lev ? 1 : 0) | (v1 > lev ? 2 : 0) | (v2 > lev ? 4 : 0) | (v3 > lev ? 8 : 0);
        if (c === 0 || c === 15) continue;
        const e = [[X(i) + lerp(v0, v1) * (X(i + 1) - X(i)), Zc(j)], [X(i + 1), Zc(j) + lerp(v1, v2) * (Zc(j + 1) - Zc(j))], [X(i) + lerp(v3, v2) * (X(i + 1) - X(i)), Zc(j + 1)], [X(i), Zc(j) + lerp(v0, v3) * (Zc(j + 1) - Zc(j))]];
        const T2 = { 1: [[3, 0]], 2: [[0, 1]], 3: [[3, 1]], 4: [[1, 2]], 5: [[3, 0], [1, 2]], 6: [[0, 2]], 7: [[3, 2]], 8: [[2, 3]], 9: [[0, 2]], 10: [[0, 1], [2, 3]], 11: [[1, 2]], 12: [[1, 3]], 13: [[0, 1]], 14: [[3, 0]] };
        for (const [s0, s1] of T2[c]) seg.push(e[s0][0], 0, e[s0][1], ...col, e[s1][0], 0, e[s1][1], ...col);
      }
    }
    const b = box(), arr = new Float32Array(seg); for (let i = 0; i < arr.length; i += 6) { arr[i] *= b.x; arr[i + 2] *= b.z; }
    V.contRaw = new Float32Array(seg); nCont = arr.length / 6;
    gl.bindBuffer(gl.ARRAY_BUFFER, bufs.cont); gl.bufferData(gl.ARRAY_BUFFER, arr, gl.STATIC_DRAW);
  }
  function hist() {
    if (!V.raw) return;                                       // small histogram of |value| / reference (log scale of the ratio), with the threshold
    const c = Q("hist"), r = c.getBoundingClientRect(), d = Math.min(2, devicePixelRatio || 1); c.width = Math.max(10, Math.round((r.width || 280) * d)); c.height = Math.round(46 * d);
    const g = c.getContext("2d"), W = c.width, H = c.height, nb = 64, h = new Float64Array(nb), raw = V.raw;
    for (let i = 0; i < raw.length; i += Math.max(1, raw.length >> 18)) { const x = Math.abs(raw[i]) / V.refv; if (x > 0) h[clamp(Math.floor((Math.log10(Math.min(1, x)) + 4) / 4 * nb), 0, nb - 1)]++; }
    const mx = Math.max(...h.map(v => Math.log1p(v)), 1); g.clearRect(0, 0, W, H); g.fillStyle = css("--muted");
    for (let i = 0; i < nb; i++) { const bh = Math.log1p(h[i]) / mx * (H - 4); g.fillRect(i * W / nb + 1, H - bh, W / nb - 1, bh); }
    const th = thr(), xx = clamp((Math.log10(Math.max(th, 1e-4)) + 4) / 4, 0, 1) * W; g.strokeStyle = css("--accent"); g.lineWidth = 2 * d; g.beginPath(); g.moveTo(xx, 0); g.lineTo(xx, H); g.stroke();
  }

  // ---- drawing
  function resize() {
    const r = main.getBoundingClientRect(), d = Math.min(2, devicePixelRatio || 1); V.W = Math.max(50, r.width); V.H = Math.max(50, r.height);
    for (const c of [glc, ovc]) { c.width = Math.round(V.W * d); c.height = Math.round(V.H * d); }
    ov.setTransform(d, 0, 0, d, 0, 0); V.dpr = d; V.dirty = true; hist();
  }
  function frame(scale = 1) {
    if (!V.g || !V.Z) { gl.viewport(0, 0, glc.width, glc.height); gl.clearColor(...bgCol(), 1); gl.clear(gl.COLOR_BUFFER_BIT); return; }
    VP = matVP();
    const g = V.g, b = box(), bg = bgCol(), eye = eyeOf();
    gl.viewport(0, 0, glc.width, glc.height); gl.clearColor(bg[0], bg[1], bg[2], 1); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT); gl.enable(gl.DEPTH_TEST); gl.depthFunc(gl.LEQUAL); gl.bindVertexArray(vao);
    const bindT = (u, id, unit) => { gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, tex[id]); gl.uniform1i(u, unit); };
    const cont = P.mode === "cont", floorMap = P.fl || cont;
    gl.useProgram(PR.floor.p); gl.uniformMatrix4fv(PR.floor.u.uVP, false, VP); gl.uniform3f(PR.floor.u.uBox, b.x, b.y, b.z);
    gl.uniform1f(PR.floor.u.uUse, floorMap ? 1 : 0); gl.uniform4f(PR.floor.u.uCol, bg[0] * 0.93, bg[1] * 0.93, bg[2] * 0.93, 1); bindT(PR.floor.u.uMap, "map", 3); gl.drawArrays(gl.TRIANGLES, 0, 6);
    if (P.mode === "surf") {
      const s = PR.surf; gl.useProgram(s.p); gl.uniformMatrix4fv(s.u.uVP, false, VP); gl.uniform3f(s.u.uBox, b.x, b.y, b.z); gl.uniform2i(s.u.uN, g.nrt, g.nmz);
      gl.uniform1f(s.u.uSigned, V.signed ? 1 : 0); gl.uniform1f(s.u.uLight, P.lt ? 1 : 0); gl.uniform3f(s.u.uEye, eye[0], eye[1], eye[2]);
      bindT(s.u.uH, "h", 0); bindT(s.u.uPal, "pal", 1); gl.drawArrays(gl.TRIANGLES, 0, (g.nrt - 1) * (g.nmz - 1) * 6);
    } else if (P.mode === "sticks" && nStick) {
      const s = PR.stick; gl.useProgram(s.p); gl.uniformMatrix4fv(s.u.uVP, false, VP); gl.uniform3f(s.u.uBox, b.x, b.y, b.z); bindT(s.u.uPal, "pal", 1);
      const loc = gl.getAttribLocation(s.p, "aP"); gl.bindBuffer(gl.ARRAY_BUFFER, bufs.stick); gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, 3, gl.FLOAT, false, 0, 0); gl.vertexAttribDivisor(loc, 1);
      gl.drawArraysInstanced(gl.LINES, 0, 2, nStick); gl.vertexAttribDivisor(loc, 0); gl.disableVertexAttribArray(loc);
    } else if (P.mode === "fall") {
      const s = PR.row; gl.useProgram(s.p); gl.uniformMatrix4fv(s.u.uVP, false, VP); gl.uniform3f(s.u.uBox, b.x, b.y, b.z); gl.uniform2i(s.u.uN, g.nrt, g.nmz); gl.uniform1f(s.u.uSigned, V.signed ? 1 : 0);
      gl.uniform3f(s.u.uBg, bg[0], bg[1], bg[2]); gl.uniform4f(s.u.uFlat, 0, 0, 0, 0); bindT(s.u.uH, "h", 0); bindT(s.u.uPal, "pal", 1);
      const step = Math.max(1, Math.ceil(g.nrt / 140));
      gl.enable(gl.POLYGON_OFFSET_FILL); gl.polygonOffset(1, 1);
      for (let i = 0; i < g.nrt; i += step) { gl.uniform1i(s.u.uRow, i); gl.uniform1i(s.u.uMode, 0); gl.uniform1f(s.u.uLift, 0); gl.drawArrays(gl.TRIANGLE_STRIP, 0, g.nmz * 2); }
      gl.disable(gl.POLYGON_OFFSET_FILL);
      for (let i = 0; i < g.nrt; i += step) { gl.uniform1i(s.u.uRow, i); gl.uniform1i(s.u.uMode, 1); gl.uniform1f(s.u.uLift, 0.0006); gl.drawArrays(gl.LINE_STRIP, 0, g.nmz); }
    } else if (cont && nCont) {
      const s = PR.line; gl.useProgram(s.p); gl.uniformMatrix4fv(s.u.uVP, false, VP); gl.uniform1f(s.u.uA, 1);
      gl.bindBuffer(gl.ARRAY_BUFFER, bufs.cont); const lp = gl.getAttribLocation(s.p, "aP"), lc = gl.getAttribLocation(s.p, "aC");
      gl.enableVertexAttribArray(lp); gl.vertexAttribPointer(lp, 3, gl.FLOAT, false, 24, 0); gl.enableVertexAttribArray(lc); gl.vertexAttribPointer(lc, 3, gl.FLOAT, false, 24, 12);
      gl.drawArrays(gl.LINES, 0, nCont); gl.disableVertexAttribArray(lp); gl.disableVertexAttribArray(lc);
    }
    // the time plane and the spectrum line at that RT
    if (V.plane && V.cur) {
      const i = V.cur.i, x = px(i), y1 = Math.max(b.y * V.zmax, b.y * 0.2) * 1.08, y0 = V.signed ? b.y * V.zmin * 1.08 : 0, h = b.z / 2, ac = css("--accent"), m = /#([0-9a-f]{6})/i.exec(ac), c = m ? [parseInt(m[1].slice(0, 2), 16) / 255, parseInt(m[1].slice(2, 4), 16) / 255, parseInt(m[1].slice(4), 16) / 255] : [0.17, 0.36, 0.54];
      const quad = new Float32Array([x, y0, -h, ...c, x, y0, h, ...c, x, y1, -h, ...c, x, y1, -h, ...c, x, y0, h, ...c, x, y1, h, ...c]);
      const s = PR.line; gl.useProgram(s.p); gl.uniformMatrix4fv(s.u.uVP, false, VP); gl.uniform1f(s.u.uA, 0.2); gl.enable(gl.BLEND); gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA); gl.depthMask(false);
      gl.bindBuffer(gl.ARRAY_BUFFER, bufs.plane); gl.bufferData(gl.ARRAY_BUFFER, quad, gl.DYNAMIC_DRAW);
      const lp = gl.getAttribLocation(s.p, "aP"), lc = gl.getAttribLocation(s.p, "aC"); gl.enableVertexAttribArray(lp); gl.vertexAttribPointer(lp, 3, gl.FLOAT, false, 24, 0); gl.enableVertexAttribArray(lc); gl.vertexAttribPointer(lc, 3, gl.FLOAT, false, 24, 12);
      gl.drawArrays(gl.TRIANGLES, 0, 6); gl.depthMask(true); gl.disable(gl.BLEND); gl.disableVertexAttribArray(lp); gl.disableVertexAttribArray(lc);
      if (P.mode !== "sticks") {
        const r = PR.row; gl.useProgram(r.p); gl.uniformMatrix4fv(r.u.uVP, false, VP); gl.uniform3f(r.u.uBox, b.x, b.y, b.z); gl.uniform2i(r.u.uN, g.nrt, g.nmz); gl.uniform1f(r.u.uSigned, V.signed ? 1 : 0);
        gl.uniform4f(r.u.uFlat, c[0], c[1], c[2], 1); gl.uniform1i(r.u.uRow, i); gl.uniform1i(r.u.uMode, 1); gl.uniform1f(r.u.uLift, 0.003); gl.disable(gl.DEPTH_TEST);
        bindT(r.u.uH, "h", 0); bindT(r.u.uPal, "pal", 1); gl.drawArrays(gl.LINE_STRIP, 0, g.nmz); gl.enable(gl.DEPTH_TEST);
      }
    }
    gl.bindVertexArray(null);
    overlay(scale);
  }
  const fmtMz = v => v.toFixed(hr ? 4 : 1);
  function overlay(scale = 1) {
    const g = V.g, b = box(), c = ov; c.save(); c.setTransform(V.dpr * scale, 0, 0, V.dpr * scale, 0, 0); c.clearRect(0, 0, V.W, V.H);
    const ink = css("--muted"), strong = css("--ink"); c.font = "11px system-ui, sans-serif"; c.fillStyle = ink; c.strokeStyle = ink; c.lineWidth = 1;
    const e = eyeOf(), cw = [[-b.x / 2, -b.z / 2], [b.x / 2, -b.z / 2], [b.x / 2, b.z / 2], [-b.x / 2, b.z / 2]];
    // the floor outline
    c.beginPath(); cw.forEach(([x, z], i) => { const q = project(x, 0, z); i ? c.lineTo(q[0], q[1]) : c.moveTo(q[0], q[1]); }); c.closePath(); c.stroke();
    // axes: RT on the edge nearest the viewer along x, m/z on the nearest edge along z; limited to a few ticks so labels never overlap
    const zn = e[2] > 0 ? b.z / 2 : -b.z / 2, xn = e[0] > 0 ? b.x / 2 : -b.x / 2;
    const lab = (txt, q, dx, dy, al) => { c.textAlign = al; c.fillStyle = strong; c.fillText(txt, q[0] + dx, q[1] + dy); };
    const place = list => { let last = -1e9; return list.filter(t => { if (t.s - last < 46) return false; last = t.s; return true; }); };
    const rtT = nice(rtAt(0), rtAt(g.nrt - 1), 6).filter(t => t >= rtAt(0) && t <= rtAt(g.nrt - 1)).map(t => ({ t, i: iOf(t) })).map(o => ({ ...o, q: project(px(o.i), 0, zn), s: 0 }));
    rtT.forEach(o => { o.s = o.q[0]; });
    for (const o of place(rtT.sort((u, v) => u.s - v.s))) { const q2 = project(px(o.i), 0, zn * 1.06); c.beginPath(); c.moveTo(o.q[0], o.q[1]); c.lineTo(q2[0], q2[1]); c.stroke(); lab(String(+o.t.toFixed(2)), q2, 0, 13, "center"); }
    const tm = project(0, 0, zn * 1.2); lab("RT (min)", tm, 0, 26, "center");
    const mzT = nice(mzAt(0), mzAt(g.nmz - 1), 6).filter(t => t >= mzAt(0) && t <= mzAt(g.nmz - 1)).map(t => { const j = jOf(t), q = project(xn, 0, pz(j)); return { t, j, q, s: q[1] }; });
    for (const o of place(mzT.sort((u, v) => u.s - v.s))) { const q2 = project(xn * 1.06, 0, pz(o.j)); c.beginPath(); c.moveTo(o.q[0], o.q[1]); c.lineTo(q2[0], q2[1]); c.stroke(); lab(hr ? String(+o.t.toFixed(o.t > 1000 ? 1 : 2)) : String(Math.round(o.t)), q2, xn > 0 ? 4 : -4, 4, xn > 0 ? "left" : "right"); }
    const tz = project(xn * 1.22, 0, 0); lab("m/z", tz, 0, 4, "center");
    // the vertical axis, with the intensity of a few heights
    const cx = e[0] > 0 ? -b.x / 2 : b.x / 2, cz = e[2] > 0 ? -b.z / 2 : b.z / 2, top = V.zmax;
    const a0 = project(cx, 0, cz), a1 = project(cx, b.y * top, cz); c.beginPath(); c.moveTo(a0[0], a0[1]); c.lineTo(a1[0], a1[1]); c.stroke();
    const inv = z => { const r = P.hs === "lin" ? z : P.hs === "log" ? (Math.pow(10, z * 3) - 1) / 1000 : z * z; return r * V.refv; };
    let lastY = 1e9; for (let k = 4; k >= 1; k--) { const z = top * k / 4, q = project(cx, b.y * z, cz); if (lastY - q[1] < 24) continue; lastY = q[1]; c.beginPath(); c.moveTo(q[0] - 3, q[1]); c.lineTo(q[0] + 3, q[1]); c.stroke(); lab(fmt(inv(z)), q, e[0] > 0 ? -6 : 6, 3, e[0] > 0 ? "right" : "left"); }
    // the marked points (the same list as the 2D map) that fall in the region
    try {
      MAPPA.pts(p).forEach((s, n) => {
        const i = iOf(s.rt), j = jOf(s.mz); if (i < 0 || j < 0 || i > g.nrt - 1 || j > g.nmz - 1) return;
        const z = heightAt(i, j), q = project(px(i), z * b.y, pz(j)), q0 = project(px(i), 0, pz(j));
        c.strokeStyle = strong; c.beginPath(); c.moveTo(q0[0], q0[1]); c.lineTo(q[0], q[1]); c.stroke(); c.fillStyle = css("--panel"); c.beginPath(); c.arc(q[0], q[1], 8, 0, 7); c.fill(); c.stroke();
        c.fillStyle = strong; c.textAlign = "center"; c.font = "bold 10px system-ui, sans-serif"; c.fillText(String(n + 1), q[0], q[1] + 3.5); c.font = "11px system-ui, sans-serif";
      });
    } catch (err) { /* no points */ }
    // the ray: from the floor to the surface at the cursor
    if (V.cur) {
      const i = V.cur.i, j = V.cur.j, z = V.Z[i * g.nmz + j], q = project(px(i), z * b.y, pz(j)), q0 = project(px(i), 0, pz(j));
      c.strokeStyle = css("--accent"); c.lineWidth = 2; c.beginPath(); c.moveTo(q0[0], q0[1]); c.lineTo(q[0], q[1]); c.stroke();
      c.fillStyle = css("--accent"); c.beginPath(); c.arc(q[0], q[1], 4, 0, 7); c.fill(); c.beginPath(); c.arc(q0[0], q0[1], 3, 0, 7); c.fill();
    }
    c.restore();
  }
  const heightAt = (fi, fj) => {
    const g = V.g, i0 = clamp(Math.floor(fi), 0, g.nrt - 2), j0 = clamp(Math.floor(fj), 0, g.nmz - 2), u = clamp(fi - i0, 0, 1), w = clamp(fj - j0, 0, 1), Z = V.Z, n = g.nmz;
    return (Z[i0 * n + j0] * (1 - u) + Z[(i0 + 1) * n + j0] * u) * (1 - w) + (Z[i0 * n + j0 + 1] * (1 - u) + Z[(i0 + 1) * n + j0 + 1] * u) * w;
  };
  // ---- the mouse ray meets the height grid (CPU)
  function pick(mx, my) {
    if (!V.g) return null;
    const b = box(), g = V.g, e = eyeOf(), c = V.cam, f = norm3(sub3(c.t, e)), r = norm3(cross3(f, [0, 1, 0])), u = cross3(r, f), th = Math.tan(FOV / 2), asp = V.W / V.H;
    const nx = (mx / V.W) * 2 - 1, ny = 1 - (my / V.H) * 2, d = norm3([f[0] + r[0] * nx * th * asp + u[0] * ny * th, f[1] + r[1] * nx * th * asp + u[1] * ny * th, f[2] + r[2] * nx * th * asp + u[2] * ny * th]);
    const lo = [-b.x / 2, V.signed ? b.y * V.zmin - 0.01 : -0.01, -b.z / 2], hi = [b.x / 2, b.y * Math.max(V.zmax, 0.05) + 0.01, b.z / 2];
    let t0 = 0, t1 = 1e9;
    for (let k = 0; k < 3; k++) {
      if (Math.abs(d[k]) < 1e-9) { if (e[k] < lo[k] || e[k] > hi[k]) return null; continue; }
      let a = (lo[k] - e[k]) / d[k], bb = (hi[k] - e[k]) / d[k]; if (a > bb) [a, bb] = [bb, a]; t0 = Math.max(t0, a); t1 = Math.min(t1, bb); if (t0 > t1) return null;
    }
    const ns = clamp(2 * (g.nrt + g.nmz), 200, 3000), dt = (t1 - t0) / ns;
    const fo = t => { const x = e[0] + d[0] * t, z = e[2] + d[2] * t, fi = (x / b.x + 0.5) * (g.nrt - 1), fj = (0.5 - z / b.z) * (g.nmz - 1); return [fi, fj, e[1] + d[1] * t - heightAt(fi, fj) * b.y]; };
    let prev = fo(t0), hit = null;
    for (let s = 1; s <= ns; s++) {
      const t = t0 + s * dt, cu = fo(t);
      if ((prev[2] > 0) !== (cu[2] > 0)) { let a = t - dt, bq = t; for (let k = 0; k < 12; k++) { const m = (a + bq) / 2, fm = fo(m); if ((fm[2] > 0) === (prev[2] > 0)) a = m; else bq = m; } hit = fo(bq); break; }
      prev = cu;
    }
    if (!hit && Math.abs(d[1]) > 1e-9) { const t = -e[1] / d[1]; if (t > 0) { const x = e[0] + d[0] * t, z = e[2] + d[2] * t; if (Math.abs(x) <= b.x / 2 && Math.abs(z) <= b.z / 2) hit = [(x / b.x + 0.5) * (g.nrt - 1), (0.5 - z / b.z) * (g.nmz - 1), 0]; } }
    return hit ? { i: clamp(Math.round(hit[0]), 0, g.nrt - 1), j: clamp(Math.round(hit[1]), 0, g.nmz - 1), fi: hit[0], fj: hit[1] } : null;
  }

  // ---- read-out and the two small plots
  function readout() {
    if (!V.cur || !V.g) { rd.textContent = ""; return; }
    const g = V.g, { i, j } = V.cur, v = V.raw[i * g.nmz + j], rt = rtAt(i), mz = mzAt(j);
    const cell = !hr && g.dmz >= 0.9 ? T("m3d.read.nom", { n: Math.round(g.mz0 + j * g.dmz + 0.2) }) : "";
    rd.textContent = (V.lock ? T("mappa.read.locked") + " · " : "") + `${f.label}${V.ref != null ? " − " + (E.files[V.ref] || {}).label : ""} · RT ${rt.toFixed(2)} min · m/z ${fmtMz(mz)}${cell ? " (" + cell + ")" : ""} · ${T("mappa.read.int")} ${(V.signed && v > 0 ? "+" : "") + fmt(v)}`;
    Q("ti").value = String(i); Q("tt").textContent = `RT ${rt.toFixed(2)}`;
    mini();
  }
  function mini() {
    const g = V.g, { i, j } = V.cur, col = new Float32Array(g.nrt), row = V.raw.subarray(i * g.nmz, (i + 1) * g.nmz);
    for (let a = 0; a < g.nrt; a++) col[a] = V.raw[a * g.nmz + j];
    plot(xicc, null, col, i, T("m3d.xic", { mz: fmtMz(mzAt(j)) }), [rtAt(0).toFixed(1), rtAt(g.nrt - 1).toFixed(1)]);
    plot(spcc, null, row, j, T("m3d.spc", { rt: rtAt(i).toFixed(2) }), [fmtMz(mzAt(0)), fmtMz(mzAt(g.nmz - 1))]);
  }

  // ---- interaction
  const setCur = (i, j) => { const g = V.g; if (!g) return; i = clamp(i, 0, g.nrt - 1); j = clamp(j, 0, g.nmz - 1); if (V.cur && V.cur.i === i && V.cur.j === j) return; V.cur = { i, j }; readout(); V.dirty = true; };
  const ptrs = new Map(); let drag = null, moved = 0, lastPinch = 0, lpT = 0, lpDone = false;
  glc.addEventListener("pointerdown", e => {
    glc.setPointerCapture(e.pointerId); ptrs.set(e.pointerId, { x: e.offsetX, y: e.offsetY }); moved = 0;
    drag = { x: e.offsetX, y: e.offsetY, pan: e.shiftKey || e.button === 2 || e.button === 1 }; if (ptrs.size === 2) lastPinch = 0; glc.focus({ preventScroll: true });
    clearTimeout(lpT); lpDone = false;
    if (e.pointerType !== "mouse" && ptrs.size === 1) { const at = { x: e.offsetX, y: e.offsetY, cx: e.clientX, cy: e.clientY }; lpT = setTimeout(() => { if (ptrs.size === 1 && moved < 8) { lpDone = true; openMenu({ ...at, lp: true }); } }, 550); }       // long press = the menu
  });
  glc.addEventListener("pointermove", e => {
    const q = ptrs.get(e.pointerId);
    if (!q) { if (V.lock) return; const h = pick(e.offsetX, e.offsetY); if (h) setCur(h.i, h.j); return; }       // locked: the cursor stays
    const dx = e.offsetX - q.x, dy = e.offsetY - q.y; q.x = e.offsetX; q.y = e.offsetY; moved += Math.abs(dx) + Math.abs(dy);
    if (ptrs.size === 2) {                                  // two fingers: move, and pinch = zoom
      const [A, B] = [...ptrs.values()], dist = Math.hypot(A.x - B.x, A.y - B.y); if (lastPinch) V.cam.d = clamp(V.cam.d * lastPinch / dist, 0.2, 12); lastPinch = dist; panBy(dx / 2, dy / 2); V.dirty = true; return;
    }
    if (drag && drag.pan) panBy(dx, dy); else { V.cam.az -= dx * 0.4; V.cam.el = clamp(V.cam.el + dy * 0.4, -89, 89); }
    V.dirty = true;
  });
  const up = e => {
    const had = ptrs.delete(e.pointerId); if (!had) return; lastPinch = 0; clearTimeout(lpT);
    if (moved < 5 && ptrs.size === 0 && e.type === "pointerup" && !lpDone) {
      if (e.button === 0) click(e);                                                             // a click that does not move = fix the point
      else if (e.button === 2) openMenu({ x: e.offsetX, y: e.offsetY, cx: e.clientX, cy: e.clientY });       // a right click that does not move = the menu (a drag moves the view)
    }
    drag = null;
  };
  glc.addEventListener("pointerup", up); glc.addEventListener("pointercancel", up);
  glc.addEventListener("contextmenu", e => e.preventDefault());
  function panBy(dx, dy) {
    const e = eyeOf(), c = V.cam, f = norm3(sub3(c.t, e)), r = norm3(cross3(f, [0, 1, 0])), u = cross3(r, f), k = 2 * c.d * Math.tan(FOV / 2) / V.H;
    for (let a = 0; a < 3; a++) c.t[a] += (-dx * r[a] + dy * u[a]) * k;
  }
  glc.addEventListener("wheel", e => { e.preventDefault(); V.cam.d = clamp(V.cam.d * Math.exp(e.deltaY * 0.0012), 0.2, 12); V.dirty = true; }, { passive: false });
  function click(e) {
    const h = pick(e.offsetX, e.offsetY); if (!h) return; setCur(h.i, h.j);
    MAPPA.addPointAt(p, rtAt(h.i), mzAt(h.j), V.g.log ? mzAt(h.j) * V.g.dlog : V.g.dmz).then(() => { V.dirty = true; });
  }
  glc.addEventListener("dblclick", e => {
    const h = pick(e.offsetX, e.offsetY); if (!h) return;
    const g = V.g, b = box(); let near = -1, bd = 14 * 14;
    try { MAPPA.pts(p).forEach((s, n) => { const i = iOf(s.rt), j = jOf(s.mz); if (i < 0 || j < 0 || i > g.nrt - 1 || j > g.nmz - 1) return; const q = project(px(i), heightAt(i, j) * b.y, pz(j)), d2 = (q[0] - e.offsetX) ** 2 + (q[1] - e.offsetY) ** 2; if (d2 < bd) { bd = d2; near = n; } }); } catch (err) { /* none */ }
    if (near >= 0) { const s = MAPPA.pts(p)[near]; close(); MAPPA.spectrumAt(p, s.rt); return; }
    V.cam.t = [px(h.fi), heightAt(h.fi, h.fj) * b.y * 0.5, pz(h.fj)]; V.dirty = true;       // double click = centre on that point
  });
  const stepCur = (di, dj) => { if (!V.cur) return; V.plane = true; dlg.querySelector('[data-m="pl"]').checked = true; setCur(V.cur.i + di, V.cur.j + dj); };
  dlg.addEventListener("keydown", e => {
    if (e.target.matches("input[type=text],input.mzf,select") ) return;
    const k = e.key; let used = true;
    if (k === "ArrowLeft") stepCur(-1, 0); else if (k === "ArrowRight") stepCur(1, 0); else if (k === "ArrowUp") stepCur(0, 1); else if (k === "ArrowDown") stepCur(0, -1);
    else if (k === "+" || k === "=") V.cam.d = clamp(V.cam.d * 0.85, 0.2, 12); else if (k === "-" || k === "_") V.cam.d = clamp(V.cam.d / 0.85, 0.2, 12);
    else if (k >= "1" && k <= "4") setView(+k); else if (k === "r" || k === "R") reset();
    else if (k === "l" || k === "L") toggleLock(); else if (k === "?") MAPPA.help("3d");
    else if (k === "ContextMenu" || (k === "F10" && e.shiftKey)) { const r = glc.getBoundingClientRect(); openMenu({ cur: true, cx: r.left + r.width / 2, cy: r.top + r.height / 2, kbd: true }); }
    else used = false;
    if (used) { e.preventDefault(); V.dirty = true; }
  });
  function reset() { Object.assign(P, { ex: 1, asp: DEF.asp }); Q("ex").value = 1; Q("asp").value = DEF.asp; P.sig = hr ? 0 : 1; Q("sig").value = P.sig; V.rt = [a.x0, a.x1]; V.mz = [a.y0, a.y1]; resetCam(35, 32); savePrefs(); load(); }

  // ---- navigation: the commands, the menu and the bar are the ones of the 2D map (MAPPA.CMDS in mappa.js)
  const cellOf = h => h && { rt: rtAt(h.i), mz: mzAt(h.j), v: V.raw[h.i * V.g.nmz + h.j] };
  const ctxEl = () => document.getElementById("ctx");
  function ad3(hit) {
    return {
      kind: "3d", p, k: V.k, pt: () => cellOf(V.lock ? V.cur : hit || V.cur), locked: () => V.lock, toggleLock,
      resetAll: () => { resetCam(35, 32); },
      view: n => setView(n), zoomBy: f => { V.cam.d = clamp(V.cam.d * f, 0.2, 12); V.dirty = true; },
      host: () => dlg.appendChild(ctxEl()),                    // the menu must live inside the modal dialog to be seen and used
      after: () => { close(); }                                // the XIC and the spectrum appear in the panels of the page
    };
  }
  function toggleLock() { V.lock = !V.lock; MAPPA.lockBtn(navBar, V.lock); readout(); }
  function openMenu(at) {
    const hit = at.cur ? null : pick(at.x, at.y);
    if (hit && !V.lock) setCur(hit.i, hit.j);
    MAPPA.showMenu(ad3(hit), { preventDefault() {}, clientX: at.cx, clientY: at.cy, kbd: !!at.kbd, lp: !!at.lp });
  }
  const navBar = MAPPA.barFor(() => ad3(null), "3d"); dlg.querySelector(".m3d-top").after(navBar);
  dlg.addEventListener("keydown", e => { const c = ctxEl(); if (e.key === "Escape" && c && !c.hidden && dlg.contains(c)) { e.preventDefault(); e.stopPropagation(); c.hidden = true; glc.focus({ preventScroll: true }); } }, true);     // Esc closes the menu first, not the viewer
  dlg.addEventListener("cancel", e => { const c = ctxEl(); if (c && !c.hidden && dlg.contains(c)) { e.preventDefault(); c.hidden = true; } });

  // ---- sidebar
  const on = (k, ev, fn) => Q(k).addEventListener(ev, fn);
  on("close", "click", () => close()); on("tog", "click", () => { dlg.classList.toggle("hid"); P.side = !dlg.classList.contains("hid"); savePrefs(); resize(); });
  on("file", "change", e => { V.k = +e.target.value; if (V.ref === V.k) { V.ref = null; Q("ref").value = ""; } load(); });
  on("ref", "change", e => { V.ref = e.target.value === "" ? null : +e.target.value; if (V.ref != null && P.mode === "sticks") { P.mode = "surf"; Q("mode").value = "surf"; } load(); });
  on("norm", "change", e => { V.norm = e.target.value; if (V.ref != null) load(); });
  const rangeIn = () => { const r0 = num(Q("rt0")), r1 = num(Q("rt1")), m0 = num(Q("mz0")), m1 = num(Q("mz1")); if (r0 != null && r1 != null && r1 > r0 && m0 != null && m1 != null && m1 > m0 && m0 > 0) { V.rt = [r0, r1]; V.mz = [m0, m1]; load(); } else msg(T("m3d.err.range")); };
  on("apply", "click", rangeIn); ["rt0", "rt1", "mz0", "mz1"].forEach(k => on(k, "change", rangeIn));
  on("sync", "click", () => { const b2 = p._a; if (!b2) return; V.rt = [b2.x0, b2.x1]; V.mz = [b2.y0, b2.y1]; load(); });
  on("mode", "change", e => { P.mode = e.target.value; savePrefs(); if (P.mode === "sticks" && !V.pts) loadSticks(V.gen); V.dirty = true; });
  on("hs", "change", e => { P.hs = e.target.value; savePrefs(); process(); });
  on("ex", "input", e => { P.ex = +e.target.value; savePrefs(); contours(); V.dirty = true; });
  on("asp", "input", e => { P.asp = +e.target.value; savePrefs(); contours(); V.dirty = true; });
  on("pal", "change", e => { prefs.pal = e.target.value; MAPPA.prefSave(); upload(); V.dirty = true; });
  on("lt", "change", e => { P.lt = e.target.checked; savePrefs(); V.dirty = true; });
  on("fl", "change", e => { P.fl = e.target.checked; savePrefs(); V.dirty = true; });
  on("pl", "change", e => { V.plane = e.target.checked; V.dirty = true; });
  on("thr", "change", e => { const v = num(e.target); if (v != null && v >= 0 && v < 100) { prefs.thr = v / 100; MAPPA.prefSave(); process(); if (V.pts) uploadSticks(); } });
  on("bg", "change", e => { prefs.bg = e.target.checked; MAPPA.prefSave(); load(); });
  on("sig", "input", e => { P.sig = +e.target.value; savePrefs(); process(); });
  on("hist", "click", e => { const r = e.target.getBoundingClientRect(), t = Math.pow(10, (e.clientX - r.left) / r.width * 4 - 4); prefs.thr = +Math.min(0.99, t).toFixed(4); MAPPA.prefSave(); process(); if (V.pts) uploadSticks(); });
  [1, 2, 3, 4].forEach(n => on("v" + n, "click", () => setView(n))); on("reset", "click", reset);
  on("ti", "input", e => { V.plane = true; dlg.querySelector('[data-m="pl"]').checked = true; if (V.cur) setCur(+e.target.value, V.cur.j); });
  const playBtn = Q("play");
  if (REDUCED()) { playBtn.hidden = true; }
  const stop = () => { if (V.playing) { cancelAnimationFrame(V.playing); V.playing = 0; } playBtn.innerHTML = "&#9654;"; playBtn.setAttribute("aria-label", T("m3d.play")); };
  on("play", "click", () => {
    if (V.playing) { stop(); return; }
    V.plane = true; dlg.querySelector('[data-m="pl"]').checked = true; playBtn.innerHTML = "&#10074;&#10074;"; playBtn.setAttribute("aria-label", T("m3d.pause"));
    let t0 = performance.now(), i0 = V.cur ? V.cur.i : 0; const dur = 12000;
    const tick = t => { const g = V.g; if (!g) return; const i = Math.min(g.nrt - 1, i0 + Math.round((t - t0) / dur * g.nrt)); setCur(i, V.cur.j); if (i >= g.nrt - 1) { stop(); return; } V.playing = requestAnimationFrame(tick); };
    V.playing = requestAnimationFrame(tick);
  });
  on("png", "click", () => exportPng());
  function exportPng() {
    const s = Math.min(3, Math.floor(4096 / Math.max(V.W, V.H)) || 1), d = V.dpr; if (!V.g) return;
    const w0 = glc.width, h0 = glc.height;
    glc.width = Math.round(V.W * d * s); glc.height = Math.round(V.H * d * s); ovc.width = glc.width; ovc.height = glc.height;
    frame(s);
    const out = document.createElement("canvas"); out.width = glc.width; out.height = glc.height; const o = out.getContext("2d"); o.drawImage(glc, 0, 0); o.drawImage(ovc, 0, 0);
    glc.width = w0; glc.height = h0; ovc.width = w0; ovc.height = h0; ov.setTransform(d, 0, 0, d, 0, 0); V.dirty = true;
    out.toBlob(bl => { saveBlob(bl, `${f.label}_3d.png`); }, "image/png");
  }

  // ---- life cycle
  let raf = 0, alive = true;
  const loop = () => { if (!alive) return; if (V.dirty) { V.dirty = false; frame(); } raf = requestAnimationFrame(loop); };
  const ro = new ResizeObserver(() => { resize(); });
  function close() { if (dlg.open) dlg.close(); }
  dlg.addEventListener("close", () => {
    alive = false; stop(); cancelAnimationFrame(raf); ro.disconnect(); const ext = gl.getExtension("WEBGL_lose_context"); if (ext) ext.loseContext(); { const c = ctxEl(); if (c && dlg.contains(c)) document.body.appendChild(c); } dlg.remove();
    try { draw(p); } catch (e) { /* the panel is gone */ }
    if (opener && opener.isConnected) opener.focus();
  });
  window.__m3d = { V, P, ptrs, setView, pick, project: (...a2) => project(...a2), box, rtAt, mzAt, px, pz, heightAt, frame, close };     // for the tests
  dlg.showModal(); resize(); ro.observe(main); glc.focus({ preventScroll: true });
  if (!window.ResizeObserver) resize();
  load().then(() => { raf = requestAnimationFrame(loop); });
}
