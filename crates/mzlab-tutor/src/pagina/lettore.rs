//! The state machine of `pagina`: the `Reader` methods that open, close and flush the elements of a page.
use super::*;

impl Reader {
    pub(super) fn new(file: &str) -> Reader {
        Reader {
            pg: Page {
                file: file.trim_end_matches(".html").to_string(),
                ..Page::default()
            },
            stack: Vec::new(),
            skip: None,
            cur: String::new(),
            buf: String::new(),
            sink: Sink::Body,
            sink_depth: 0,
            question: String::new(),
            hidden: String::new(),
            in_exercise: None,
            in_figdesc: None,
            caption_last: false,
            label_open: false,
            term_anchor: String::new(),
            term: String::new(),
            in_dd: false,
            kicker: String::new(),
        }
    }

    /// Closes the paragraph being collected; `caption` marks it as a figure caption, `fig` as a figure.
    fn flush(&mut self, fig: bool, caption: bool) {
        let t = squash(&self.cur);
        self.cur.clear();
        if !t.is_empty() {
            self.pg.paras.push((t, fig));
            self.caption_last = caption;
        }
    }

    fn plain_break(&mut self) {
        self.flush(false, false);
    }

    pub(super) fn text(&mut self, t: &str) {
        if self.skip.is_some() {
            return;
        }
        let target = match self.sink {
            Sink::Body => &mut self.cur,
            Sink::Kicker | Sink::Heading | Sink::Title | Sink::Term => &mut self.buf,
            Sink::Question => &mut self.question,
            Sink::Hidden => &mut self.hidden,
        };
        decode(t, target);
    }
}

impl Reader {
    pub(super) fn open(&mut self, name: String, attrs: &str) {
        let classes = attr(attrs, "class").unwrap_or_default();
        if self.skip.is_some() || (skipped(&name, &classes) && !is_void(&name)) {
            if self.skip.is_none() {
                self.skip = Some(self.stack.len());
            }
            if !is_void(&name) {
                self.stack.push(El {
                    name,
                    classes,
                    id: None,
                });
            }
            return;
        }
        if is_void(&name) {
            if name == "br" && self.sink == Sink::Body {
                self.cur.push(' ');
            }
            return;
        }
        let id = attr(attrs, "id");
        let keep_label = std::mem::take(&mut self.label_open);
        self.structure(&name, &classes, id.clone(), keep_label);
        self.inline_marks(&name, &classes);
        self.stack.push(El { name, classes, id });
    }

    fn set_sink(&mut self, sink: Sink, depth: usize) {
        self.sink = sink;
        self.sink_depth = depth;
        if !matches!(sink, Sink::Hidden | Sink::Question) {
            self.buf.clear();
        }
    }

    /// Cell separators, exponents and fractions inside the running text.
    fn inline_marks(&mut self, name: &str, classes: &str) {
        if self.sink != Sink::Body {
            return;
        }
        if (name == "td" || name == "th") && !self.cur.is_empty() {
            self.cur.push_str(" | ");
        }
        if name == "sup" && self.cur.ends_with(|c: char| c.is_ascii_digit()) {
            self.cur.push('^');
        }
        if name == "span" && has_class(classes, "frac") {
            self.cur.push(' ');
        }
    }

    pub(super) fn finish(mut self) -> Page {
        self.plain_break();
        self.pg.emit_section();
        self.pg.chapter = if self.kicker.is_empty() {
            self.pg.title.clone()
        } else {
            self.kicker
        };
        self.pg
    }
}

