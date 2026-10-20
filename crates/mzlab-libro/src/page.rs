//! Page assembly: the Teoria chapter (or hidden Libro draft) around the converted body, same head, CSS and nav as the chapters.
use crate::blocks::Parser;
use crate::{escape, is_w, splitlines, LibroError};

const SKELETON: &str = "<!doctype html>\n<html lang=\"it\" data-esteso><head><meta charset=\"utf-8\">\n<!--\n@HEAD@\n-->\n<meta name=\"robots\" content=\"noindex\">\n<title>@TITLE@ · Libro</title><link rel=\"stylesheet\" href=\"teoria.css\"><script src=\"tema.js\"></script></head>\n<body>\n<div class=\"wrap\"><nav id=\"side\"></nav><main>\n@BODY@\n</main></div>\n</body></html>\n";

const TEORIA: &str = "<!doctype html>\n<html lang=\"it\"@HTMLATTR@><head><meta charset=\"utf-8\">\n@HEAD@<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>@TITLE@</title><link rel=\"icon\" href=\"../favicon-32.png\"><link rel=\"stylesheet\" href=\"teoria.css\"><script src=\"tema.js\"></script></head>\n<body>\n<div class=\"wrap\"><nav id=\"side\"></nav><main>\n@BODY@\n</main></div>\n@SCRIPTS@\n</body></html>\n";

const TEMA: &str = include_str!("tema_testa.html");
const MODI: &str = include_str!("../../../tools/libro/snippets/modi.html");
const QUIZ: &str = include_str!("../../../tools/libro/snippets/formulario-quiz.html");

/// What differs between the two builds of the Python CLI, plus whether raw ```html blocks are allowed for this file.
#[derive(Clone, Copy, Default)]
pub struct Options {
    /// Drop `<!-- privato -->` blocks and refuse unresolved [I].
    pub pubblico: bool,
    /// Published Teoria chapter (own head, scripts from `<!-- script: x.js -->`) instead of a hidden Libro draft.
    pub teoria: bool,
    /// The file is listed in tools/libro/importati.txt (raw ```html blocks allowed).
    pub raw_ok: bool,
}

/// Position of a fixed inline snippet: `true` = ahead of ../appname.js, `false` = at the end of the page.
fn snippet(name: &str) -> Option<(&'static str, bool)> {
    match name {
        "modi" => Some((MODI, true)),
        "formulario-quiz" => Some((QUIZ, false)),
        _ => None,
    }
}

pub(crate) fn known_snippet(name: &str) -> bool {
    snippet(name).is_some()
}

fn snippet_text(name: &str) -> &'static str {
    snippet(name)
        .map(|s| s.0.trim_end_matches('\n'))
        .unwrap_or("")
}

