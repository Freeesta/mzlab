"""Static check against the Safari floor of the site (16.4, docs/agenti/compat-safari.md): greps the interface sources (mzlab/web, flow, TP_Mine;
classic scripts, modules, workers, sw.js, CSS) for web APIs, JS syntax and CSS that Safari 16.4 lacks. Each use must be listed, with its count and
the guard or fallback that covers it, in tools/safari_api_ammesse.txt; a new or unlisted use fails. Run: python3 tools/safari_api.py [--elenco].
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AMMESSE = ROOT / "tools" / "safari_api_ammesse.txt"
DIRS = ("mzlab/web", "flow", "TP_Mine")
SKIP = ("mzlab/web/vendor/", "mzlab/web/esempi/", "mzlab/web/lang/", "mzlab/web/elements.js", "/tests/", "TP_Mine/data/")
EXT = {".js", ".mjs", ".css", ".html"}

# id, first Safari with it ("-" = none yet), what it is, regex. JS ones skip comment lines; CSS ones look at CSS files and CSS kept in JS strings.
JS = [
    ("AbortSignal.any", "17.4", "AbortSignal.any", r"\bAbortSignal\.any\b"),
    ("Object.groupBy", "17.4", "Object/Map.groupBy", r"\b(?:Object|Map)\.groupBy\b"),
    ("Promise.withResolvers", "17.4", "Promise.withResolvers", r"\bPromise\.withResolvers\b"),
    ("Promise.try", "18.2", "Promise.try", r"\bPromise\.try\("),
    ("Set methods", "17", "Set union/intersection/...", r"\.(?:union|intersection|difference|symmetricDifference|isSubsetOf|isSupersetOf|isDisjointFrom)\("),
    ("Iterator helpers", "18.4", "Iterator.from / iterator map,filter,take", r"\bIterator\.from\b|\.(?:values|keys|entries)\(\)\.(?:map|filter|take|drop|flatMap|reduce|toArray|some|every|find)\("),
    ("ArrayBuffer.transfer", "17.4", "ArrayBuffer transfer", r"\.transfer(?:ToFixedLength)?\(\s*\)"),
    ("Float16Array", "18.2", "Float16Array", r"\bFloat16Array\b"),
    ("RegExp.escape", "18.2", "RegExp.escape", r"\bRegExp\.escape\b"),
    ("regex v flag", "17", "regular expression /v", r"(?:^|[=(,:!&|?;{}\[]|\breturn)\s*/(?![/*])(?:\\.|\[(?:\\.|[^\]\\])*\]|[^/\\\n\[])+/[dgimsuy]*v[dgimsuy]*(?![\w$])|\bnew RegExp\([^)]*,\s*[\"'`][a-z]*v[a-z]*[\"'`]\s*\)"),
    ("import attributes", "17.2", "import ... with { type }", r"\bimport\b[^;\n]*\bwith\s*\{\s*type\s*:"),
    ("using", "-", "explicit resource management", r"^\s*(?:await\s+)?using\s+[A-Za-z_$][\w$]*\s*="),
    ("URL.canParse", "17", "URL.canParse / URL.parse", r"\bURL\.(?:canParse|parse)\("),
    ("Blob.bytes", "18", "Blob/Response .bytes()", r"\b(?:[Bb]lob|res|response|resp|file)\.bytes\(\s*\)"),
    ("checkVisibility", "17.4", "Element.checkVisibility", r"\.checkVisibility\("),
    ("requestIdleCallback", "-", "requestIdleCallback", r"\brequestIdleCallback\b"),
    ("scheduler", "-", "scheduler.postTask / yield", r"\bscheduler\.(?:postTask|yield)\b"),
    ("userAgentData", "-", "navigator.userAgentData", r"\buserAgentData\b"),
    ("file pickers", "-", "showOpenFilePicker / showSaveFilePicker / showDirectoryPicker", r"\bshow(?:Open|Save)FilePicker\b|\bshowDirectoryPicker\b"),
    ("createWritable", "26", "FileSystemFileHandle.createWritable", r"\.createWritable\("),
    ("FileSystemObserver", "-", "FileSystemObserver", r"\bFileSystemObserver\b"),
    ("getAsFileSystemHandle", "-", "DataTransferItem.getAsFileSystemHandle", r"\bgetAsFileSystemHandle\b"),
    ("handle.remove", "-", "FileSystemHandle.remove()", r"\b(?:fh|dh|h|handle|file|dir)\.remove\(\s*(?:\{[^}]*recursive[^}]*\})?\s*\)"),
    ("performance.memory", "-", "performance.memory / measureUserAgentSpecificMemory", r"\bperformance\.(?:memory|measureUserAgentSpecificMemory)\b"),
    ("deviceMemory", "-", "navigator.deviceMemory", r"\bdeviceMemory\b"),
    ("ReadableStream async iteration", "-", "ReadableStream.from / for await over a stream / .values() of a stream", r"\bReadableStream\.from\b|for\s+await\s*\([^)]*\bof\s+[\w.]*(?:body|stream|readable)\b"),
    ("scrollend", "-", "scrollend event", r"[\"']scrollend[\"']|\bonscrollend\b"),
    ("popover", "17", "popover API", r"\.(?:showPopover|hidePopover|togglePopover)\(|\bpopover=|setAttribute\(\s*[\"']popover[\"']"),
    ("closedby", "-", "dialog closedby / requestClose", r"\bclosedby\b|\.requestClose\("),
    ("navigation API", "26", "window.navigation", r"\bnavigation\.(?:navigate|addEventListener|currentEntry|entries)\b"),
    ("canvas filter", "18", "CanvasRenderingContext2D.filter", r"\b(?:ctx|g|c|x|cx|gc|context)\.filter\s*=(?!=)"),
    ("canvas letterSpacing", "18.4", "CanvasRenderingContext2D.letterSpacing / wordSpacing", r"\.(?:letterSpacing|wordSpacing)\s*=(?!=)"),
    ("device-pixel-content-box", "-", "ResizeObserver device-pixel-content-box", r"device-pixel-content-box|devicePixelContentBoxSize"),
    ("getCoalescedEvents", "18.2", "PointerEvent.getCoalescedEvents", r"\bgetCoalescedEvents\b"),
    ("pointerrawupdate", "-", "pointerrawupdate", r"\bpointerrawupdate\b"),
    ("startViewTransition", "18", "document.startViewTransition", r"\bstartViewTransition\b"),
    ("Error.captureStackTrace", "-", "Error.captureStackTrace (V8)", r"\bError\.captureStackTrace\b"),
    ("WebGPU", "26", "navigator.gpu", r"\bnavigator\.gpu\b"),
    ("EyeDropper", "-", "EyeDropper", r"\bEyeDropper\b"),
    ("OffscreenCanvas webgl", "17", "OffscreenCanvas with webgl", r"OffscreenCanvas[^;\n]*getContext\(\s*[\"']webgl"),
    ("wasm exceptions/JSPI", "-", "WebAssembly.Tag / Suspending / promising", r"\bWebAssembly\.(?:Tag|Exception|Suspending|promising)\b"),
    ("wheel ctrlKey zoom", "-", "Ctrl+wheel zoom: a trackpad pinch on Safari is a gesture* event (compat.js turns it into this wheel)", r"[\"']wheel[\"'][^\n]*\bctrlKey\b"),
    ("ClipboardItem", "13.1", "ClipboardItem after an await: Safari wants the item made in the click (a Promise of the Blob)", r"\bnew\s+ClipboardItem\("),
]
CSS = [
    ("CSS nesting", "16.5", "CSS nesting (&)", r"(?:^|[{};])\s*&(?:[\s:.\[>+~]|#(?!\d|x[0-9a-fA-F]))"),
    ("text-wrap", "17.5", "text-wrap: balance / pretty", r"\btext-wrap(?:-style)?\s*:"),
    ("light-dark", "17.5", "light-dark()", r"\blight-dark\("),
    ("@starting-style", "17.5", "@starting-style", r"@starting-style"),
    ("@scope", "17.4", "@scope", r"@scope\b"),
    ("field-sizing", "-", "field-sizing", r"\bfield-sizing\s*:"),
    ("anchor positioning", "26", "anchor positioning", r"\banchor-name\s*:|\bposition-anchor\s*:|\banchor\(|position-try"),
    ("container style queries", "18", "@container style()", r"@container[^{]*\bstyle\("),
    ("content-visibility", "18", "content-visibility", r"\bcontent-visibility\s*:"),
    ("scrollbar props", "18.2", "scrollbar-width / scrollbar-gutter / scrollbar-color", r"\bscrollbar-(?:width|gutter|color)\s*:"),
    ("transition-behavior", "17.4", "transition-behavior / allow-discrete", r"\btransition-behavior\s*:|allow-discrete"),
    ("interpolate-size", "-", "interpolate-size / calc-size()", r"\binterpolate-size\s*:|\bcalc-size\("),
    (":user-invalid", "16.5", ":user-invalid / :user-valid", r":user-(?:in)?valid\b"),
    (":popover-open", "17", ":popover-open", r":popover-open"),
    ("view-transition", "18", "view-transition-name / ::view-transition", r"view-transition"),
    ("backdrop-filter", "18", "backdrop-filter without -webkit-", r"(?<![-\w])backdrop-filter\s*:"),
    ("text-box", "18.2", "text-box-trim / text-box", r"\btext-box(?:-trim|-edge)?\s*:"),
    ("contain-intrinsic-size", "17", "contain-intrinsic-size", r"\bcontain-intrinsic-(?:size|width|height)\s*:"),
    ("font-size-adjust", "17", "font-size-adjust", r"\bfont-size-adjust\s*:"),
    ("prefers-reduced-transparency", "-", "prefers-reduced-transparency", r"prefers-reduced-transparency"),
]
COMMENT = re.compile(r"^\s*(?://|/?\*)")


def files() -> list[Path]:
    out = []
    for d in DIRS:
        for p in sorted((ROOT / d).rglob("*")):
            rel = p.relative_to(ROOT).as_posix()
            if p.suffix in EXT and p.is_file() and not any(s in rel or rel.startswith(s) for s in SKIP):
                out.append(p)
    return out


def css_parts(p: Path, text: str) -> list[tuple[int, str]]:
    """The CSS of a file with line numbers: the whole of a .css, the <style> of an .html, the CSS-looking string literals of a script."""
    lines = text.splitlines()
    if p.suffix == ".css":
        return list(enumerate(lines, 1))
    looks = re.compile(r"[{;]\s*[a-z-]+\s*:[^;{}]+[;}]|@(?:media|supports|container|keyframes|font-face|layer)\b")
    return [(i, l) for i, l in enumerate(lines, 1) if looks.search(l) and not COMMENT.match(l)]


def scan() -> dict[tuple[str, str], list[int]]:
    hits: dict[tuple[str, str], list[int]] = {}
    for p in files():
        rel = p.relative_to(ROOT).as_posix()
        text = p.read_text(encoding="utf-8", errors="replace")
        if p.suffix != ".css":
            for i, line in enumerate(text.splitlines(), 1):
                if COMMENT.match(line):
                    continue
                for sid, _v, _d, rx in JS:
                    if re.search(rx, line):
                        hits.setdefault((rel, sid), []).append(i)
        for i, line in css_parts(p, text):
            for sid, _v, _d, rx in CSS:
                if re.search(rx, line):
                    hits.setdefault((rel, sid), []).append(i)
    return hits


def ammesse() -> dict[tuple[str, str], int]:
    """file::id::count  # the guard or fallback (one line per file and id)."""
    out: dict[tuple[str, str], int] = {}
    for raw in AMMESSE.read_text(encoding="utf-8").splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        f, sid, n = line.split("::")
        out[(f.strip(), sid.strip())] = int(n)
    return out