impl Reader {
    /// What an opening tag does to the sections, exercises, figures and glossary being collected.
    fn structure(&mut self, name: &str, classes: &str, id: Option<String>, keep_label: bool) {
        let depth = self.stack.len();
        let has = |c: &str| has_class(classes, c);
        let is_exercise = name == "details" && (has("q") || has("poe"));
        let is_figdesc = name == "details" && has("figdesc");
        if is_exercise && self.in_exercise.is_none() {
            self.plain_break();
            self.pg.emit_section();
            self.in_exercise = Some(depth);
            self.question.clear();
            self.hidden.clear();
            self.set_sink(Sink::Hidden, depth);
        } else if is_figdesc && self.in_figdesc.is_none() {
            // the description joins the figure: no break before it, its summary is not text
            self.in_figdesc = Some(depth);
            match self.pg.paras.pop().filter(|_| self.caption_last) {
                Some(last) => {
                    self.cur = last.0;
                    self.cur.push(' ');
                }
                None => self.plain_break(),
            }
        } else if let Some(ex) = self.in_exercise {
            if name == "summary" && depth == ex + 1 {
                self.set_sink(Sink::Question, depth);
            } else if breaks(name) && self.sink == Sink::Hidden {
                self.hidden.push(' ');
            }
        } else if name == "summary" && self.in_figdesc.is_some() {
            self.skip = Some(depth);
        } else if name == "h1" {
            self.plain_break();
            self.set_sink(Sink::Title, depth);
        } else if name == "h2" || name == "h3" {
            self.plain_break();
            self.pg.emit_section();
            self.set_sink(Sink::Heading, depth);
        } else if name == "div" && has("kicker") {
            self.set_sink(Sink::Kicker, depth);
        } else if name == "dt" {
            self.plain_break();
            self.pg.emit_section();
            self.set_sink(Sink::Term, depth);
            self.term_anchor = id.unwrap_or_default();
        } else if name == "dd" && !self.term_anchor.is_empty() {
            self.in_dd = true;
            self.cur.clear();
        } else if self.in_figdesc.is_some() {
            // paragraphs inside the description stay in one
            if matches!(name, "p" | "li") && !self.cur.is_empty() {
                self.cur.push(' ');
            }
        } else if breaks(name) && !(name == "p" && keep_label) && !self.in_dd {
            self.plain_break();
        }
    }
}

impl Reader {
    pub(super) fn close(&mut self, name: &str) {
        let Some(pos) = self.stack.iter().rposition(|e| e.name == name) else {
            return;
        };
        let el = self.stack.remove(pos);
        self.stack.truncate(pos);
        if let Some(d) = self.skip {
            if pos <= d {
                self.skip = None;
            }
            return;
        }
        self.end_sink(pos, name, &el);
        if self.end_special(pos, name) {
            return;
        }
        let in_box = self
            .stack
            .last()
            .is_some_and(|p| p.name == "div" && has_class(&p.classes, "box"));
        if name == "b" && in_box {
            self.cur.push_str(". ");
            self.label_open = true;
        } else if name == "figcaption" {
            self.flush(false, true);
        } else if breaks(name)
            && self.in_figdesc.is_none()
            && !self.in_dd
            && self.sink == Sink::Body
        {
            self.plain_break();
        }
    }

    /// The element that took the text of a heading, term, kicker or question closes: keep what it collected.
    fn end_sink(&mut self, pos: usize, name: &str, el: &El) {
        if self.sink == Sink::Body
            || pos != self.sink_depth
            || (self.sink == Sink::Hidden && name == "summary")
        {
            return;
        }
        match self.sink {
            Sink::Kicker => self.kicker = squash(&self.buf),
            Sink::Title => {
                self.pg.title = squash(&self.buf);
                self.pg.sect_title = self.pg.title.clone();
            }
            Sink::Heading => {
                self.pg.sect_title = squash(&self.buf);
                self.pg.sect_anchor = el.id.clone().unwrap_or_else(|| slug(&self.pg.sect_title));
            }
            Sink::Term => self.term = squash(&self.buf),
            Sink::Body | Sink::Question | Sink::Hidden => {}
        }
        if self.sink == Sink::Question {
            self.sink = Sink::Hidden;
            self.sink_depth = self.in_exercise.unwrap_or(0);
        } else if self.sink != Sink::Hidden {
            self.sink = Sink::Body;
        }
        self.buf.clear();
    }
}

impl Reader {
    /// Closing of an exercise, a figure description or a glossary definition: they emit their own blocks. True when the tag is fully handled.
    fn end_special(&mut self, pos: usize, name: &str) -> bool {
        if self.in_exercise == Some(pos) {
            let q = squash(&self.question);
            if !q.is_empty() {
                let h = squash(&self.hidden);
                let (anchor, title) = (self.pg.sect_anchor.clone(), self.pg.sect_title.clone());
                self.pg.push(&anchor, &title, "esercizio", q, Some(h));
            }
            self.in_exercise = None;
            self.sink = Sink::Body;
            return true;
        }
        if self.in_figdesc == Some(pos) {
            self.in_figdesc = None;
            // the figure's text is closed here so the next paragraph does not join it
            self.flush(true, false);
            return true;
        }
        if name == "dd" && self.in_dd {
            self.in_dd = false;
            let def = squash(&self.cur);
            self.cur.clear();
            let text = format!("{}: {def}", self.term);
            for piece in cut(&text) {
                let (anchor, term) = (self.term_anchor.clone(), self.term.clone());
                self.pg.push(&anchor, &term, "glossario", piece, None);
            }
            self.term_anchor.clear();
            return true;
        }
        false
    }
}
