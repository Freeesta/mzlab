//! A small backtracking regular expression for the name models of `campioni/3` (no dependency: the size of the .wasm). Supports literals, `.`,
//! classes (`[a-z]`, `[^_]`, `\d \w \s` and their negations), groups (`(...)`, `(?:...)`, `(?P<name>...)`, `(?<name>...)`), `|`, `* + ? {n} {n,} {n,m}`
//! (lazy with a trailing `?`), `^`, `$` and a leading `(?i)`. A match is always of the whole text; a step budget and a length limit stop any blow-up.

use crate::error::{Error, Result};

const MAX_STEPS: u32 = 200_000;
const MAX_TEXT: usize = 256;
const MAX_REP: u32 = 1000;
const MAX_PATTERN: usize = 512;

enum Item {
    Range(char, char),
    Digit(bool),
    Word(bool),
    Space(bool),
}

enum Node {
    Lit(char),
    Any,
    Class(Vec<Item>, bool),
    Begin,
    End,
    Group(Option<usize>, Box<Node>),
    Cat(Vec<Node>),
    Alt(Vec<Node>),
    Rep(Box<Node>, u32, u32, bool),
}

pub struct Regex {
    root: Node,
    ci: bool,
    /// The name of each capture group (None for an unnamed one), in the order of the opening parentheses.
    pub names: Vec<Option<String>>,
}

pub type Caps = Vec<Option<(usize, usize)>>;

struct Parser {
    s: Vec<char>,
    i: usize,
    names: Vec<Option<String>>,
}

fn bad(pos: usize) -> Error {
    Error::new("error.badModello").with("pos", pos)
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        let hit = self.peek() == Some(c);
        self.i += hit as usize;
        hit
    }

    fn alt(&mut self) -> Result<Node> {
        let mut v = vec![self.cat()?];
        while self.eat('|') {
            v.push(self.cat()?);
        }
        Ok(if v.len() == 1 {
            v.remove(0)
        } else {
            Node::Alt(v)
        })
    }

    fn cat(&mut self) -> Result<Node> {
        let mut v = Vec::new();
        while !matches!(self.peek(), None | Some('|') | Some(')')) {
            let a = self.atom()?;
            v.push(self.rep(a)?);
        }
        Ok(Node::Cat(v))
    }

    fn number(&mut self) -> Option<u32> {
        let st = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        let t: String = self.s[st..self.i].iter().collect();
        t.parse().ok()
    }

    fn rep(&mut self, a: Node) -> Result<Node> {
        let (lo, hi) = match self.peek() {
            Some('*') => (0, MAX_REP),
            Some('+') => (1, MAX_REP),
            Some('?') => (0, 1),
            Some('{') => {
                let save = self.i;
                self.i += 1;
                let lo = self.number();
                let hi = match (lo, self.eat(',')) {
                    (Some(l), false) => Some(l),
                    (Some(_), true) => self.number().or(Some(MAX_REP)),
                    _ => None,
                };
                match (lo, hi) {
                    (Some(l), Some(h)) if self.peek() == Some('}') && l <= h && h <= MAX_REP => {
                        (l, h)
                    }
                    (Some(_), Some(_)) if self.peek() == Some('}') => return Err(bad(save)),
                    _ => {
                        // not a quantifier: the brace is a plain character
                        self.i = save;
                        return Ok(a);
                    }
                }
            }
            _ => return Ok(a),
        };
        self.i += 1;
        let lazy = self.eat('?');
        Ok(Node::Rep(Box::new(a), lo, hi, lazy))
    }

    fn escape(&mut self) -> Result<Item> {
        let c = self.peek().ok_or_else(|| bad(self.i))?;
        self.i += 1;
        Ok(match c {
            'd' => Item::Digit(false),
            'D' => Item::Digit(true),
            'w' => Item::Word(false),
            'W' => Item::Word(true),
            's' => Item::Space(false),
            'S' => Item::Space(true),
            'n' => Item::Range('\n', '\n'),
            't' => Item::Range('\t', '\t'),
            c => Item::Range(c, c),
        })
    }

    /// One character of a class, or None after pushing a class escape (`\d`) into `items`.
    fn class_char(&mut self, c: char, items: &mut Vec<Item>) -> Result<Option<char>> {
        if c != '\\' {
            return Ok(Some(c));
        }
        match self.escape()? {
            Item::Range(a, _) => Ok(Some(a)),
            it => {
                items.push(it);
                Ok(None)
            }
        }
    }

    fn class(&mut self) -> Result<Node> {
        let neg = self.eat('^');
        let mut items = Vec::new();
        let mut first = true;
        loop {
            let c = self.peek().ok_or_else(|| bad(self.i))?;
            self.i += 1;
            if c == ']' && !first {
                break;
            }
            first = false;
            let Some(lo) = self.class_char(c, &mut items)? else {
                continue;
            };
            if self.peek() == Some('-') && self.s.get(self.i + 1).is_some_and(|&n| n != ']') {
                self.i += 1;
                let h = self.peek().ok_or_else(|| bad(self.i))?;
                self.i += 1;
                let hi = self.class_char(h, &mut items)?.ok_or_else(|| bad(self.i))?;
                if hi < lo {
                    return Err(bad(self.i));
                }
                items.push(Item::Range(lo, hi));
            } else {
                items.push(Item::Range(lo, lo));
            }
        }
        Ok(Node::Class(items, neg))
    }

    fn group(&mut self) -> Result<Node> {
        let mut cap = None;
        if self.eat('?') {
            if !self.eat(':') {
                self.eat('P');
                if !self.eat('<') {
                    return Err(bad(self.i));
                }
                let st = self.i;
                while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    self.i += 1;
                }
                let name: String = self.s[st..self.i].iter().collect();
                if name.is_empty() || !self.eat('>') || self.names.contains(&Some(name.clone())) {
                    return Err(bad(st));
                }
                cap = Some(self.names.len());
                self.names.push(Some(name));
            }
        } else {
            cap = Some(self.names.len());
            self.names.push(None);
        }
        let inner = self.alt()?;
        if !self.eat(')') {
            return Err(bad(self.i));
        }
        Ok(Node::Group(cap, Box::new(inner)))
    }

    fn atom(&mut self) -> Result<Node> {
        let c = self.peek().ok_or_else(|| bad(self.i))?;
        self.i += 1;
        Ok(match c {
            '.' => Node::Any,
            '^' => Node::Begin,
            '$' => Node::End,
            '(' => return self.group(),
            '[' => return self.class(),
            '\\' => match self.escape()? {
                Item::Range(a, _) => Node::Lit(a),
                it => Node::Class(vec![it], false),
            },
            '*' | '+' | '?' => return Err(bad(self.i - 1)),
            c => Node::Lit(c),
        })
    }
}

