"""Checks the two interface languages (Italian and English) of mzLab. Part of `python3 tools/verifica.py --rapida`.

    python3 tools/controlla_i18n.py            # checks; exit code 1 if something is wrong
    python3 tools/controlla_i18n.py --elenco   # also lists, file by file, Italian words still outside the catalogs

What is checked (specification: PROMPT_INGLESE.md of the data repository, section 4):
 1. every key used (`t("..")`, `data-i18n*="..")`, keys of Python messages) exists in lang/it.js and lang/en.js; no orphan key;
 2. placeholders and ICU structure are the same in the two languages; ICU braces are balanced;
 3. the Italian text written in the static HTML (elements with data-i18n*) equals lang/it.js;
 4. no Italian outside the catalogs: in the string literals of the files already migrated (MIGRATED) no frequent Italian word;
    the other files are only listed with --elenco (they become mandatory one by one as parts B-E migrate them);
    motivated exceptions are in tools/i18n_eccezioni.txt;
 5. the glossary (lang/glossario.json): if the Italian text contains an Italian form of an entry, the English text must contain an
    English form and none of the forms to avoid;
 6. lang/contesto.json describes every key that has parameters.
The catalogs are plain JSON inside `I18N.add("xx", { ... });`: the first and the last line are cut off and the rest is parsed as JSON.
"""
from __future__ import annotations

import html
import json
import re
import sys
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"
LANG = WEB / "lang"
ECCEZIONI = ROOT / "tools" / "i18n_eccezioni.txt"

# JS files whose strings must already be free of Italian (grows as the parts of the work migrate files)
MIGRATED = {"mzlab/web/elementare.js", "mzlab/web/visualizza.js", "mzlab/web/barra.js", "mzlab/web/paletta.js", "mzlab/web/browser-worker.js", "mzlab/web/browser.js", "mzlab/web/calcola.js", "mzlab/web/draw.js", "mzlab/web/mappa3d.js", "mzlab/web/explore.js", "mzlab/web/ordine.js", "mzlab/web/conversione.js", "mzlab/web/pannelli.js", "mzlab/web/grafici.js", "mzlab/web/picchi.js", "mzlab/web/eventi.js", "mzlab/web/i18n.js", "mzlab/web/icons-modi.js", "mzlab/web/index.html", "mzlab/web/modi.js", "mzlab/web/perdite.js", "mzlab/web/perf.js", "mzlab/web/scroll.js", "mzlab/web/settings.js", "mzlab/web/spettro.js", "mzlab/web/tables.js", "mzlab/web/help.js", "mzlab/web/liste.js", "mzlab/web/libreria.js", "mzlab/web/livelli.js", "mzlab/web/dda.js", "mzlab/web/composizione.js", "mzlab/web/origine.js", "mzlab/web/banco.js", "mzlab/web/libreria-worker.js", "mzlab/web/hr.js", "mzlab/web/tabs.js", "mzlab/web/touch.js", "mzlab/web/xlsx.js"}
# not checked for Italian words: third-party code, the Teoria (Italian only, by decision), TP Mine, the catalogs themselves
SKIP_DIRS = ("mzlab/web/vendor/", "mzlab/web/teoria/", "mzlab/web/lang/", "mzlab/web/esempi/", "TP_Mine/", "target/", "pkg/", "crates/")
SKIP_FILES = {"mzlab/web/elements.js"}