def controlla() -> list[str]:
    hits, ok = scan(), ammesse()
    ver = {s[0]: s[1] for s in JS + CSS}
    bad = []
    for (f, sid), lines in sorted(hits.items()):
        want = ok.get((f, sid))
        if want is None:
            bad.append(f"{f}:{lines[0]}: {sid} (Safari {ver[sid]}) not covered: add a guard or a fallback in compat.js, then list it in tools/safari_api_ammesse.txt")
        elif len(lines) > want:
            bad.append(f"{f}:{','.join(map(str, lines))}: {sid} used {len(lines)} times, {want} listed in tools/safari_api_ammesse.txt")
    for (f, sid), n in sorted(ok.items()):
        if len(hits.get((f, sid), [])) < n:
            bad.append(f"tools/safari_api_ammesse.txt: {f}::{sid}::{n} lists more uses than there are ({len(hits.get((f, sid), []))}): lower or remove the line")
    return bad


def main() -> int:
    if "--elenco" in sys.argv:
        for (f, sid), lines in sorted(scan().items()):
            print(f"{f}::{sid}::{len(lines)}  # lines {','.join(map(str, lines))}")
        return 0
    bad = controlla()
    for b in bad:
        print(b)
    print(f"safari_api: {len(bad)} problems" if bad else "safari_api: ok")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
