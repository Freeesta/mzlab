// Ported from Attributio curation.py (the `re.search` of the name map).
//! The small regular-expression dialect of the curation tables, searched anywhere in a lowercase name: literals, `.`, classes `[...]`, `\b \w \d \s` (and
//! their negations), `^ $`, groups `( )` and `(?: )`, alternation, the quantifiers `? * + {m} {m,n}`, and the lookarounds `(?= ) (?! ) (?<= ) (?<! )`. No
//! dependency: the `regex` crate has no lookaround and the tables use it (`(?<!nor)ketamine`). A pattern that does not parse never matches (the audit lists it).

#[derive(Clone, Debug)]
enum Item {
    Range(char, char),
    Word(bool),
    Digit(bool),
    Space(bool),
}

#[derive(Clone, Debug)]
enum Node {
    Char(char),
    Any,
    Class(Vec<Item>, bool),
    Start,
    End,
    WordB(bool),
    Seq(Vec<Node>),
    Alt(Vec<Node>),
    Rep(Box<Node>, usize, usize),
    Look(Box<Node>, bool, bool),
}

/// A parsed pattern.
#[derive(Clone, Debug)]
pub struct Pattern {
    root: Option<Node>,
    text: String,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl Item {
    fn has(&self, c: char) -> bool {
        match *self {
            Item::Range(a, b) => a <= c && c <= b,
            Item::Word(neg) => is_word(c) != neg,
            Item::Digit(neg) => c.is_ascii_digit() != neg,
            Item::Space(neg) => c.is_whitespace() != neg,
        }
    }
}

struct Parser {
    s: Vec<char>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }
    fn eat(&mut self, c: char) -> bool {
        let ok = self.peek() == Some(c);
        if ok {
            self.i += 1;
        }
        ok
    }
    fn starts(&self, t: &str) -> bool {
        t.chars()
            .enumerate()
            .all(|(k, c)| self.s.get(self.i + k) == Some(&c))
    }

    fn alt(&mut self) -> Option<Node> {
        let mut arms = vec![self.seq()?];
        while self.eat('|') {
            arms.push(self.seq()?);
        }
        Some(if arms.len() == 1 {
            arms.remove(0)
        } else {
            Node::Alt(arms)
        })
    }

    fn seq(&mut self) -> Option<Node> {
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.atom()?;
            out.push(self.quantified(atom)?);
        }
        Some(Node::Seq(out))
    }

    fn quantified(&mut self, atom: Node) -> Option<Node> {
        let (lo, hi) = match self.peek() {
            Some('?') => (0, 1),
            Some('*') => (0, usize::MAX),
            Some('+') => (1, usize::MAX),
            Some('{') => return self.braces(atom),
            _ => return Some(atom),
        };
        self.i += 1;
        self.eat('?'); // lazy marker: only the existence of a match matters
        Some(Node::Rep(Box::new(atom), lo, hi))
    }

    fn braces(&mut self, atom: Node) -> Option<Node> {
        let close = self.s[self.i..].iter().position(|&c| c == '}')? + self.i;
        let body: String = self.s[self.i + 1..close].iter().collect();
        let (lo, hi) = match body.split_once(',') {
            Some((a, b)) => (
                a.trim().parse().ok()?,
                if b.trim().is_empty() {
                    usize::MAX
                } else {
                    b.trim().parse().ok()?
                },
            ),
            None => {
                let n = body.trim().parse().ok()?;
                (n, n)
            }
        };
        self.i = close + 1;
        self.eat('?');
        Some(Node::Rep(Box::new(atom), lo, hi))
    }

    fn group(&mut self) -> Option<Node> {
        let (look, behind, neg) = if self.starts("?:") {
            self.i += 2;
            (false, false, false)
        } else if self.starts("?=") || self.starts("?!") {
            let n = self.s[self.i + 1] == '!';
            self.i += 2;
            (true, false, n)
        } else if self.starts("?<=") || self.starts("?<!") {
            let n = self.s[self.i + 2] == '!';
            self.i += 3;
            (true, true, n)
        } else {
            (false, false, false)
        };
        let inner = self.alt()?;
        self.eat(')').then_some(())?;
        Some(if look {
            Node::Look(Box::new(inner), behind, neg)
        } else {
            inner
        })
    }

    fn atom(&mut self) -> Option<Node> {
        let c = self.peek()?;
        self.i += 1;
        Some(match c {
            '(' => self.group()?,
            '[' => self.class()?,
            '.' => Node::Any,
            '^' => Node::Start,
            '$' => Node::End,
            '\\' => self.escape()?,
            '?' | '*' | '+' => return None,
            _ => Node::Char(c),
        })
    }

    fn escape(&mut self) -> Option<Node> {
        let c = self.peek()?;
        self.i += 1;
        Some(match c {
            'b' => Node::WordB(true),
            'B' => Node::WordB(false),
            'w' | 'W' | 'd' | 'D' | 's' | 'S' => Node::Class(vec![shorthand(c)], false),
            'n' => Node::Char('\n'),
            't' => Node::Char('\t'),
            _ => Node::Char(c),
        })
    }

    fn class(&mut self) -> Option<Node> {
        let neg = self.eat('^');
        let mut items = Vec::new();
        let mut first = true;
        loop {
            let c = self.peek()?;
            self.i += 1;
            if c == ']' && !first {
                break;
            }
            first = false;
            let lo = if c == '\\' {
                let e = self.peek()?;
                self.i += 1;
                if "wWdDsS".contains(e) {
                    items.push(shorthand(e));
                    continue;
                }
                match e {
                    'n' => '\n',
                    't' => '\t',
                    _ => e,
                }
            } else {
                c
            };
            if self.peek() == Some('-') && self.s.get(self.i + 1).is_some_and(|&n| n != ']') {
                self.i += 1;
                let mut hi = self.peek()?;
                self.i += 1;
                if hi == '\\' {
                    hi = self.peek()?;
                    self.i += 1;
                }
                items.push(Item::Range(lo, hi));
            } else {
                items.push(Item::Range(lo, lo));
            }
        }
        Some(Node::Class(items, neg))
    }
}

