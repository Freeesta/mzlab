// @ts-check
// Brings the Markdown written by the visual editor back to the dialect the converter and Claude read (tools/libro/LEGGIMI.md):
// the editor escapes [ and ~, writes "*" bullets and pads tables; none of that is needed, so the file stays clean and small diffs stay small.

/** @param {string} md */
export function normalize(md) {
  let fence = false;
  const out = md.split("\n").map((line) => {
    if (/^\s*(```|~~~)/.test(line)) fence = !fence;
    if (fence || /^\s*(```|~~~)/.test(line)) return line;
    // keep code spans untouched, clean the rest of the line
    return line
      .split(/(`[^`]*`)/)
      .map((part, i) => (i % 2 ? part : part.replace(/\\(\[|~)/g, "$1")))
      .join("")
      .replace(/^(\s*)\* /, "$1- ");
  });
  return out.join("\n").replace(/\n{3,}/g, "\n\n").replace(/\s*$/, "\n");
}