struct Run<'a> {
    s: &'a [char],
    ci: bool,
    steps: u32,
    caps: Caps,
}

type K<'k> = &'k mut dyn FnMut(&mut Run, usize) -> bool;

fn item_hit(it: &Item, c: char) -> bool {
    match *it {
        Item::Range(a, b) => a <= c && c <= b,
        Item::Digit(n) => c.is_ascii_digit() != n,
        Item::Word(n) => (c.is_alphanumeric() || c == '_') != n,
        Item::Space(n) => c.is_whitespace() != n,
    }
}

impl Run<'_> {
    fn class_hit(&self, items: &[Item], neg: bool, c: char) -> bool {
        let one = |c: char| items.iter().any(|it| item_hit(it, c));
        let hit = one(c) || (self.ci && (c.to_lowercase().any(one) || c.to_uppercase().any(one)));
        hit != neg
    }

    fn lit_hit(&self, a: char, b: char) -> bool {
        a == b || (self.ci && a.to_lowercase().eq(b.to_lowercase()))
    }

    fn step(&mut self, n: &Node, i: usize, k: K) -> bool {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return false;
        }
        let at = self.s.get(i).copied();
        match n {
            Node::Lit(a) => at.is_some_and(|c| self.lit_hit(*a, c)) && k(self, i + 1),
            Node::Any => at.is_some() && k(self, i + 1),
            Node::Class(items, neg) => {
                at.is_some_and(|c| self.class_hit(items, *neg, c)) && k(self, i + 1)
            }
            Node::Begin => i == 0 && k(self, i),
            Node::End => i == self.s.len() && k(self, i),
            Node::Group(cap, inner) => self.step(inner, i, &mut |me, j| {
                let old = cap.map(|c| me.caps[c]);
                if let Some(c) = cap {
                    me.caps[*c] = Some((i, j));
                }
                if k(me, j) {
                    return true;
                }
                if let (Some(c), Some(o)) = (cap, old) {
                    me.caps[*c] = o;
                }
                false
            }),
            Node::Cat(v) => self.cat(v, i, k),
            Node::Alt(v) => v.iter().any(|a| self.step(a, i, &mut *k)),
            Node::Rep(a, lo, hi, lazy) => self.rep(a, (*lo, *hi, *lazy), 0, i, k),
        }
    }

    fn cat(&mut self, v: &[Node], i: usize, k: K) -> bool {
        match v.split_first() {
            None => k(self, i),
            Some((a, rest)) => self.step(a, i, &mut |me, j| me.cat(rest, j, &mut *k)),
        }
    }

    fn rep(&mut self, a: &Node, r: (u32, u32, bool), n: u32, i: usize, k: K) -> bool {
        let (lo, hi, lazy) = r;
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return false;
        }
        let more = |me: &mut Run, k: K| {
            n < hi
                && me.step(a, i, &mut |m2, j| {
                    // an empty round would loop forever
                    (j != i || n < lo) && m2.rep(a, r, n + 1, j, &mut *k)
                })
        };
        if n < lo {
            more(self, k)
        } else if lazy {
            k(self, i) || more(self, k)
        } else {
            more(self, &mut *k) || k(self, i)
        }
    }
}

impl Regex {
    /// Compiles `pattern`; an error is `error.badModello` with the position.
    pub fn new(pattern: &str) -> Result<Regex> {
        if pattern.chars().count() > MAX_PATTERN {
            return Err(bad(MAX_PATTERN));
        }
        let (ci, body) = match pattern.strip_prefix("(?i)") {
            Some(b) => (true, b),
            None => (false, pattern),
        };
        let mut p = Parser {
            s: body.chars().collect(),
            i: 0,
            names: Vec::new(),
        };
        let root = p.alt()?;
        if p.i < p.s.len() {
            return Err(bad(p.i));
        }
        Ok(Regex {
            root,
            ci,
            names: p.names,
        })
    }

    /// The captures (char spans, one per group) when the whole of `text` matches; None when it does not, or is longer than the limit.
    pub fn full(&self, text: &[char]) -> Option<Caps> {
        if text.len() > MAX_TEXT {
            return None;
        }
        let mut r = Run {
            s: text,
            ci: self.ci,
            steps: 0,
            caps: vec![None; self.names.len()],
        };
        let n = text.len();
        let ok = r.step(&self.root, 0, &mut |_, j| j == n);
        ok.then_some(r.caps)
    }
}
