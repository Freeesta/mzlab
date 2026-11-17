// @ts-strict
// One way to show errors: an uncaught `error` or `unhandledrejection` becomes a short translated message in a role=alert box with a
// «Copy details» button (version, route, stack). Nothing is hidden: the browser console still gets the original error.
const BOX = "errbox";
const seen = new Set();
let count = 0;
/** @type {string[]} */
const details = [];
/** @type {string[]} one short line per error (message only, no stack) */
const brief = [];
/** @type {{where: string, message: string, stack: string}[]} the last 20 errors for the bug report (the report cuts names and URLs: bug-report-testo.js) */
const recent = [];
/** @type {string[]} the last 15 clicks, as the id of the button (never its text, never what was typed) */
const clicks = [];

/** @param {unknown} e */
function describe(e) {
  if (e instanceof Error) return (e.stack || e.name + ": " + e.message);
  try { return typeof e === "string" ? e : JSON.stringify(e); } catch (_) { return String(e); }
}

function header() {
  const w = /** @type {any} */ (window);
  return ["mzLab " + (w.MZLAB_VERSION || "?") + " (" + (w.MZLAB_COMMIT || "?") + ")", location.hash || "#/", navigator.userAgent, new Date().toISOString()].join("\n");
}

/** @param {string} k */
const tr = k => (window.I18N ? I18N.t(k) : k);

/** @param {unknown} e  @param {string} [where] */
export function segnala(e, where = "") {
  const text = describe(e);
  const msg = e instanceof Error ? e.message : String(e);
  if (/ResizeObserver loop/.test(msg) || seen.has(text)) return;
  seen.add(text);
  details.push((where ? "[" + where + "] " : "") + text);
  if (details.length > 20) details.shift();
  brief.push(((where ? "[" + where + "] " : "") + msg).split(/\r?\n/)[0].slice(0, 300));
  if (brief.length > 20) brief.shift();
  recent.push({ where, message: msg.split(/\r?\n/)[0].slice(0, 300), stack: e instanceof Error ? String(e.stack || "") : "" });
  if (recent.length > 20) recent.shift();
  count++;
  let box = document.getElementById(BOX);
  if (!box) {
    box = document.createElement("div");
    box.id = BOX;
    box.setAttribute("role", "alert");
    box.style.cssText = "position:fixed;left:12px;bottom:12px;z-index:100001;max-width:min(460px,calc(100vw - 24px));display:flex;gap:8px;align-items:center;flex-wrap:wrap;padding:8px 12px;background:var(--panel);color:var(--ink);border:1px solid var(--bad);border-left:4px solid var(--bad);border-radius:8px;box-shadow:0 6px 20px rgba(0,0,0,.22);font-size:13px";
    const m = document.createElement("span");
    m.className = "errmsg"; m.style.flex = "1 1 220px";
    const cp = document.createElement("button");
    cp.type = "button"; cp.className = "errcopy"; cp.style.minHeight = "24px";
    const x = document.createElement("button");
    x.type = "button"; x.className = "errx"; x.style.minHeight = "24px";
    cp.onclick = async () => {
      const t = header() + "\n\n" + details.join("\n\n");
      try { await navigator.clipboard.writeText(t); cp.textContent = tr("errori.copied"); }
      catch (_) { window.prompt(tr("errori.copy"), t); }
    };
    x.onclick = () => { /** @type {HTMLElement} */ (box).remove(); count = 0; };
    box.append(m, cp, x);
    document.body.appendChild(box);
  }
  box.querySelector(".errmsg").textContent = tr("errori.msg") + (count > 1 ? " (" + count + ")" : "");
  box.querySelector(".errcopy").textContent = tr("errori.copy");
  const x = box.querySelector(".errx");
  x.textContent = "×"; x.setAttribute("aria-label", tr("errori.close"));
}

addEventListener("error", ev => segnala(ev.error || ev.message, "error"));
addEventListener("unhandledrejection", ev => segnala(ev.reason, "promise"));
// breadcrumb: the id of the control the user last pressed (or its data-v / data-a / data-tab), else its tag; capture phase, no text, nothing sent
addEventListener("click", ev => {
  const el = ev.target instanceof Element ? ev.target.closest("button,a,summary,[role=button],[role=tab],input[type=checkbox],input[type=radio]") : null;
  if (!el) return;
  const k = ["data-v", "data-a", "data-tab"].find(a => el.getAttribute(a));
  clicks.push(el.id ? "#" + el.id : k ? `${el.tagName.toLowerCase()}[${k}=${el.getAttribute(k)}]` : el.tagName.toLowerCase());
  if (clicks.length > 15) clicks.shift();
}, true);
/** @type {any} */ (window).ERRORI = { segnala, recenti: (/** @type {number} */ n) => brief.slice(-n), ultimi: (/** @type {number} */ n) => recent.slice(-n), azioni: (/** @type {number} */ n) => clicks.slice(-n) };