# frequent Italian words (those that are also English words are left out: a, e, per, in, ...)
ITALIAN_WORDS = set("""il lo la le gli dei del della delle degli dello nel nella nelle nei negli sul sulla sulle sui con una uno
che non sono è più può puoi hai hanno ogni tutti tutto tutte nessun nessuna nessuno questo questa questi queste quello quella
carica caricare scegli scegliere apri aprire chiudi chiudere annulla salva salvare scarica scaricare elimina cancella
mostra nascondi seleziona trascina rilascia aggiungi rimuovi copia incolla nuovo nuova nuovi vecchio file-non
dati spettro spettri picco picchi errore errori attendi attendere caricamento prova esempio esempi aiuto
perché quando dove come anche ancora già ora poi solo senza dopo prima fra tra dalla dallo dalle dagli sia siamo
cromatogramma cromatogrammi traccia tracce tempi picco segnale intensità asse assi fondo scheda schede pannello pannelli grafico grafici
campione campioni bianco bianchi integrazione integrare aree tabella tabelle riga righe colonna colonne valore valori nome nomi tipo tipi
esperimento esperimenti precursore precursori ione ioni frammenti frammento perdita perdite neutra neutre massa masse formule addotto addotti
libreria librerie contaminante contaminanti risoluzione bassa alta ingrandisce ingrandire ingrandito riduci allarga vista intera barra
laterale voce voci clic destro sinistro doppio rotella tasto tasti premi premere selezionato selezionati selezione scelta attivo attiva attivi
acceso spento spenta accesa vuoto vuota mancano manca serve servono puoi possono vedere vedi guarda leggi scrivi scrivere scritto copiato
salvato esporta esportare immagine sovrapposti impilati cascata sottrai sottrazione sottratto livello retta curva mediana massimo minimo
intervallo larghezza altezza lunghezza unità secondi minuti ore giorni avviso attenzione alcuni alcune almeno oltre sopra sotto accanto
dentro fuori quale quali cui mentre però quindi invece infatti ormai sempre mai meno molto poco troppo bene male durante fino verso circa
tramite nell nella sull sugli dall dell nello sullo dagli negli quel quella quelli numero numeri picchi cosa cose modo modi
nessuno scheda chiaro scuro rilevamento rilevato rilevati tolto tolti toglie togliere aggiunge aggiungere aggiunto trovato trova trovare
mostrare mostrato nascosto nascondere cambia cambiare cambiato cerca cercare ricerca risultato risultati fonte fonti didascalia titolo
etichetta etichette decimali decimale lingua testo colori colore tema impostazioni strumento strumenti metodo metodi acquisizione
sovrapporre disegna disegnare disegnato disegni struttura strutture molecola molecole carica cariche positivo negativo positiva negativa
vuol dire significa significato esempio esempi spiegazione guida aiuto domanda domande risposta risposte
""".split())

PARAM_RE = re.compile(r"\{([A-Za-z_][\w]*)")


# ---------------------------------------------------------------- catalogs
def load_catalog(lang: str) -> dict[str, str]:
    p = LANG / f"{lang}.js"
    lines = p.read_text(encoding="utf-8").strip().splitlines()
    if not lines[0].startswith(f'I18N.add("{lang}", {{') or lines[-1].strip() != "});":
        raise ValueError(f"{p.name}: the first line must be `I18N.add(\"{lang}\", {{` and the last `}});`")
    return json.loads("{" + "\n".join(lines[1:-1]) + "}")


def icu_signature(msg: str) -> tuple[list, list]:
    """(sorted (name, type) of the arguments, sorted plural categories) of a message; raises on unbalanced braces."""
    depth = 0
    for ch in msg:
        depth += (ch == "{") - (ch == "}")
        if depth < 0:
            raise ValueError("unbalanced }")
    if depth:
        raise ValueError("unbalanced {")
    args, cats = [], []

    def walk(s: str) -> None:
        i = 0
        while i < len(s):
            if s[i] != "{":
                i += 1
                continue
            d, j = 0, i
            while j < len(s):
                d += (s[j] == "{") - (s[j] == "}")
                if d == 0:
                    break
                j += 1
            inner = s[i + 1:j]
            parts = [x.strip() for x in inner.split(",", 2)]
            kind = parts[1] if len(parts) > 1 else "plain"
            if re.fullmatch(r"[A-Za-z_]\w*", parts[0]):
                args.append((parts[0], kind))
            if kind == "plural" and len(parts) > 2:
                rest = parts[2]
                for m in re.finditer(r"(=\d+|zero|one|two|few|many|other)\s*\{", rest):
                    cats.append(m.group(1))
                for m in re.finditer(r"\{((?:[^{}]|\{[^{}]*\})*)\}", rest):
                    walk(m.group(1))
            i = j + 1

    walk(msg)
    return sorted(set(args)), sorted(set(cats))


