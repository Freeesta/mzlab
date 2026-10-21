//! Reads one Teoria page into sections and paragraphs for `blocchi`: a small state machine over the HTML tokens (headings, glossary, exercises, figures).
//! Private to the crate; `blocchi::build` is the way in.
use crate::blocchi::{cut, Page};
use crate::html::{attr, breaks, decode, has_class, is_void, skipped, squash, tokens, El, Tok};
use mzlab_libro::slug;

/// Where the text being read goes.
#[derive(PartialEq, Clone, Copy)]
enum Sink {
    Body,
    Kicker,
    Heading,
    Title,
    Question,
    Hidden,
    Term,
}

/// The state of one page being read: the stack of open elements, the text collected so far and where the next text goes.
struct Reader {
    pg: Page,
    stack: Vec<El>,
    // depth of the stack at which a skipped element opened
    skip: Option<usize>,
    cur: String,
    buf: String,
    sink: Sink,
    sink_depth: usize,
    question: String,
    hidden: String,
    in_exercise: Option<usize>,
    in_figdesc: Option<usize>,
    caption_last: bool,
    label_open: bool,
    term_anchor: String,
    term: String,
    in_dd: bool,
    kicker: String,
}

mod lettore;

pub(crate) fn read_page(file: &str, html: &str) -> Option<Page> {
    // the Pratica pages are exercises with live figures and `index` is the menu: neither is text to cite
    if html.contains("http-equiv=\"refresh\"")
        || file.starts_with("pratica")
        || file == "index.html"
    {
        return None;
    }
    let body = html.find("<main").map_or(html, |m| &html[m..]);
    let mut rd = Reader::new(file);
    for tok in tokens(body) {
        match tok {
            Tok::Text(t) => rd.text(t),
            Tok::Open(name, attrs) => rd.open(name, attrs),
            Tok::Close(name) => rd.close(&name),
        }
    }
    Some(rd.finish())
}
