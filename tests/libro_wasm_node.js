// Converts every chapter of mzlab/web/teoria/src through the mzlab-libro .wasm (wasm-bindgen --target nodejs) and writes the pages into <out dir>
// (see tests/test_libro_wasm.py). usage: node libro_wasm_node.js <pkg dir> <src dir> <out dir> <importati.txt>
const fs = require("fs");
const path = require("path");
const [pkg, src, out, listed] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_libro_wasm.js"));
const imported = new Set(fs.readFileSync(listed, "utf8").split("\n").map(l => l.split("#")[0].trim()).filter(Boolean));
console.log(w.version());
for (const f of fs.readdirSync(src).filter(n => n.endsWith(".md")).sort()) {
  fs.writeFileSync(path.join(out, f.replace(/\.md$/, ".html")), w.convert_chapter(fs.readFileSync(src + "/" + f, "utf8"), f, imported.has(f)));
}
try { w.convert_chapter("no heading here\n", "x.md", false); } catch (e) { console.log("error " + e); }
// the preview marks the editable blocks; an edit goes back into the source or is refused with a message
const md = "# T\n\nUno due.\n";
console.log("marked " + /<p data-src="3-3">Uno due\.<\/p>/.test(w.preview_chapter(md, "01-x.md", false)));
console.log("edited " + JSON.stringify(w.edit_preview_block(md, "01-x.md", false, "3-3", "Uno <b>tre</b>.")));
try { w.edit_preview_block(md, "01-x.md", false, "3-3", "a *b* c"); } catch (e) { console.log("refused " + e); }
// retitled sections: the headings check, and the search index rebuilt for one chapter
console.log("retitled " + w.headings_retitled("# T\n\n## A {#a}\n", "# T\n\n## B {#a}\n", "01-x.md"));
try { w.headings_retitled("# T\n\n## A {#a}\n", "# T\n", "01-x.md"); } catch (e) { console.log("refusedh " + e); }
const idx = "// h\n// [..]\nconst INDICE_RICERCA = [\n[\"T\", \"capitolo\", \"01-x.html\", \"Cap. 1\", \"t\"],\n[\"A\", \"sezione\", \"01-x.html#a\", \"Cap. 1\", \"a\"]\n];\n";
console.log("reindexed " + JSON.stringify(w.reindex_chapter(idx, "01-x.html", '<h2 id="a">B &amp; c</h2>')));
