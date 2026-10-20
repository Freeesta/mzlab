# Third-party code shipped with TP Finder (works offline)

- `ketcher/` : Ketcher 3.18.0 (EPAM Systems), Apache-2.0, standalone build (Indigo in WebAssembly, no server).
  Built from the npm packages ketcher-react, ketcher-core and ketcher-standalone with Vite; React 18 (MIT) is bundled.
  Source: https://github.com/epam/ketcher  License: ketcher/LICENSE.txt
- `openchemlib.js` : OpenChemLib JS 9.25.1 (Zakodium / Actelion), BSD-3-Clause. Used for formula and exact mass
  of the structures drawn in Ketcher. Source: https://github.com/cheminfo/openchemlib-js  License: openchemlib.LICENSE
- `libro/` : bundle of the Libro editor page (`mzlab/web/libro/`): Milkdown/Crepe 7.22.3 (MIT) with KaTeX 0.19 (MIT), CodeMirror and the other npm packages listed in `libro/THIRD-PARTY.txt` (all MIT, dompurify MPL-2.0 or Apache-2.0).
  Built by `tools/libro/editor/build.mjs` (esbuild); licences: `libro/LICENSE-milkdown.txt`, `libro/LICENSE-katex.txt`. Contains no text of the Libro.