# ---------------------------------------------------------------- usage in the sources
def source_files(exts: tuple[str, ...]) -> list[Path]:
    out = []
    for p in sorted(list((ROOT / "mzlab").rglob("*")) + list((ROOT / "tools").glob("*"))):
        if not p.is_file() or p.suffix not in exts:
            continue
        rel = p.relative_to(ROOT).as_posix()
        if rel.startswith(SKIP_DIRS) or rel in SKIP_FILES or "__pycache__" in rel or rel.startswith("mzlab/web/lang/"):
            continue
        out.append(p)
    return out


T_CALL = re.compile(r"""(?<![\w$.])(?:I18N\.)?t\(\s*(["'`])([A-Za-z][\w]*(?:\.[\w]+)+)\1""")
T_DYN = re.compile(r"""(?<![\w$.])(?:I18N\.)?t\(\s*`([A-Za-z][\w]*(?:\.[\w]+)*\.)\$\{""")     # t(`prefix.${x}`): keeps every key with that prefix
DATA_ATTR = re.compile(r"""data-i18n(?:-[a-z-]+)?=["']([A-Za-z][\w]*(?:\.[\w]+)+)["']""")
PY_KEY = re.compile(r"""(?:UserError\(\s*|["'](?:error_key|label_key)["']\s*:\s*|["']key["']\s*:\s*|message\(\s*)["']([a-z][\w]*(?:\.[\w]+)+)["']""")


KEY_LITERAL = re.compile(r"""["'`]([a-z][A-Za-z0-9]*(?:\.[A-Za-z0-9]+)+)["'`]""")


def used_keys(catalog_keys: set[str] = frozenset()) -> tuple[dict[str, set[str]], set[str]]:
    keys: dict[str, set[str]] = {}
    prefixes: set[str] = set()
    for p in source_files((".js", ".html", ".py")):
        rel = p.relative_to(ROOT).as_posix()
        text = p.read_text(encoding="utf-8", errors="replace")
        found = set()
        if p.suffix == ".py":
            if rel.startswith("tools/"):
                continue
            found |= set(PY_KEY.findall(text))
        else:
            found |= {m.group(2) for m in T_CALL.finditer(text)} | set(DATA_ATTR.findall(text))
            found |= {k for k in KEY_LITERAL.findall(text) if k in catalog_keys}          # t(cond ? "a.b" : "c.d"), keys in tables
            prefixes |= set(T_DYN.findall(text))
        for k in found:
            keys.setdefault(k, set()).add(rel)
    return keys, prefixes