fn shorthand(c: char) -> Item {
    match c {
        'w' => Item::Word(false),
        'W' => Item::Word(true),
        'd' => Item::Digit(false),
        'D' => Item::Digit(true),
        's' => Item::Space(false),
        _ => Item::Space(true),
    }
}

type Cont<'a> = &'a mut dyn FnMut(usize) -> bool;

fn run(n: &Node, s: &[char], i: usize, k: Cont) -> bool {
    match n {
        Node::Char(c) => s.get(i) == Some(c) && k(i + 1),
        Node::Any => s.get(i).is_some_and(|&c| c != '\n') && k(i + 1),
        Node::Class(items, neg) => {
            s.get(i)
                .is_some_and(|&c| items.iter().any(|it| it.has(c)) != *neg)
                && k(i + 1)
        }
        Node::Start => i == 0 && k(i),
        Node::End => i == s.len() && k(i),
        Node::WordB(want) => {
            let before = i > 0 && is_word(s[i - 1]);
            let after = s.get(i).copied().is_some_and(is_word);
            ((before != after) == *want) && k(i)
        }
        Node::Seq(v) => seq(v, s, i, k),
        Node::Alt(v) => v.iter().any(|a| run(a, s, i, &mut *k)),
        Node::Rep(a, lo, hi) => rep(a, *lo, *hi, 0, s, i, k),
        Node::Look(a, behind, neg) => {
            let found = if *behind {
                (0..=i).any(|j| run(a, s, j, &mut |e| e == i))
            } else {
                run(a, s, i, &mut |_| true)
            };
            found != *neg && k(i)
        }
    }
}

fn seq(v: &[Node], s: &[char], i: usize, k: Cont) -> bool {
    match v.split_first() {
        None => k(i),
        Some((a, rest)) => run(a, s, i, &mut |j| seq(rest, s, j, &mut *k)),
    }
}

fn rep(a: &Node, lo: usize, hi: usize, count: usize, s: &[char], i: usize, k: Cont) -> bool {
    if count < hi
        && run(a, s, i, &mut |j| {
            (j != i || count < lo) && rep(a, lo, hi, count + 1, s, j, &mut *k)
        })
    {
        return true;
    }
    count >= lo && k(i)
}

impl Pattern {
    /// Parse `text`; a pattern outside the dialect is kept but never matches.
    pub fn new(text: &str) -> Pattern {
        let mut p = Parser {
            s: text.chars().collect(),
            i: 0,
        };
        let root = p.alt().filter(|_| p.i == p.s.len());
        Pattern {
            root,
            text: text.to_string(),
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Whether the pattern parsed.
    pub fn is_valid(&self) -> bool {
        self.root.is_some()
    }
    /// `re.search`: whether the pattern matches anywhere in `haystack`.
    pub fn search(&self, haystack: &str) -> bool {
        let Some(root) = &self.root else {
            return false;
        };
        let s: Vec<char> = haystack.chars().collect();
        (0..=s.len()).any(|i| run(root, &s, i, &mut |_| true))
    }
}
