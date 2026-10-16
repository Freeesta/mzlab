// @ts-check
// TPMINE-PRIVATE  TPV3D: the 3D viewer of Tabui (administrator mode only, used by the panel of reactivity, 46_reattivita.js). Canvas 2D, no WebGL, no library: balls and sticks sorted by depth with simple
// shading, a halo under each atom (radius and opacity proportional to the value of the card), a thick outline and the written index on the atoms above the threshold (never colour alone), hydrogen
// bonds dashed, an on/off switch for the H. Controls: buttons (rotate ← → ↑ ↓, zoom + −, Reset view) and the same keys with the canvas in focus; dragging is an extra, never the only way.
// No automatic rotation (prefers-reduced-motion safe). Colours only from CSS variables (Okabe-Ito, light and dark theme). Texts in English.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  const CSS = `
.tpv3{--v3-C:#6b675c;--v3-H:#d9d6cd;--v3-N:#0072B2;--v3-O:#D55E00;--v3-F:#56B4E9;--v3-S:#c9b800;--v3-P:#E69F00;--v3-Cl:#009E73;--v3-Br:#7a3b00;--v3-I:#5b3f8c;--v3-X:#999999;--v3-halo:#CC79A7;--v3-bond:#8a8578;--v3-hb:#0072B2}
:root[data-theme=dark] .tpv3{--v3-C:#b3b8c0;--v3-H:#e4e7eb;--v3-N:#56B4E9;--v3-O:#ff8f4d;--v3-S:#F0E442;--v3-Cl:#33c79e;--v3-Br:#d8a070;--v3-I:#b79cf0;--v3-halo:#e79fc7;--v3-bond:#98a2b0;--v3-hb:#56B4E9}
@media (prefers-color-scheme:dark){:root:not([data-theme=light]) .tpv3{--v3-C:#b3b8c0;--v3-H:#e4e7eb;--v3-N:#56B4E9;--v3-O:#ff8f4d;--v3-S:#F0E442;--v3-Cl:#33c79e;--v3-Br:#d8a070;--v3-I:#b79cf0;--v3-halo:#e79fc7;--v3-bond:#98a2b0;--v3-hb:#56B4E9}}
.tpv3 canvas{display:block;width:100%;max-width:520px;aspect-ratio:480/340;border:1px solid var(--line);border-radius:6px;background:var(--panel);touch-action:none}
.tpv3 canvas:focus-visible,.tpv3 button:focus-visible,.tpv3 input:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.tpv3 .v3-bar{display:flex;flex-wrap:wrap;gap:6px;align-items:center;margin:6px 0 0;font-size:12px}
.tpv3 .v3-bar button{min-width:28px;min-height:28px;padding:2px 8px;cursor:pointer;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink);font:inherit}
.tpv3 .v3-bar button:hover{background:var(--soft)}
.tpv3 .v3-bar label{display:inline-flex;align-items:center;gap:4px;min-height:28px}
@media(pointer:coarse){.tpv3 .v3-bar button,.tpv3 .v3-bar label{min-height:40px;min-width:40px}}`;
  const injectCss = () => { if (document.getElementById("tpv3-css")) return; const s = document.createElement("style"); s.id = "tpv3-css"; s.textContent = CSS; document.head.appendChild(s); };

  /** radius of the ball by element, in Å (about half of the van der Waals radius) */
  const RAD = /** @type {Record<string, number>} */ ({ H: 0.55, C: 0.85, N: 0.8, O: 0.78, F: 0.7, P: 1.0, S: 1.0, Cl: 0.95, Br: 1.05, I: 1.15 });
  const KNOWN = ["C", "H", "N", "O", "F", "S", "P", "Cl", "Br", "I"];
  const STEP = 15 * Math.PI / 180;

  /** projects the atoms: rotation about the vertical axis (yaw) then the horizontal one (pitch), orthographic @param {{x:number,y:number,z:number}[]} pts @param {number} yaw @param {number} pitch
   *  @returns {{x:number,y:number,z:number}[]} x to the right, y down, z towards the viewer */
  function project(pts, yaw, pitch) {
    const cy = Math.cos(yaw), sy = Math.sin(yaw), cp = Math.cos(pitch), sp = Math.sin(pitch);
    return pts.map(p => { const x1 = p.x * cy + p.z * sy, z1 = -p.x * sy + p.z * cy; return { x: x1, y: p.y * cp - z1 * sp, z: p.y * sp + z1 * cp }; });
  }

  /**
   * @typedef {{ i: number, el: string, x: number, y: number, z: number }} V3Atom
   * @typedef {{ atoms: V3Atom[], bonds?: {a:number,b:number,order?:number}[], hbonds?: {donor_h:number,acceptor:number}[], label?: string }} V3Model
   * @param {HTMLElement} host @param {V3Model} model @param {{ onChange?: () => void }} [opt]
   */
  function create(host, model, opt = {}) {
    injectCss();
    const atoms = model.atoms, at = new Map(atoms.map((a, k) => [a.i, k]));
    const wrap = document.createElement("div"); wrap.className = "tpv3";
    const aria = () => `3D model of ${model.label || "the molecule"}, ${atoms.length} atoms, card ${st.card}. Arrow keys rotate, plus and minus zoom, 0 resets the view, H shows or hides the hydrogens.`;
    wrap.innerHTML = `<canvas tabindex="0" role="img" width="480" height="340"></canvas><div class="v3-bar" role="group" aria-label="3D view controls">`
      + [["left", "Rotate left", "◀"], ["right", "Rotate right", "▶"], ["up", "Rotate up", "▲"], ["down", "Rotate down", "▼"], ["in", "Zoom in", "+"], ["out", "Zoom out", "−"], ["reset", "Reset view", "Reset view"]]
        .map(([k, l, t]) => `<button type="button" data-v3="${k}" aria-label="${l}">${t}</button>`).join("")
      + `<label><input type="checkbox" data-v3="h" checked> Show H</label></div>`;
    host.appendChild(wrap);
    const cv = /** @type {HTMLCanvasElement} */ (wrap.querySelector("canvas")), ctx = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d"));
    const hCheck = /** @type {HTMLInputElement} */ (wrap.querySelector('input[data-v3="h"]'));
    const st = { yaw: 0.5, pitch: -0.3, zoom: 1, showH: true, card: "", thr: 0.05, nOver: 0 };
    /** @type {number[]} */ let values = atoms.map(() => 0);
    /** @type {string[]} */ let labels = atoms.map(a => String(a.i));
    const cx = atoms.reduce((s, a) => s + a.x, 0) / (atoms.length || 1), cy = atoms.reduce((s, a) => s + a.y, 0) / (atoms.length || 1), cz = atoms.reduce((s, a) => s + a.z, 0) / (atoms.length || 1);
    const pts = atoms.map(a => ({ x: a.x - cx, y: a.y - cy, z: a.z - cz }));
    const reach = Math.max(1, ...pts.map(p => Math.hypot(p.x, p.y, p.z))) + 0.9;
    const css = (/** @type {string} */ n) => getComputedStyle(cv).getPropertyValue(n).trim() || "#888";
    const colour = (/** @type {string} */ el) => css("--v3-" + (KNOWN.includes(el) ? el : "X"));

    function sync() {
      cv.dataset.yaw = st.yaw.toFixed(3); cv.dataset.pitch = st.pitch.toFixed(3); cv.dataset.zoom = st.zoom.toFixed(3); cv.dataset.showH = String(st.showH); cv.dataset.over = String(st.nOver);
      cv.setAttribute("aria-label", aria()); hCheck.checked = st.showH;
    }
    function draw() {
      const dpr = window.devicePixelRatio || 1, w = Math.max(200, cv.clientWidth || 480), h = Math.round(w * 340 / 480);
      if (cv.width !== Math.round(w * dpr) || cv.height !== Math.round(h * dpr)) { cv.width = Math.round(w * dpr); cv.height = Math.round(h * dpr); }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0); ctx.clearRect(0, 0, w, h);
      const sc = Math.min(w, h) / 2 / reach * st.zoom, P = project(pts, st.yaw, st.pitch);
      const vis = (/** @type {number} */ k) => st.showH || atoms[k].el !== "H";
      const X = (/** @type {number} */ k) => w / 2 + P[k].x * sc, Y = (/** @type {number} */ k) => h / 2 + P[k].y * sc;
      const vmax = Math.max(1e-9, ...values.map((v, k) => (vis(k) ? v : 0)));
      /** @type {{ z: number, f: () => void }[]} */ const items = [];
      st.nOver = 0;
      for (const b of model.bonds || []) {                  // sticks, drawn as half sticks in the colour of each end
        const a = at.get(b.a), c = at.get(b.b); if (a === undefined || c === undefined || !vis(a) || !vis(c)) continue;
        items.push({ z: (P[a].z + P[c].z) / 2 - 0.01, f: () => {
          const mx = (X(a) + X(c)) / 2, my = (Y(a) + Y(c)) / 2; ctx.lineCap = "round"; ctx.lineWidth = Math.max(2, 0.22 * sc * ((b.order || 1) > 1 ? 1.15 : 1));
          ctx.strokeStyle = colour(atoms[a].el); ctx.beginPath(); ctx.moveTo(X(a), Y(a)); ctx.lineTo(mx, my); ctx.stroke();
          ctx.strokeStyle = colour(atoms[c].el); ctx.beginPath(); ctx.moveTo(mx, my); ctx.lineTo(X(c), Y(c)); ctx.stroke();
          if ((b.order || 1) > 1) { ctx.strokeStyle = css("--panel"); ctx.lineWidth = Math.max(1, 0.07 * sc); ctx.beginPath(); ctx.moveTo(X(a), Y(a)); ctx.lineTo(X(c), Y(c)); ctx.stroke(); }
        } });
      }
      for (const hb of model.hbonds || []) {                 // hydrogen bonds, dashed
        const a = at.get(hb.donor_h), c = at.get(hb.acceptor); if (a === undefined || c === undefined || !vis(a) || !vis(c)) continue;
        items.push({ z: (P[a].z + P[c].z) / 2 - 0.02, f: () => { ctx.save(); ctx.setLineDash([4, 4]); ctx.lineWidth = 1.5; ctx.strokeStyle = css("--v3-hb"); ctx.beginPath(); ctx.moveTo(X(a), Y(a)); ctx.lineTo(X(c), Y(c)); ctx.stroke(); ctx.restore(); } });
      }
      atoms.forEach((a, k) => {
        if (!vis(k)) return;
        const over = values[k] >= st.thr && values[k] > 0; if (over) st.nOver++;
        const r = (RAD[a.el] || 0.9) * 0.62 * sc, v = Math.max(0, values[k]) / vmax;
        items.push({ z: P[k].z, f: () => {
          if (v > 0.001) { const hr = r * (1.3 + 2.2 * v); ctx.save(); ctx.globalAlpha = 0.12 + 0.4 * v; ctx.fillStyle = css("--v3-halo"); ctx.beginPath(); ctx.arc(X(k), Y(k), hr, 0, 6.2832); ctx.fill(); ctx.restore(); }
          const g = ctx.createRadialGradient(X(k) - r * 0.35, Y(k) - r * 0.35, r * 0.1, X(k), Y(k), r), col = colour(a.el);
          g.addColorStop(0, css("--panel")); g.addColorStop(0.35, col); g.addColorStop(1, col);
          ctx.beginPath(); ctx.arc(X(k), Y(k), r, 0, 6.2832); ctx.fillStyle = g; ctx.fill();
          ctx.lineWidth = over ? 3.5 : 1; ctx.strokeStyle = over ? css("--ink") : css("--v3-bond"); ctx.stroke();
          if (over) { ctx.font = "bold 12px system-ui, sans-serif"; ctx.textAlign = "center"; ctx.textBaseline = "middle"; const t = labels[k]; ctx.lineWidth = 3; ctx.strokeStyle = css("--panel"); ctx.strokeText(t, X(k), Y(k) - r - 9); ctx.fillStyle = css("--ink"); ctx.fillText(t, X(k), Y(k) - r - 9); }
        } });
      });
      items.sort((p, q) => p.z - q.z).forEach(it => it.f());
      sync();
    }
    /** @param {number} dyaw @param {number} dpitch */
    const rotate = (dyaw, dpitch) => { st.yaw += dyaw; st.pitch = Math.max(-1.5, Math.min(1.5, st.pitch + dpitch)); draw(); opt.onChange?.(); };
    /** @param {number} f */
    const zoom = f => { st.zoom = Math.max(0.4, Math.min(4, st.zoom * f)); draw(); opt.onChange?.(); };
    const reset = () => { st.yaw = 0.5; st.pitch = -0.3; st.zoom = 1; draw(); opt.onChange?.(); };
    const act = (/** @type {string} */ k) => {
      if (k === "left") rotate(-STEP, 0); else if (k === "right") rotate(STEP, 0); else if (k === "up") rotate(0, -STEP); else if (k === "down") rotate(0, STEP);
      else if (k === "in") zoom(1.25); else if (k === "out") zoom(0.8); else if (k === "reset") reset();
    };
    wrap.querySelectorAll("button[data-v3]").forEach(b => b.addEventListener("click", () => act(/** @type {HTMLElement} */ (b).dataset.v3 || "")));
    hCheck.addEventListener("change", () => { st.showH = hCheck.checked; draw(); });
    cv.addEventListener("keydown", e => {
      const k = ({ ArrowLeft: "left", ArrowRight: "right", ArrowUp: "up", ArrowDown: "down", "+": "in", "=": "in", "-": "out", "0": "reset", Home: "reset" })[/** @type {"+"} */ (e.key)];
      if (k) { e.preventDefault(); act(k); } else if (e.key === "h" || e.key === "H") { e.preventDefault(); st.showH = !st.showH; draw(); }
    });
    /** @type {{x:number,y:number}|null} */ let drag = null;       // dragging is only an addition to the buttons and the keys
    cv.addEventListener("pointerdown", e => { drag = { x: e.clientX, y: e.clientY }; try { cv.setPointerCapture(e.pointerId); } catch (_) { /* not supported */ } });
    cv.addEventListener("pointermove", e => { if (!drag) return; rotate((e.clientX - drag.x) * 0.01, (e.clientY - drag.y) * 0.01); drag = { x: e.clientX, y: e.clientY }; });
    const stop = () => { drag = null; };
    cv.addEventListener("pointerup", stop); cv.addEventListener("pointercancel", stop);
    draw();
    return {
      canvas: cv, rotate, zoom, reset, draw, /** @returns {typeof st} a copy of the state of the view */ state: () => ({ ...st }),
      /** @param {number[]} v values per atom (same order as `atoms`) @param {string} card name of the card @param {number} thr threshold @param {string[]} [lab] written index of each atom */
      setValues(v, card, thr, lab) { values = v.slice(); st.card = card; st.thr = thr; if (lab) labels = lab.slice(); draw(); },
      /** @param {number} thr */ setThreshold(thr) { st.thr = thr; draw(); },
      /** @param {boolean} on */ setShowH(on) { st.showH = on; draw(); },
      destroy() { wrap.remove(); },
    };
  }

  G.TPV3D = { create, project, esc };
})();