/// Convert the Markdown of section `nome` (a file name such as `12-dati.md`) to a full HTML page.
pub fn convert(text: &str, nome: &str, opt: &Options) -> Result<String, LibroError> {
    let prefix: String = nome
        .split('_')
        .next()
        .unwrap_or("")
        .split('.')
        .next()
        .unwrap_or("")
        .to_string();
    let mut p = Parser::new(nome, &prefix, opt);
    let lines = splitlines(text);
    let mut body = p.blocks(&lines, 1)?;
    if opt.pubblico && p.ctx.ipotesi > 0 {
        return Err(LibroError::new(
            nome,
            1,
            format!(
                "{} unresolved [I] tag(s): not allowed in a public build",
                p.ctx.ipotesi
            ),
        ));
    }
    body = subst_refs(&body, &p, nome)?;
    if p.titoli.is_empty() {
        return Err(LibroError::new(nome, 1, "the section has no heading"));
    }
    let title = if p.page_title.is_empty() {
        p.titoli[0]
            .1
            .chars()
            .filter(|c| !matches!(c, '`' | '*' | '$'))
            .collect()
    } else {
        p.page_title.clone()
    };
    let page = if opt.teoria {
        let mut head: String = p.head.iter().map(|h| format!("<!--{h}-->\n")).collect();
        if p.esteso {
            let number: String = nome.chars().take_while(|c| c.is_ascii_digit()).collect();
            let lines: Vec<String> = p.head.iter().map(|h| h.replace("--", "-")).collect();
            head = format!(
                "<!--\nchapter: {number} {}\n{}\n-->\n<meta name=\"robots\" content=\"noindex\">\n",
                title.replace("--", "-"),
                lines.join("\n")
            );
            p.html_attrs
                .push(("data-esteso".to_string(), String::new()));
        }
        let mut srcs: Vec<&str> = Vec::new();
        if !p.no_appname {
            srcs.push("../appname.js");
        }
        srcs.extend(p.scripts_before.iter().map(String::as_str));
        srcs.push("teoria.js");
        srcs.extend(p.scripts.iter().map(String::as_str));
        let tags: String = srcs
            .iter()
            .map(|x| format!("<script src=\"{x}\"></script>"))
            .collect();
        let before: String = p
            .snippets
            .iter()
            .filter(|n| snippet(n).is_some_and(|s| s.1))
            .map(|n| format!("{}\n", snippet_text(n)))
            .collect();
        let after: String = p
            .snippets
            .iter()
            .filter(|n| snippet(n).is_some_and(|s| !s.1))
            .map(|n| format!("\n{}", snippet_text(n)))
            .collect();
        let attrs: String = p
            .html_attrs
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    format!(" {k}")
                } else {
                    format!(" {k}=\"{v}\"")
                }
            })
            .collect();
        let full = if p.full_title.is_empty() {
            format!("{} · Teoria QqQ lab", escape(&title, true))
        } else {
            p.full_title.clone()
        };
        TEORIA
            .replace("@HTMLATTR@", &attrs)
            .replace("@HEAD@", &head)
            .replace("@TITLE@", &full)
            .replace("@BODY@", &body)
            .replace("@SCRIPTS@", &format!("{before}{tags}{after}"))
    } else {
        let head: Vec<String> = p.head.iter().map(|h| h.replace("--", "-")).collect();
        SKELETON
            .replace("@HEAD@", &head.join("\n"))
            .replace("@TITLE@", &escape(&title, true))
            .replace("@BODY@", &body)
    };
    inject_tema(&page)
}

/// `{{eq:id}}` / `{{fig:id}}` placeholders to their numbered labels, in a single pass.
fn subst_refs(body: &str, p: &Parser, nome: &str) -> Result<String, LibroError> {
    let c: Vec<char> = body.chars().collect();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    while i < c.len() {
        if let Some((end, kind, id)) = placeholder(&c, i) {
            let label = if kind == "eq" {
                p.eq.get(&id)
            } else {
                p.fig.get(&id)
            };
            let Some(label) = label else {
                return Err(LibroError::new(
                    nome,
                    1,
                    format!("reference @{kind}:{id} points to nothing"),
                ));
            };
            out.push_str(&if kind == "eq" {
                format!("eq. {label}")
            } else {
                format!("Figura {label}")
            });
            i = end;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    Ok(out)
}

fn placeholder(c: &[char], i: usize) -> Option<(usize, &'static str, String)> {
    if !c[i..].starts_with(&['{', '{']) {
        return None;
    }
    for kind in ["eq", "fig"] {
        let k: Vec<char> = kind.chars().collect();
        if c[i + 2..].starts_with(&k) && c.get(i + 2 + k.len()) == Some(&':') {
            let st = i + 3 + k.len();
            let mut j = st;
            while j < c.len() && (is_w(c[j]) || c[j] == '-') {
                j += 1;
            }
            if j > st && c[j..].starts_with(&['}', '}']) {
                return Some((j + 2, kind, c[st..j].iter().collect()));
            }
        }
    }
    None
}

/// The theme snippet of tools/tema_testa.py, inserted right after `<meta charset="utf-8">` (an older block is dropped first).
fn inject_tema(page: &str) -> Result<String, LibroError> {
    const OPEN: &str = "<!--tema-->";
    const CLOSE: &str = "<!--/tema-->";
    let mut text = page.to_string();
    while let Some(a) = text.find(OPEN) {
        let Some(b) = text[a + OPEN.len()..].find(CLOSE) else {
            break;
        };
        text.replace_range(a..a + OPEN.len() + b + CLOSE.len(), "");
    }
    const META: &str = "<meta charset=\"utf-8\">";
    let Some(m) = text.find(META) else {
        return Err(LibroError::new("page", 1, "no <meta charset> in page"));
    };
    text.insert_str(m + META.len(), TEMA.trim_end_matches('\n'));
    Ok(text)
}
