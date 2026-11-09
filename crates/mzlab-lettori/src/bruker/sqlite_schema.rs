//! The column names of a `CREATE TABLE` text.

/// The names of the columns of a `CREATE TABLE` text, and which one is an `INTEGER PRIMARY KEY` (the alias of the rowid). Table constraints are skipped.
pub(super) fn columns_of(sql: &str) -> (Vec<String>, Option<usize>, Option<Vec<usize>>) {
    let Some(ap) = sql.find('(') else {
        return (Vec::new(), None, None);
    };
    let mut parts: Vec<String> = Vec::new();
    let (mut prof, mut cur) = (0usize, String::new());
    let mut quotes: Option<char> = None;
    for ch in sql[ap + 1..].chars() {
        if let Some(q) = quotes {
            cur.push(ch);
            if ch == q {
                quotes = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => {
                quotes = Some(ch);
                cur.push(ch);
            }
            '[' => {
                quotes = Some(']');
                cur.push(ch);
            }
            '(' => {
                prof += 1;
                cur.push(ch);
            }
            ')' if prof == 0 => break,
            ')' => {
                prof -= 1;
                cur.push(ch);
            }
            ',' if prof == 0 => parts.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    parts.push(cur);
    let mut names: Vec<String> = Vec::new();
    let mut alias = None;
    let mut key: Vec<String> = Vec::new();
    for p in parts {
        let t = p.trim();
        let first_one = t.split_whitespace().next().unwrap_or("");
        let up = first_one.to_ascii_uppercase();
        let tu = t.to_ascii_uppercase();
        if up == "PRIMARY" {
            if let (Some(a), Some(b)) = (t.find('('), t.rfind(')')) {
                key = t[a + 1..b]
                    .split(',')
                    .map(|n| clean_name(n.trim()))
                    .collect();
            }
            continue;
        }
        if matches!(up.as_str(), "FOREIGN" | "UNIQUE" | "CHECK" | "CONSTRAINT") || t.is_empty() {
            continue;
        }
        if tu.contains("INTEGER PRIMARY KEY") && !tu.contains("DESC") {
            alias = Some(names.len());
        }
        if tu.contains("PRIMARY KEY") {
            key = vec![clean_name(first_one)];
        }
        names.push(clean_name(first_one));
    }
    // a WITHOUT ROWID table stores its primary key first, then the other columns in table order
    let order = sql.to_ascii_uppercase().contains("WITHOUT ROWID").then(|| {
        let mut o: Vec<usize> = key
            .iter()
            .filter_map(|k| names.iter().position(|n| n.eq_ignore_ascii_case(k)))
            .collect();
        o.dedup();
        let remainder: Vec<usize> = (0..names.len()).filter(|i| !o.contains(i)).collect();
        o.extend(remainder);
        o
    });
    (names, alias, order)
}

fn clean_name(n: &str) -> String {
    n.trim_matches(|c| matches!(c, '"' | '\'' | '`' | '[' | ']'))
        .to_string()
}

#[cfg(test)]
mod prove {
    use super::*;

    #[test]
    fn columns_and_alias() {
        let (c, a, _) = columns_of(
            "CREATE TABLE Frames (Id INTEGER PRIMARY KEY, Time REAL NOT NULL, \"Polarity\" CHAR(1) CHECK (Polarity IN ('+', '-')), FOREIGN KEY (Id) REFERENCES X(Id))",
        );
        assert_eq!(c, ["Id", "Time", "Polarity"]);
        assert_eq!(a, Some(0));
    }
}
