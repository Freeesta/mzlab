// @ts-check
// @ts-strict
// Minimal .xlsx writer (no library): an Excel file is a zip of a few XML files. Here the zip is "stored" (no compression)
// with a CRC32 per file, which every reader accepts. Numbers are real numeric cells, text is written inline.
//
//   xlsxBuild([{ name: "Dati", head: ["RT (min)", "Intensità (cps)"], rows: [[1.2, 3400], ...], widths: [14, 18] }, ...]) -> Uint8Array
//   dlx("nome.xlsx", sheets)       downloads it (works in the local program and in the browser version: Blob)
//
// A cell is a number (NaN/Infinity/null/"" = empty cell), a string, or { v, b: true } for bold. `head` is written in bold and frozen.

const XLSX_MIME = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const _crcT = (() => { const t = new Uint32Array(256); for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } return t; })();
function xlsxCrc32(u8) { let c = 0xFFFFFFFF; for (let i = 0; i < u8.length; i++) c = _crcT[(c ^ u8[i]) & 255] ^ (c >>> 8); return (c ^ 0xFFFFFFFF) >>> 0; }

// zip without compression: files = [[name, Uint8Array], ...]
function zipStore(files) {
  const enc = new TextEncoder(), now = new Date();
  const dtime = (now.getHours() << 11) | (now.getMinutes() << 5) | (now.getSeconds() >> 1), ddate = ((now.getFullYear() - 1980) << 9) | ((now.getMonth() + 1) << 5) | now.getDate();
  const parts = [], central = []; let off = 0;
  for (const [name, data] of files) {
    const nm = enc.encode(name), crc = xlsxCrc32(data);
    const lh = new DataView(new ArrayBuffer(30));
    lh.setUint32(0, 0x04034b50, true); lh.setUint16(4, 20, true); lh.setUint16(6, 0x0800, true); lh.setUint16(8, 0, true);
    lh.setUint16(10, dtime, true); lh.setUint16(12, ddate, true); lh.setUint32(14, crc, true); lh.setUint32(18, data.length, true); lh.setUint32(22, data.length, true);
    lh.setUint16(26, nm.length, true); lh.setUint16(28, 0, true);
    parts.push(new Uint8Array(lh.buffer), nm, data);
    const ch = new DataView(new ArrayBuffer(46));
    ch.setUint32(0, 0x02014b50, true); ch.setUint16(4, 20, true); ch.setUint16(6, 20, true); ch.setUint16(8, 0x0800, true); ch.setUint16(10, 0, true);
    ch.setUint16(12, dtime, true); ch.setUint16(14, ddate, true); ch.setUint32(16, crc, true); ch.setUint32(20, data.length, true); ch.setUint32(24, data.length, true);
    ch.setUint16(28, nm.length, true); ch.setUint32(42, off, true);
    central.push(new Uint8Array(ch.buffer), nm);
    off += 30 + nm.length + data.length;
  }
  const cdSize = central.reduce((s, a) => s + a.length, 0), end = new DataView(new ArrayBuffer(22));
  end.setUint32(0, 0x06054b50, true); end.setUint16(8, files.length, true); end.setUint16(10, files.length, true); end.setUint32(12, cdSize, true); end.setUint32(16, off, true);
  const all = [...parts, ...central, new Uint8Array(end.buffer)], out = new Uint8Array(all.reduce((s, a) => s + a.length, 0));
  let p = 0; for (const a of all) { out.set(a, p); p += a.length; }
  return out;
}

const _xe = s => String(s).replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F￾￿]/g, "").replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const _col = i => { let s = ""; for (i++; i > 0; i = Math.floor((i - 1) / 26)) s = String.fromCharCode(65 + (i - 1) % 26) + s; return s; };

function _cell(v, ref, bold) {
  if (v && typeof v === "object") { bold = v.b || bold; v = v.v; }
  const st = bold ? ' s="1"' : "";
  if (typeof v === "number") return Number.isFinite(v) ? `<c r="${ref}"${st}><v>${v}</v></c>` : "";
  if (v == null || v === "") return "";
  const t = String(v);
  return `<c r="${ref}"${st} t="inlineStr"><is><t${/^\s|\s$|\n/.test(t) ? ' xml:space="preserve"' : ""}>${_xe(t)}</t></is></c>`;
}
function _sheetXml(sh) {
  const rows = sh.head ? [sh.head.map(h => ({ v: h, b: true })), ...sh.rows] : sh.rows;
  const ncol = Math.max(1, ...rows.map(r => r.length));
  const widths = sh.widths || Array.from({ length: ncol }, (_, c) => Math.min(60, Math.max(10, ...rows.slice(0, 200).map(r => { const x = r[c] && typeof r[c] === "object" ? r[c].v : r[c]; return typeof x === "string" ? x.length + 2 : 12; }))));
  let x = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetViews><sheetView workbookViewId="0">` +
    (sh.head ? `<pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/>` : "") + `</sheetView></sheetViews><cols>` +
    widths.map((w, i) => `<col min="${i + 1}" max="${i + 1}" width="${w}" customWidth="1"/>`).join("") + `</cols><sheetData>`;
  rows.forEach((r, i) => {
    const cs = r.map((v, c) => _cell(v, _col(c) + (i + 1), sh.head && i === 0)).join("");
    if (cs) x += `<row r="${i + 1}">${cs}</row>`;
  });
  return x + `</sheetData></worksheet>`;
}

function xlsxBuild(sheets) {
  const enc = new TextEncoder(), used = new Set();
  const names = sheets.map((s, i) => {                       // Excel: at most 31 characters, none of []:*?/\ , unique
    let n = String(s.name || "Foglio" + (i + 1)).replace(/[\[\]:*?\/\\]/g, "-").slice(0, 31) || "Foglio" + (i + 1), k = 2, b = n;
    while (used.has(n.toLowerCase())) n = b.slice(0, 31 - String(k).length - 1) + "_" + k++;
    used.add(n.toLowerCase()); return n;
  });
  const H = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n`, R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships", SM = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
  const files = [
    ["[Content_Types].xml", H + `<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>` +
      sheets.map((_, i) => `<Override PartName="/xl/worksheets/sheet${i + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>`).join("") + `</Types>`],
    ["_rels/.rels", H + `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="${R}/officeDocument" Target="xl/workbook.xml"/></Relationships>`],
    ["xl/workbook.xml", H + `<workbook xmlns="${SM}" xmlns:r="${R}"><sheets>` + names.map((n, i) => `<sheet name="${_xe(n)}" sheetId="${i + 1}" r:id="rId${i + 1}"/>`).join("") + `</sheets></workbook>`],
    ["xl/_rels/workbook.xml.rels", H + `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">` + sheets.map((_, i) => `<Relationship Id="rId${i + 1}" Type="${R}/worksheet" Target="worksheets/sheet${i + 1}.xml"/>`).join("") +
      `<Relationship Id="rId${sheets.length + 1}" Type="${R}/styles" Target="styles.xml"/></Relationships>`],
    ["xl/styles.xml", H + `<styleSheet xmlns="${SM}"><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>`],
    ...sheets.map((s, i) => [`xl/worksheets/sheet${i + 1}.xml`, _sheetXml(s)]),
  ].map(([n, t]) => [n, enc.encode(t)]);
  return zipStore(files);
}
// download as .xlsx (name without extension is completed)
function dlx(name, sheets) { dl(/\.xlsx$/i.test(name) ? name : name + ".xlsx", xlsxBuild(sheets), XLSX_MIME); }
if (typeof module !== "undefined") module.exports = { xlsxBuild, zipStore, xlsxCrc32 };
