// Zoom step of the wheel / trackpad pinch (ASSE.wheelFactor, mzlab/web/asse.js): prints one JSON object of named checks. See tests/test_asse_wheel.py.
const fs = require("fs"), vm = require("vm");
const src = fs.readFileSync(__dirname + "/../mzlab/web/asse.js", "utf8") + "\n;globalThis.ASSE = ASSE;";
const g = { I18N: { t: k => k } }; vm.createContext(g); vm.runInContext(src, g);
const { wheelFactor: f, WHEEL_K, WHEEL_MAX } = g.ASSE, out = {};
const near = (a, b) => Math.abs(a - b) < 1e-9;
out.constants = WHEEL_K === 0.0054 && WHEEL_MAX === 0.24;
out.trackpad_pinch = near(f({ deltaY: -5, deltaMode: 0 }), Math.exp(-5 * 0.0054));       // 35% stronger than the old exp(-5 * 0.004)
out.faster_than_old = Math.log(f({ deltaY: 5 })) / (5 * 0.004) > 1.34;
out.mouse_notch_unchanged = near(f({ deltaY: 100, deltaMode: 0 }), Math.exp(60 * 0.004)) && near(f({ deltaY: -100 }), Math.exp(-60 * 0.004));
out.clamp = near(f({ deltaY: 5000 }), Math.exp(0.24)) && near(f({ deltaY: -5000 }), Math.exp(-0.24));
out.line_mode = near(f({ deltaY: 3, deltaMode: 1 }), Math.exp(0.24));                  // Firefox notch: 3 lines, normalised then clamped
out.page_mode = near(f({ deltaY: 1, deltaMode: 2 }), Math.exp(0.24));
out.zero = f({ deltaY: 0 }) === 1;
console.log(JSON.stringify(out));