# ---------------------------------------------------------------- static HTML = it.js
class _Static(HTMLParser):
    """Collects (key, attribute or None, text) for each element with data-i18n*."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.stack: list[list] = []
        self.found: list[tuple[str, str | None, str]] = []

    def handle_starttag(self, tag, attrs):
        d = dict(attrs)
        for a, target in (("data-i18n-title", "title"), ("data-i18n-placeholder", "placeholder"), ("data-i18n-aria-label", "aria-label")):
            if a in d:
                self.found.append((d[a], target, d.get(target, "")))
        if tag in ("br", "img", "input", "meta", "link", "hr"):
            return
        key = d.get("data-i18n") or d.get("data-i18n-html")
        self.stack.append([tag, key, [], "data-i18n-html" in d])

    def handle_data(self, data):
        for fr in self.stack:
            if fr[1]:
                fr[2].append(data)

    def handle_endtag(self, tag):
        for i in range(len(self.stack) - 1, -1, -1):
            if self.stack[i][0] == tag:
                fr = self.stack[i]
                del self.stack[i:]
                if fr[1]:
                    self.found.append((fr[1], "html" if fr[3] else None, "".join(fr[2])))
                return


def check_static_html(it: dict, errs: list) -> None:
    for p in source_files((".html",)):
        ps = _Static()
        ps.feed(p.read_text(encoding="utf-8"))
        for key, attr, text in ps.found:
            if key not in it:
                continue
            want = it[key]
            if attr == "html":                       # data-i18n-html: compare the visible text (tags and entities removed)
                want = html.unescape(re.sub(r"<[^>]+>", "", want))
            norm = lambda x: re.sub(r"\s+", " ", x.replace("{APP}", "{app}")).strip()      # {APP} in the page is {app} in the catalog
            if norm(text) != norm(want):
                errs.append(f"HTML {p.name}: «{key}»{' (' + attr + ')' if attr else ''} shows «{text.strip()[:50]}» but it.js has «{it[key][:50]}»")


# ---------------------------------------------------------------- Italian outside the catalogs
def js_strings(src: str):
    """Yields (line, text) for every string literal outside comments. Template literals are split: the static text between the ${...} is one
    string, and the code inside each ${...} is scanned again (nested strings and templates). Tolerant: regex literals can confuse it (see the
    exceptions file)."""
    n = len(src)

    def scan(i: int, line: int, stop_at_brace: bool):
        depth = 0
        while i < n:
            c = src[i]
            if c == "\n":
                line += 1
            if src.startswith("//", i):
                j = src.find("\n", i)
                i = n if j < 0 else j
                continue
            if src.startswith("/*", i):
                j = src.find("*/", i + 2)
                j = n if j < 0 else j + 2
                line += src.count("\n", i, j)
                i = j
                continue
            if stop_at_brace:
                if c == "{":
                    depth += 1
                elif c == "}":
                    if depth == 0:
                        return i + 1, line
                    depth -= 1
            if c in "\"'":
                j = i + 1
                while j < n and src[j] != c:
                    if src[j] == "\\":
                        j += 1
                    elif src[j] == "\n":
                        break
                    j += 1
                yield line, src[i + 1:j]
                i = j + 1
                continue
            if c == "`":
                i += 1
                start, ln0 = i, line
                while i < n and src[i] != "`":
                    if src[i] == "\\":
                        i += 2
                        continue
                    if src.startswith("${", i):
                        yield ln0, src[start:i]
                        line += src.count("\n", start, i)
                        sub = scan(i + 2, line, True)
                        while True:
                            try:
                                item = next(sub)
                                yield item
                            except StopIteration as stop:
                                i, line = stop.value
                                break
                        start, ln0 = i, line
                        continue
                    i += 1
                yield ln0, src[start:i]
                line += src.count("\n", start, i)
                i += 1
                continue
            i += 1
        return i, line

    gen = scan(0, 1, False)
    while True:
        try:
            yield next(gen)
        except StopIteration:
            return


def load_exceptions() -> set[tuple[str, str]]:
    out = set()
    if ECCEZIONI.exists():
        for ln in ECCEZIONI.read_text(encoding="utf-8").splitlines():
            ln = ln.strip()
            if ln and not ln.startswith("#"):
                f, _, snippet = ln.partition("\t")
                out.add((f.strip(), snippet.strip()))
    return out


def italian_in(text: str) -> list[str]:
    words = re.findall(r"(?<![\w.\-#&])[A-Za-zÀ-ÿ]+(?!\w|-|\.\w)", text)      # not the pieces of identifiers: data-del, int.menu.del, #np-liste
    return [w for w in words if w.lower() in ITALIAN_WORDS]


def check_italian(errs: list, listing: bool) -> None:
    exc = load_exceptions()
    summary = []
    for p in source_files((".js", ".py")):
        rel = p.relative_to(ROOT).as_posix()
        if p.suffix == ".py" and not rel.startswith("mzlab/"):
            continue
        strict = rel in MIGRATED
        if not strict and not listing:
            continue
        hits = []
        if p.suffix == ".js":
            for ln, s in js_strings(p.read_text(encoding="utf-8", errors="replace")):
                w = italian_in(s)
                if w and not any(f == rel and sn in s for f, sn in exc):
                    hits.append((ln, w[0], s[:60]))
        if hits and strict:
            for ln, w, s in hits[:8]:
                errs.append(f"Italian outside the catalogs: {rel}:{ln} «{w}» in «{s}»")
        elif hits:
            summary.append((len(hits), rel))
    for p in source_files((".html",)):                     # inline <script> blocks of the pages
        rel = p.relative_to(ROOT).as_posix()
        if rel not in MIGRATED:
            continue
        for m in re.finditer(r"<script(?![^>]*\bsrc=)[^>]*>(.*?)</script>", p.read_text(encoding="utf-8"), re.S):
            base = p.read_text(encoding="utf-8")[:m.start(1)].count("\n")
            for ln, s in js_strings(m.group(1)):
                w = italian_in(s)
                if w and not any(f == rel and sn in s for f, sn in exc):
                    errs.append(f"Italian outside the catalogs: {rel}:{base + ln} «{w[0]}» in «{s[:60]}»")
    if listing:
        print("Files not yet migrated (strings with Italian words):")
        for n, rel in sorted(summary, reverse=True):
            print(f"  {n:5d}  {rel}")


# ---------------------------------------------------------------- glossary
def has_form(text: str, forms: list[str]) -> bool:
    for f in forms:
        if re.search(r"(?<![\wÀ-ÿ])" + re.escape(f) + r"(?:s|es)?(?![\wÀ-ÿ])", text, re.I):          # plural of the English forms too
            return True
    return False


def check_glossary(it: dict, en: dict, errs: list) -> None:
    g = json.loads((LANG / "glossario.json").read_text(encoding="utf-8"))
    fonti = g.get("fonti", {})
    for v in g["voci"]:
        if v.get("fonte") not in fonti:
            errs.append(f"glossary: entry «{v['id']}» without a known source")
        for k, itxt in it.items():
            for ex in v.get("escludi_it", []):               # Italian phrases where the word has another meaning ("in fondo" = at the bottom)
                itxt = itxt.replace(ex, "")
            if k not in en or not has_form(itxt, v["it"]):
                continue
            etxt = en[k]
            if not has_form(etxt, v["en"]):
                errs.append(f"glossary «{v['id']}»: {k}: Italian has «{v['it'][0]}», English must use one of {v['en']}  (en: «{etxt[:60]}»)")
            if has_form(etxt, v.get("evita_en", [])):
                errs.append(f"glossary «{v['id']}»: {k}: English uses a form to avoid {v['evita_en']}  (en: «{etxt[:60]}»)")


# ---------------------------------------------------------------- main
def run(listing: bool = False) -> list[str]:
    errs: list[str] = []
    try:
        it, en = load_catalog("it"), load_catalog("en")
    except Exception as e:  # noqa: BLE001
        return [f"catalogs: {e}"]
    for k in sorted(set(it) - set(en)):
        errs.append(f"missing in en.js: {k}")
    for k in sorted(set(en) - set(it)):
        errs.append(f"missing in it.js: {k}")
    for k in sorted(set(it) & set(en)):
        try:
            si, se = icu_signature(it[k]), icu_signature(en[k])
        except ValueError as e:
            errs.append(f"{k}: {e}")
            continue
        if si[0] != se[0]:
            errs.append(f"{k}: different placeholders (it {si[0]} / en {se[0]})")
    used, prefixes = used_keys(set(it) | set(en))
    for k, files in sorted(used.items()):
        if k not in it or k not in en:
            errs.append(f"key used but not in the catalogs: {k}  ({', '.join(sorted(files))})")
    for k in sorted(set(it) | set(en)):
        if k not in used and not any(k.startswith(p) for p in prefixes):
            errs.append(f"orphan key (nobody uses it): {k}")
    check_static_html(it, errs)
    check_italian(errs, listing)
    check_glossary(it, en, errs)
    ctx = json.loads((LANG / "contesto.json").read_text(encoding="utf-8"))
    for k in sorted(it):
        if PARAM_RE.search(it[k]) and k not in ctx:
            errs.append(f"contesto.json: missing description for the key with parameters: {k}")
    return errs


def main() -> None:
    errs = run("--elenco" in sys.argv)
    if errs:
        print(f"i18n: {len(errs)} problems")
        for e in errs[:60]:
            print("  -", e)
        sys.exit(1)
    print("i18n: OK (catalogs, keys, HTML, glossary)")


if __name__ == "__main__":
    main()
