// The xlsx reader of the Import data card (flow/lazy/flow-xlsx.js): a workbook built here with the zip format and read back. Exit code 1 on the first failure
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert"), zlib = require("zlib");
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const ctx = vm.createContext({ TextDecoder, Response, Blob, DecompressionStream, Uint8Array, DataView, Error, Map, String, Number, Math, parseInt });
ctx.FLOW = {}; ctx.globalThis = ctx;
vm.runInContext(fs.readFileSync(path.resolve(__dirname, "../flow/lazy/flow-xlsx.js"), "utf8"), ctx);
const crc = b => { let c, t = [], r = ~0; for (let n = 0; n < 256; n++) { c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } for (const x of b) r = t[(r ^ x) & 255] ^ (r >>> 8); return ~r >>> 0; };
const zip = files => {
  const loc = [], cen = []; let at = 0;
  for (const [name, text, stored] of files) {
    const raw = Buffer.from(text), body = stored ? raw : zlib.deflateRawSync(raw), nm = Buffer.from(name), h = Buffer.alloc(30);
    h.writeUInt32LE(0x04034b50, 0); h.writeUInt16LE(20, 4); h.writeUInt16LE(stored ? 0 : 8, 8); h.writeUInt32LE(crc(raw), 14); h.writeUInt32LE(body.length, 18); h.writeUInt32LE(raw.length, 22); h.writeUInt16LE(nm.length, 26);
    const c = Buffer.alloc(46); c.writeUInt32LE(0x02014b50, 0); c.writeUInt16LE(20, 4); c.writeUInt16LE(20, 6); c.writeUInt16LE(stored ? 0 : 8, 10); c.writeUInt32LE(crc(raw), 16); c.writeUInt32LE(body.length, 20); c.writeUInt32LE(raw.length, 24); c.writeUInt16LE(nm.length, 28); c.writeUInt32LE(at, 42);
    loc.push(h, nm, body); cen.push(c, nm); at += 30 + nm.length + body.length;
  }
  const cd = Buffer.concat(cen), e = Buffer.alloc(22); e.writeUInt32LE(0x06054b50, 0); e.writeUInt16LE(files.length, 8); e.writeUInt16LE(files.length, 10); e.writeUInt32LE(cd.length, 12); e.writeUInt32LE(at, 16);
  const all = Buffer.concat([...loc, cd, e]); return all.buffer.slice(all.byteOffset, all.byteOffset + all.length);
};
const sheet = '<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c><c r="C1" t="s"><v>2</v></c></row>'
  + '<row r="2"><c r="A2" t="s"><v>3</v></c><c r="C2"><v>30</v></c></row><row r="3"><c r="A3" t="inlineStr"><is><t>c &amp; d.mzML</t></is></c><c r="B3" t="s"><v>4</v></c><c r="D3"><v>2</v></c></row></sheetData></worksheet>';
const strings = '<sst><si><t>File</t></si><si><t>Role</t></si><si><t>Time (min)</t></si><si><t>a.mzML</t></si><si><r><t>bla</t></r><r><t>nk</t></r></si></sst>';
(async () => {
  for (const stored of [false, true]) {
    const buf = zip([["xl/sharedStrings.xml", strings, stored], ["xl/worksheets/sheet1.xml", sheet, stored], ["xl/worksheets/sheet2.xml", "<worksheet><sheetData/></worksheet>", stored]]);
    eq(await ctx.FLOW.readXlsx(buf), [["File", "Role", "Time (min)"], ["a.mzML", "", "30"], ["c & d.mzML", "blank", "", "2"]]);
  }
  eq(await ctx.FLOW.xlsxToText(zip([["xl/sharedStrings.xml", strings], ["xl/worksheets/sheet1.xml", sheet]])), "File\tRole\tTime (min)\na.mzML\t\t30\nc & d.mzML\tblank\t\t2");
  await assert.rejects(() => ctx.FLOW.readXlsx(new Uint8Array(60).buffer), /not an xlsx/);
  console.log("ok");
})().catch(e => { console.error(e); process.exit(1); });
