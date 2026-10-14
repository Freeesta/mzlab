/* QqQ lab - Simulazione dinamica e visiva dei tempi del quadrupolo:
   Fascio continuo ESI (3 ioni distinti: A m/z 200, B target m/z 350, C m/z 500).
   Confronto fra scansione continua Full Scan vs selezione MRM e costruzione del picco.
*/
"use strict";

document.addEventListener("DOMContentLoaded", () => {
  const el = document.getElementById("sim-tempi-quad");
  if (!el) return;

  const box = document.createElement("div");
  box.className = "sim-box";
  el.appendChild(box);

  const col = i => getComputedStyle(document.documentElement).getPropertyValue(`--c${i}`).trim() || "#2b5c8a";
  const colMuted = getComputedStyle(document.documentElement).getPropertyValue("--muted").trim() || "#6b675c";
  const colLine = getComputedStyle(document.documentElement).getPropertyValue("--line").trim() || "#e4e2dc";
  const colInk = getComputedStyle(document.documentElement).getPropertyValue("--ink").trim() || "#24231f";
  const colPanel = getComputedStyle(document.documentElement).getPropertyValue("--panel").trim() || "#ffffff";
  const colOk = getComputedStyle(document.documentElement).getPropertyValue("--ok").trim() || "#047857";
  const colWarn = getComputedStyle(document.documentElement).getPropertyValue("--warn").trim() || "#b45309";
  const colBad = getComputedStyle(document.documentElement).getPropertyValue("--bad").trim() || "#b42318";

  let mode = "mrm";
  const btnMrm = document.createElement("button");
  btnMrm.type = "button"; btnMrm.textContent = "MRM (selezione a canali fissi)"; btnMrm.className = "on";
  const btnScan = document.createElement("button");
  btnScan.type = "button"; btnScan.textContent = "Full Scan (sweep continuo 100-900 Da)";
  const modeBar = document.createElement("div");
  modeBar.className = "btns";
  modeBar.appendChild(btnMrm); modeBar.appendChild(btnScan);
  box.appendChild(modeBar);

  const ctlWrap = document.createElement("div");
  box.appendChild(ctlWrap);

  let ctls = null;
  function setupControls() {
    ctlWrap.innerHTML = "";
    const specs = mode === "mrm" ? [
      { id: "dw", label: "Dwell time per canale", min: 5, max: 200, step: 5, value: 30, unit: "ms" },
      { id: "n", label: "Numero transizioni MRM", min: 1, max: 40, step: 1, value: 5, unit: "canali" },
      { id: "anim", label: "Velocità animazione", min: 0.1, max: 1.0, step: 0.05, value: 0.35, unit: "×" },
    ] : [
      { id: "speed", label: "Velocità sweep", min: 500, max: 12000, step: 500, value: 2000, unit: "Da/s" },
      { id: "anim", label: "Velocità animazione", min: 0.1, max: 1.0, step: 0.05, value: 0.35, unit: "×" },
    ];
    ctls = TP.controls(ctlWrap, specs, () => restart());
  }
  setupControls();

  btnMrm.onclick = () => { if (mode !== "mrm") { mode = "mrm"; btnMrm.classList.add("on"); btnScan.classList.remove("on"); setupControls(); restart(); } };
  btnScan.onclick = () => { if (mode !== "scan") { mode = "scan"; btnScan.classList.add("on"); btnMrm.classList.remove("on"); setupControls(); restart(); } };

  const playBar = document.createElement("div");
  playBar.className = "btns";
  const btnPlay = document.createElement("button");
  btnPlay.type = "button"; btnPlay.textContent = "⏸ Pausa";
  const btnRestart = document.createElement("button");
  btnRestart.type = "button"; btnRestart.textContent = "↺ Riavvia corsa (12 s)";
  playBar.appendChild(btnPlay); playBar.appendChild(btnRestart);
  box.appendChild(playBar);
  btnPlay.onclick = () => togglePlay();
  btnRestart.onclick = () => restart();

  const c = TP.canvas(box, 380);
  const { cv, ctx } = c;
  const out = document.createElement("div");
  out.className = "readout";
  box.appendChild(out);

  const T_TOTAL = 12.0, T_CENTER = 6.0, SIGMA = 1.25;
  const peakIdeal = t => Math.exp(-0.5 * Math.pow((t - T_CENTER) / SIGMA, 2));

  let tSim = 0, lastRealTime = performance.now(), paused = false, samples = [], nextSampleTime = 0;

  function getTiming() {
    let cycleSec = 0, dwellMs = 0, dutyPct = 0;
    if (mode === "mrm") {
      dwellMs = ctls.dw;
      const cycleMs = ctls.n * (dwellMs + 6);
      cycleSec = cycleMs / 1000;
      dutyPct = (dwellMs / cycleMs) * 100;
    } else {
      const sweepSec = 800 / ctls.speed;
      cycleSec = sweepSec + 0.010;
      dwellMs = (1.0 / ctls.speed) * 1000;
      dutyPct = (dwellMs / (cycleSec * 1000)) * 100;
    }
    return { cycleSec, dwellMs, dutyPct };
  }

  function restart() {
    tSim = 0; samples = []; nextSampleTime = 0; lastRealTime = performance.now();
    if (paused) draw();
  }

  function togglePlay() {
    paused = !paused; btnPlay.textContent = paused ? "▶ Avvia" : "⏸ Pausa";
    if (!paused) { lastRealTime = performance.now(); loop(); }
  }

  const PARTICLES = Array.from({ length: 42 }, () => ({
    x: Math.random(), y: (Math.random() - 0.5) * 22,
    vx: 0.16 + Math.random() * 0.08, type: Math.floor(Math.random() * 3),
    deflected: false, deflVy: 0,
  }));

  function update(dt) {
    const rate = ctls.anim || 0.35;
    tSim += dt * rate;
    const { cycleSec } = getTiming();
    if (tSim >= nextSampleTime && tSim <= T_TOTAL) {
      samples.push({ t: tSim, val: peakIdeal(tSim) });
      nextSampleTime += cycleSec;
    }
  }

  function getStatus(cycleSec, dwellMs) {
    const cycleMs = cycleSec * 1000, progressMs = (tSim * 1000) % cycleMs;
    let passingId = -1, txt = "", color = colMuted;
    if (mode === "mrm") {
      const stepMs = dwellMs + 6, ch = Math.floor(progressMs / stepMs);
      if ((progressMs % stepMs) < dwellMs) {
        passingId = ch <= 2 ? ch : -1;
        const labels = ["Ione A (m/z 200)", "Target B (m/z 350)", "Ione C (m/z 500)"];
        txt = `Filtra Canale ${ch + 1}: ${labels[ch] || "altre masse"}`;
        color = ch === 0 ? col(1) : (ch === 1 ? col(2) : (ch === 2 ? col(4) : colMuted));
      } else {
        txt = "Settling (6 ms)";
      }
    } else {
      const sweepMs = cycleMs - 10;
      if (progressMs < sweepMs) {
        const mass = 100 + (progressMs / sweepMs) * 800;
        txt = `Sweep: m/z ${TP.fmt(mass, 0)} Da (${ctls.speed} Da/s)`;
        if (Math.abs(mass - 200) < 6) { passingId = 0; color = col(1); }
        else if (Math.abs(mass - 350) < 6) { passingId = 1; color = col(2); }
        else if (Math.abs(mass - 500) < 6) { passingId = 2; color = col(4); }
      } else {
        txt = "Ritorno scala masse (settling 10 ms)";
      }
    }
    return { passingId, txt, color };
  }

  function drawInstrument(W, topH, yMid, xEsi, xQ1, xQ2, xDet, st) {
    ctx.fillStyle = colPanel; ctx.strokeStyle = colLine; ctx.lineWidth = 1;
    ctx.fillRect(8, 8, W - 16, topH); ctx.strokeRect(8, 8, W - 16, topH);

    ctx.beginPath();
    ctx.moveTo(xEsi - 30, yMid - 22); ctx.lineTo(xEsi + 8, yMid - 6);
    ctx.lineTo(xEsi + 8, yMid + 6); ctx.lineTo(xEsi - 30, yMid + 22);
    ctx.closePath(); ctx.fillStyle = colLine; ctx.fill();

    ctx.font = "bold 11px system-ui,sans-serif"; ctx.fillStyle = colInk; ctx.textAlign = "center";
    ctx.fillText("SORGENTE ESI", xEsi - 10, yMid - 28);
    ctx.font = "10px system-ui,sans-serif"; ctx.fillStyle = colMuted;
    ctx.fillText("Flusso continuo", xEsi - 10, yMid + 36);

    ctx.fillStyle = colLine;
    ctx.fillRect(xQ1, yMid - 28, xQ2 - xQ1, 10);
    ctx.fillRect(xQ1, yMid + 18, xQ2 - xQ1, 10);

    ctx.font = "bold 12px system-ui,sans-serif"; ctx.fillStyle = st.color; ctx.textAlign = "center";
    ctx.fillText(st.txt, (xQ1 + xQ2) / 2, yMid - 36);
    ctx.font = "11px system-ui,sans-serif"; ctx.fillStyle = colMuted;
    ctx.fillText("QUADRUPOLO (BARRE RF/DC)", (xQ1 + xQ2) / 2, yMid + 42);

    ctx.fillStyle = colLine;
    ctx.fillRect(xDet - 6, yMid - 24, 12, 48);
    ctx.font = "bold 11px system-ui,sans-serif"; ctx.fillStyle = colInk;
    ctx.fillText("RIVELATORE", xDet, yMid - 30);
    ctx.font = "10px system-ui,sans-serif"; ctx.fillStyle = colMuted;
    ctx.fillText(st.passingId === 1 ? "Conta Target B" : "(attesa)", xDet, yMid + 36);
  }

  function drawParticles(W, dt, yMid, xEsi, xQ1, xQ2, xDet, passingId) {
    const pCols = [col(1), col(2), col(4)];
    const pRad = [4.5, 5.5, 4.5];
    const rate = ctls.anim || 0.35;

    PARTICLES.forEach(p => {
      p.x += p.vx * dt * rate;
      if (p.x > 1.05) {
        p.x = 0; p.y = (Math.random() - 0.5) * 20; p.deflected = false; p.deflVy = 0;
      }
      const px = xEsi + p.x * (xDet - xEsi + 20);
      if (px >= xQ1 && px <= xQ2 && !p.deflected && p.type !== passingId) {
        p.deflected = true;
        p.deflVy = (p.y >= 0 ? 1 : -1) * (35 + Math.random() * 25);
      }
      if (p.deflected) p.y += p.deflVy * dt * rate;
      const py = yMid + p.y;
      if (px <= xDet || (p.type === passingId && !p.deflected)) {
        ctx.beginPath(); ctx.arc(px, py, pRad[p.type], 0, 2 * Math.PI);
        ctx.fillStyle = pCols[p.type]; ctx.fill();
        ctx.strokeStyle = colPanel; ctx.lineWidth = 1; ctx.stroke();
      }
    });
  }

  function drawPlot(W, H, cycleSec, dwellMs, dutyPct) {
    const pTop = 150, pBot = H - 32, pLeft = 55, pRight = W - 25, pW = pRight - pLeft, pH = pBot - pTop;
    const xOfT = t => pLeft + (t / T_TOTAL) * pW;
    const yOfVal = v => pBot - v * (pH - 18);

    ctx.strokeStyle = colLine; ctx.lineWidth = 1;
    ctx.beginPath(); ctx.moveTo(pLeft, pBot); ctx.lineTo(pRight, pBot);
    ctx.moveTo(pLeft, pTop); ctx.lineTo(pLeft, pBot); ctx.stroke();

    ctx.font = "11px system-ui,sans-serif"; ctx.fillStyle = colMuted; ctx.textAlign = "center";
    ctx.fillText("0 s", pLeft, pBot + 16);
    ctx.fillText(`${T_TOTAL / 2} s`, pLeft + pW / 2, pBot + 16);
    ctx.fillText(`${T_TOTAL} s (tempo cromatografico)`, pRight - 45, pBot + 16);

    ctx.save(); ctx.translate(14, pTop + pH / 2); ctx.rotate(-Math.PI / 2);
    ctx.fillText("Segnale Target B", 0, 0); ctx.restore();

    ctx.beginPath();
    for (let s = 0; s <= T_TOTAL; s += 0.05) {
      const x = xOfT(s), y = yOfVal(peakIdeal(s));
      if (s === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = colMuted; ctx.lineWidth = 1.8; ctx.setLineDash([4, 4]); ctx.stroke(); ctx.setLineDash([]);
    ctx.font = "italic 11px system-ui,sans-serif"; ctx.fillStyle = colMuted; ctx.textAlign = "left";
    ctx.fillText("– – – Picco teorico Target B (eluizione continua)", pLeft + 10, pTop + 14);

    if (samples.length > 0) {
      ctx.beginPath();
      samples.forEach((sp, idx) => {
        const x = xOfT(sp.t), y = yOfVal(sp.val);
        if (idx === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
      });
      ctx.strokeStyle = samples.length >= 12 ? colOk : (samples.length >= 7 ? colWarn : colBad);
      ctx.lineWidth = 2.2; ctx.stroke();

      samples.forEach(sp => {
        ctx.beginPath(); ctx.arc(xOfT(sp.t), yOfVal(sp.val), 4, 0, 2 * Math.PI);
        ctx.fillStyle = col(2); ctx.fill(); ctx.strokeStyle = colPanel; ctx.lineWidth = 1.2; ctx.stroke();
      });
    }

    const curX = xOfT(Math.min(tSim, T_TOTAL));
    ctx.strokeStyle = col(1); ctx.lineWidth = 1.4; ctx.setLineDash([2, 2]);
    ctx.beginPath(); ctx.moveTo(curX, pTop); ctx.lineTo(curX, pBot); ctx.stroke(); ctx.setLineDash([]);
    ctx.beginPath(); ctx.arc(curX, pBot, 4, 0, 2 * Math.PI); ctx.fillStyle = col(1); ctx.fill();

    const nPts = samples.length;
    let bText = `Punti acquisiti Target B: ${nPts}`, bCol = colMuted;
    if (tSim >= T_TOTAL) {
      bText = nPts >= 14 ? `Fine picco: ${nPts} punti (✔ Ottimo)` : (nPts >= 8 ? `Fine picco: ${nPts} punti (⚠ Al limite)` : `Fine picco: ${nPts} punti (✖ Inaccettabile)`);
      bCol = nPts >= 14 ? colOk : (nPts >= 8 ? colWarn : colBad);
    }
    ctx.font = "bold 12px system-ui,sans-serif"; ctx.fillStyle = bCol; ctx.textAlign = "right";
    ctx.fillText(bText, pRight - 6, pTop + 14);

    out.innerHTML = `
      <div style="display:flex;flex-wrap:wrap;gap:16px;justify-content:space-between;font-size:14.5px">
        <div><b>Tempo di ciclo</b>: ${TP.fmt(cycleSec * 1000, 0)} ms (${TP.fmt(cycleSec, 2)} s)</div>
        <div><b>Dwell su Target B</b>: ${TP.fmt(dwellMs, 1)} ms</div>
        <div><b>Duty cycle su B</b>: <b style="color:${dutyPct > 5 ? colOk : colBad}">${TP.fmt(dutyPct, 1)} %</b></div>
        <div><b>Frequenza</b>: ${TP.fmt(1 / cycleSec, 1)} Hz</div>
      </div>
    `;
  }

  function draw() {
    const W = c.W, H = c.H, topH = 120, yMid = 8 + topH / 2;
    const xEsi = 75, xQ1 = W * 0.36, xQ2 = W * 0.72, xDet = W - 55;
    ctx.clearRect(0, 0, W, H);
    const { cycleSec, dwellMs, dutyPct } = getTiming();
    const st = getStatus(cycleSec, dwellMs);
    drawInstrument(W, topH, yMid, xEsi, xQ1, xQ2, xDet, st);
    drawParticles(W, 0.016, yMid, xEsi, xQ1, xQ2, xDet, st.passingId);
    drawPlot(W, H, cycleSec, dwellMs, dutyPct);
  }

  function loop() {
    const now = performance.now(), dt = (now - lastRealTime) / 1000;
    lastRealTime = now;
    update(Math.min(dt, 0.1));
    draw();
    if (!paused) requestAnimationFrame(loop);
  }

  c.onresize = draw;
  loop();
});
