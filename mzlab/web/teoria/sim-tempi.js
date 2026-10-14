/* mzLab - Teoria: simulazione dinamica del campionamento del picco cromatografico.
   Mostra in tempo reale come il flusso continuo della sorgente ESI (più ioni diversi co-eluenti)
   viene filtrato a turno dal quadrupolo (Full scan vs MRM) e come si costruisce
   il picco dell'analita target rispetto alla curva teorica continua. */
"use strict";

document.addEventListener("DOMContentLoaded", () => {
  const box = document.getElementById("sim-tempi-quad");
  if (!box) return;

  const css = getComputedStyle(document.documentElement);
  const col = n => css.getPropertyValue("--c" + n).trim() || "#2b5c8a";
  const colOk = css.getPropertyValue("--ok").trim() || "#047857";
  const colBad = css.getPropertyValue("--bad").trim() || "#b42318";
  const colWarn = css.getPropertyValue("--warn").trim() || "#b45309";
  const colMuted = css.getPropertyValue("--muted").trim() || "#6b675c";
  const colLine = css.getPropertyValue("--line").trim() || "#e4e2dc";
  const colPanel = css.getPropertyValue("--panel").trim() || "#ffffff";
  const colInk = css.getPropertyValue("--ink").trim() || "#24231f";

  box.innerHTML = "";

  let mode = "mrm"; // "mrm" | "full"

  // 3 ioni distinti che escono contemporaneamente dalla sorgente continua
  const IONS = [
    { id: 0, mz: 200, label: "Ione A (m/z 200)", color: col(1) },
    { id: 1, mz: 350, label: "Target B (m/z 350)", color: col(2) },
    { id: 2, mz: 500, label: "Ione C (m/z 500)", color: col(4) }
  ];

  // Barra comandi: Modi + Play/Pausa + Riavvia
  const bar = document.createElement("div");
  bar.className = "row";
  bar.style.display = "flex";
  bar.style.flexWrap = "wrap";
  bar.style.gap = "8px";
  bar.style.alignItems = "center";
  bar.style.marginBottom = "8px";
  box.appendChild(bar);

  const btnMrm = document.createElement("button");
  btnMrm.type = "button";
  btnMrm.textContent = "SIM / MRM (Canali a turno)";
  btnMrm.className = "btn";

  const btnFull = document.createElement("button");
  btnFull.type = "button";
  btnFull.textContent = "Full Scan (Sweep continuo)";
  btnFull.className = "btn";

  const setModeBtn = () => {
    btnMrm.setAttribute("aria-pressed", String(mode === "mrm"));
    btnFull.setAttribute("aria-pressed", String(mode === "full"));
    btnMrm.style.fontWeight = mode === "mrm" ? "700" : "400";
    btnFull.style.fontWeight = mode === "full" ? "700" : "400";
    btnMrm.style.borderColor = mode === "mrm" ? "var(--accent)" : "";
    btnFull.style.borderColor = mode === "full" ? "var(--accent)" : "";
  };
  setModeBtn();

  btnMrm.onclick = () => { mode = "mrm"; setModeBtn(); toggleCtls(); restart(); };
  btnFull.onclick = () => { mode = "full"; setModeBtn(); toggleCtls(); restart(); };
  bar.append(btnMrm, btnFull);

  const btnPlay = document.createElement("button");
  btnPlay.type = "button";
  btnPlay.textContent = "⏸ Pausa";
  btnPlay.style.marginLeft = "auto";
  btnPlay.onclick = () => togglePlay();
  bar.appendChild(btnPlay);

  const btnRestart = document.createElement("button");
  btnRestart.type = "button";
  btnRestart.textContent = "↺ Ricomincia";
  btnRestart.onclick = () => restart();
  bar.appendChild(btnRestart);

  // Controlli essenziali
  const ctls = TP.controls(box, [
    { id: "n", label: "Transizioni / analiti a turno (MRM)", min: 1, max: 20, step: 1, value: 3, fmt: x => x },
    { id: "dw", label: "Dwell time per canale (MRM)", min: 10, max: 200, step: 10, value: 60, unit: "ms", fmt: x => x },
    { id: "speed", label: "Velocità di sweep (Full Scan)", min: 500, max: 8000, step: 500, value: 2000, unit: "Da/s", fmt: x => x },
    { id: "anim", label: "Velocità animazione", min: 0.1, max: 1.2, step: 0.1, value: 0.4, fmt: x => TP.fmt(x, 1) + "×" },
  ], () => { restart(); });

  function toggleCtls() {
    const cN = box.querySelector("[data-ctl=n]");
    const cDw = box.querySelector("[data-ctl=dw]");
    const cSpeed = box.querySelector("[data-ctl=speed]");
    if (cN) cN.style.display = mode === "mrm" ? "" : "none";
    if (cDw) cDw.style.display = mode === "mrm" ? "" : "none";
    if (cSpeed) cSpeed.style.display = mode === "full" ? "" : "none";
  }
  toggleCtls();

  // Canvas unico grande
  const c = TP.canvas(box, 360);
  const out = document.createElement("div");
  out.className = "readout";
  box.appendChild(out);

  const T_TOTAL = 12.0; // secondi totali della corsa simulata
  const T_CENTER = 6.0; // apice del picco cromatografico
  const SIGMA = 1.25;   // larghezza picco (FWHM ~ 3 s)

  function peakIdeal(t) {
    const u = (t - T_CENTER) / SIGMA;
    return Math.exp(-0.5 * u * u);
  }

  let tSim = 0;
  let lastRealTime = performance.now();
  let paused = false;
  let rafId = null;

  // Campioni acquisiti dello ione target (B, arancione)
  let samples = [];
  let nextSampleTime = 0;

  function getTiming() {
    let cycleSec = 0, dwellMs = 0, dutyPct = 0;
    if (mode === "mrm") {
      const pauseMs = 6;
      dwellMs = ctls.dw;
      const cycleMs = ctls.n * (dwellMs + pauseMs);
      cycleSec = cycleMs / 1000;
      dutyPct = (dwellMs / cycleMs) * 100;
    } else {
      const rangeDa = 800; // es. 100 - 900 Da
      const sweepSec = rangeDa / ctls.speed;
      cycleSec = sweepSec + 0.010; // +10 ms settling
      dwellMs = (1.0 / ctls.speed) * 1000; // ~1 Da
      dutyPct = (dwellMs / (cycleSec * 1000)) * 100;
    }
    const ptsOnPeak = (SIGMA * 2.355 * 2) / cycleSec;
    return { cycleSec, dwellMs, dutyPct, ptsOnPeak };
  }

  function restart() {
    tSim = 0;
    samples = [];
    nextSampleTime = 0;
    lastRealTime = performance.now();
    if (paused) draw();
  }

  function togglePlay() {
    paused = !paused;
    btnPlay.textContent = paused ? "▶ Avvia" : "⏸ Pausa";
    if (!paused) {
      lastRealTime = performance.now();
      loop();
    }
  }

  // Particelle ioniche continue (3 tipi: 0=blu, 1=arancione target, 2=viola)
  const PARTICLES = Array.from({ length: 42 }, (_, i) => ({
    x: Math.random(),
    y: (Math.random() - 0.5) * 22,
    speed: 0.65 + Math.random() * 0.45,
    type: i % 3 // 0, 1 o 2
  }));

  function update(dtReal) {
    if (paused) return;

    const timeScale = ctls.anim !== undefined ? ctls.anim : 0.4;
    const dtSim = dtReal * timeScale;
    tSim += dtSim;

    const { cycleSec, dwellMs } = getTiming();

    // Campionamento del Target B a ogni ciclo
    while (tSim >= nextSampleTime && nextSampleTime <= T_TOTAL) {
      const trueVal = peakIdeal(nextSampleTime);
      const noiseAmp = mode === "full" ? 0.045 : (0.012 * Math.sqrt(60 / Math.max(10, dwellMs)));
      const noise = (Math.random() - 0.5) * noiseAmp;
      samples.push({ t: nextSampleTime, val: Math.max(0, trueVal + noise) });
      nextSampleTime += cycleSec;
    }

    if (tSim > T_TOTAL + 1.2) {
      restart();
    }

    // Avanzamento particelle (fluido e calmo)
    const pSpeedFactor = Math.max(0.2, (timeScale / 0.4) * 0.55);
    PARTICLES.forEach(p => {
      p.x += p.speed * dtReal * pSpeedFactor;
      if (p.x > 1) {
        p.x = 0;
        p.y = (Math.random() - 0.5) * 22;
      }
    });
  }

  function draw() {
    const { ctx, W, H } = c;
    ctx.clearRect(0, 0, W, H);

    const { cycleSec, dwellMs, dutyPct } = getTiming();

    // Calcolo di quale ione viene trasmesso in questo millisecondo
    let currentPassingId = -1; // -1: nessuno (settling o massa diversa da A, B, C)
    let quadStatusText = "";
    let quadStatusColor = colMuted;
    let currentMass = 0;

    const cycleMs = cycleSec * 1000;
    const progressMs = (tSim * 1000) % cycleMs;

    if (mode === "mrm") {
      const stepMs = dwellMs + 6; // dwell + pause
      const activeChannel = Math.floor(progressMs / stepMs);
      const inDwell = (progressMs % stepMs) < dwellMs;

      if (inDwell) {
        if (activeChannel === 0) {
          currentPassingId = 0; // Ione A
          quadStatusText = "Filtra Canale 1: Ione A (m/z 200)";
          quadStatusColor = col(1);
        } else if (activeChannel === 1) {
          currentPassingId = 1; // Target B
          quadStatusText = "Filtra Canale 2: Target B (m/z 350)";
          quadStatusColor = col(2);
        } else if (activeChannel === 2) {
          currentPassingId = 2; // Ione C
          quadStatusText = "Filtra Canale 3: Ione C (m/z 500)";
          quadStatusColor = col(4);
        } else {
          currentPassingId = -1;
          quadStatusText = `Filtra Canale ${activeChannel + 1} (altre masse)`;
          quadStatusColor = colMuted;
        }
      } else {
        currentPassingId = -1;
        quadStatusText = "Inter-scan delay (settling 6 ms)";
        quadStatusColor = colMuted;
      }
    } else {
      // Full Scan
      const sweepMs = cycleMs - 10;
      if (progressMs < sweepMs) {
        const frac = progressMs / sweepMs;
        currentMass = 100 + frac * 800; // da 100 a 900 Da
        quadStatusText = `Sweep: m/z ${TP.fmt(currentMass, 0)} Da (${ctls.speed} Da/s)`;

        if (Math.abs(currentMass - 200) < 6) {
          currentPassingId = 0;
          quadStatusColor = col(1);
        } else if (Math.abs(currentMass - 350) < 6) {
          currentPassingId = 1;
          quadStatusColor = col(2);
        } else if (Math.abs(currentMass - 500) < 6) {
          currentPassingId = 2;
          quadStatusColor = col(4);
        } else {
          currentPassingId = -1;
          quadStatusColor = colMuted;
        }
      } else {
        currentPassingId = -1;
        quadStatusText = "Ritorno scala masse (settling 10 ms)";
        quadStatusColor = colMuted;
      }
    }

    // ============================================================== 1. STRUMENTO (IN ALTO)
    const topH = 120;
    ctx.fillStyle = colPanel;
    ctx.strokeStyle = colLine;
    ctx.lineWidth = 1;
    ctx.fillRect(8, 8, W - 16, topH);
    ctx.strokeRect(8, 8, W - 16, topH);

    const yMid = 8 + topH / 2;
    const xEsi = 75;
    const xQ1 = W * 0.36;
    const xQ2 = W * 0.72;
    const xDet = W - 55;

    // Cono ESI
    ctx.beginPath();
    ctx.moveTo(xEsi - 30, yMid - 22);
    ctx.lineTo(xEsi + 8, yMid - 6);
    ctx.lineTo(xEsi + 8, yMid + 6);
    ctx.lineTo(xEsi - 30, yMid + 22);
    ctx.closePath();
    ctx.fillStyle = colLine;
    ctx.fill();

    ctx.font = "bold 11px system-ui,sans-serif";
    ctx.fillStyle = colInk;
    ctx.textAlign = "center";
    ctx.fillText("SORGENTE ESI", xEsi - 10, yMid - 28);
    ctx.font = "10px system-ui,sans-serif";
    ctx.fillStyle = colMuted;
    ctx.fillText("Flusso continuo", xEsi - 10, yMid + 36);

    // Quadrupolo
    const hasPassing = currentPassingId >= 0;
    ctx.fillStyle = hasPassing ? (currentPassingId === 1 ? "rgba(194, 65, 12, 0.12)" : "rgba(43, 92, 138, 0.10)") : "rgba(180, 35, 24, 0.05)";
    ctx.fillRect(xQ1, yMid - 28, xQ2 - xQ1, 56);
    ctx.strokeStyle = hasPassing ? quadStatusColor : colLine;
    ctx.lineWidth = 1.5;
    ctx.strokeRect(xQ1, yMid - 28, xQ2 - xQ1, 56);

    // Barre metalliche
    ctx.fillStyle = colLine;
    ctx.fillRect(xQ1, yMid - 28, xQ2 - xQ1, 7);
    ctx.fillRect(xQ1, yMid + 21, xQ2 - xQ1, 7);

    // Testi stato quadrupolo
    ctx.font = "bold 11.5px system-ui,sans-serif";
    ctx.textAlign = "center";
    ctx.fillStyle = quadStatusColor;
    ctx.fillText(quadStatusText.toUpperCase(), (xQ1 + xQ2) / 2, yMid - 34);

    ctx.font = "10.5px system-ui,sans-serif";
    if (hasPassing) {
      ctx.fillStyle = quadStatusColor;
      ctx.fillText(`✔ TRASMETTE SOLO ${IONS[currentPassingId].label}`, (xQ1 + xQ2) / 2, yMid + 4);
    } else {
      ctx.fillStyle = colBad;
      ctx.fillText("✖ TUTTI GLI IONI SBATTONO SULLE BARRE", (xQ1 + xQ2) / 2, yMid + 4);
    }

    // Rivelatore (Detector)
    ctx.fillStyle = hasPassing ? quadStatusColor : colLine;
    ctx.strokeStyle = hasPassing ? quadStatusColor : colMuted;
    ctx.lineWidth = 1.5;
    ctx.fillRect(xDet - 15, yMid - 24, 22, 48);
    ctx.strokeRect(xDet - 15, yMid - 24, 22, 48);
    ctx.fillStyle = hasPassing ? "#ffffff" : colInk;
    ctx.font = "bold 10px system-ui,sans-serif";
    ctx.fillText("DET", xDet - 4, yMid + 3);

    // Particelle che si muovono
    PARTICLES.forEach(p => {
      const px = (xEsi + 10) + p.x * (xDet - xEsi - 22);
      let py = yMid + p.y;

      const insideQuad = px >= xQ1 && px <= xQ2;
      const pastQuad = px > xQ2;
      const isThisPassing = (p.type === currentPassingId);

      if (insideQuad) {
        if (!isThisPassing) {
          // Deviazione rapida verso le barre
          py += (p.y >= 0 ? 1 : -1) * (px - xQ1) * 0.42;
        } else {
          // Traiettoria stabile focalizzata
          py = yMid + p.y * 0.25;
        }
      } else if (pastQuad) {
        if (!isThisPassing) return; // sparita sulle barre!
        py = yMid + p.y * 0.25;
      }

      ctx.beginPath();
      ctx.arc(px, py, p.type === 1 ? 3.4 : 2.8, 0, 2 * Math.PI);
      ctx.fillStyle = IONS[p.type].color;
      ctx.fill();
    });

    // Legenda colori ioni (in basso nel riquadro strumento)
    const legX = xQ1;
    ctx.font = "10.5px system-ui,sans-serif";
    ctx.textAlign = "left";
    IONS.forEach((ion, i) => {
      const lx = legX + i * 115;
      ctx.beginPath();
      ctx.arc(lx, yMid + 45, 3.5, 0, 2 * Math.PI);
      ctx.fillStyle = ion.color;
      ctx.fill();
      ctx.fillStyle = colInk;
      ctx.fillText(ion.label, lx + 7, yMid + 48);
    });

    // ============================================================== 2. PICCO CROMATOGRAFICO (SOTTO)
    const pTop = 150;
    const pBot = H - 32;
    const pLeft = 55;
    const pRight = W - 25;
    const pW = pRight - pLeft;
    const pH = pBot - pTop;

    const xOfT = t => pLeft + (t / T_TOTAL) * pW;
    const yOfVal = v => pBot - v * (pH - 18);

    // Assi
    ctx.strokeStyle = colLine;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(pLeft, pBot);
    ctx.lineTo(pRight, pBot);
    ctx.moveTo(pLeft, pTop);
    ctx.lineTo(pLeft, pBot);
    ctx.stroke();

    ctx.font = "11px system-ui,sans-serif";
    ctx.fillStyle = colMuted;
    ctx.textAlign = "center";
    ctx.fillText("0 s", pLeft, pBot + 16);
    ctx.fillText(`${T_TOTAL / 2} s`, pLeft + pW / 2, pBot + 16);
    ctx.fillText(`${T_TOTAL} s (tempo cromatografico)`, pRight - 45, pBot + 16);

    ctx.save();
    ctx.translate(14, pTop + pH / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText("Segnale Target B", 0, 0);
    ctx.restore();

    // Curva teorica continua (Target B)
    ctx.beginPath();
    for (let s = 0; s <= T_TOTAL; s += 0.05) {
      const x = xOfT(s);
      const y = yOfVal(peakIdeal(s));
      if (s === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = colMuted;
    ctx.lineWidth = 1.8;
    ctx.setLineDash([4, 4]);
    ctx.stroke();
    ctx.setLineDash([]);

    ctx.font = "italic 11px system-ui,sans-serif";
    ctx.fillStyle = colMuted;
    ctx.textAlign = "left";
    ctx.fillText("– – –  Picco teorico Target B (eluizione continua)", pLeft + 10, pTop + 14);

    // Punti campionati registrati
    if (samples.length > 0) {
      ctx.beginPath();
      samples.forEach((sp, idx) => {
        const x = xOfT(sp.t);
        const y = yOfVal(sp.val);
        if (idx === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
      });
      const ptsCount = samples.length;
      ctx.strokeStyle = ptsCount >= 12 ? colOk : (ptsCount >= 7 ? colWarn : colBad);
      ctx.lineWidth = 2.2;
      ctx.stroke();

      samples.forEach(sp => {
        const x = xOfT(sp.t);
        const y = yOfVal(sp.val);
        ctx.beginPath();
        ctx.arc(x, y, 4, 0, 2 * Math.PI);
        ctx.fillStyle = col(2); // colore Target B
        ctx.fill();
        ctx.strokeStyle = colPanel;
        ctx.lineWidth = 1.2;
        ctx.stroke();
      });
    }

    // Linea verticale del tempo corrente
    const curX = xOfT(Math.min(tSim, T_TOTAL));
    ctx.strokeStyle = col(1);
    ctx.lineWidth = 1.4;
    ctx.setLineDash([2, 2]);
    ctx.beginPath();
    ctx.moveTo(curX, pTop);
    ctx.lineTo(curX, pBot);
    ctx.stroke();
    ctx.setLineDash([]);

    ctx.beginPath();
    ctx.arc(curX, pBot, 4, 0, 2 * Math.PI);
    ctx.fillStyle = col(1);
    ctx.fill();

    // Badge esito
    const nPts = samples.length;
    let badgeText = `Punti acquisiti Target B: ${nPts}`;
    let badgeCol = colMuted;
    if (tSim >= T_TOTAL) {
      if (nPts >= 14) {
        badgeText = `Fine picco: ${nPts} punti (✔ Ottimo per quantificare)`;
        badgeCol = colOk;
      } else if (nPts >= 8) {
        badgeText = `Fine picco: ${nPts} punti (⚠ Al limite, picco squadrato)`;
        badgeCol = colWarn;
      } else {
        badgeText = `Fine picco: ${nPts} punti (✖ Inaccettabile: apice perso)`;
        badgeCol = colBad;
      }
    }
    ctx.font = "bold 12px system-ui,sans-serif";
    ctx.fillStyle = badgeCol;
    ctx.textAlign = "right";
    ctx.fillText(badgeText, pRight - 6, pTop + 14);

    // ============================================================== SINTESI SOTTO
    out.innerHTML = `
      <div style="display:flex;flex-wrap:wrap;gap:16px;justify-content:space-between;font-size:14.5px">
        <div><b>Tempo di ciclo</b>: ${TP.fmt(cycleSec * 1000, 0)} ms (${TP.fmt(cycleSec, 2)} s)</div>
        <div><b>Dwell su Target B</b>: ${TP.fmt(dwellMs, 1)} ms</div>
        <div><b>Duty cycle su B</b>: <b style="color:${dutyPct > 5 ? colOk : colBad}">${TP.fmt(dutyPct, 1)} %</b></div>
        <div><b>Frequenza</b>: ${TP.fmt(1 / cycleSec, 1)} Hz</div>
      </div>
    `;
  }

  function loop() {
    const now = performance.now();
    const dt = (now - lastRealTime) / 1000;
    lastRealTime = now;

    update(Math.min(dt, 0.1));
    draw();

    if (!paused) {
      rafId = requestAnimationFrame(loop);
    }
  }

  c.onresize = draw;
  loop();
});
