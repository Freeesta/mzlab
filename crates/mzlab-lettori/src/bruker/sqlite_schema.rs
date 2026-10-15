//! The column names of a `CREATE TABLE` text.

/// The names of the columns of a `CREATE TABLE` text, and which one is an `INTEGER PRIMARY KEY` (the alias of the rowid). Table constraints are skipped.
pub(super) fn colonne_di(sql: &str) -> (Vec<String>, Option<usize>, Option<Vec<usize>>) {
    let Some(ap) = sql.find('(') else {
        return (Vec::new(), None, None);
    };
    let mut parti: Vec<String> = Vec::new();
    let (mut prof, mut cur) = (0usize, String::new());
    let mut virgolette: Option<char> = None;
    for ch in sql[ap + 1..].chars() {
        if let Some(q) = virgolette {
            cur.push(ch);
            if ch == q {
                virgolette = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => {
                virgolette = Some(ch);
                cur.push(ch);
            }
            '[' => {
                virgolette = Some(']');
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
            ',' if prof == 0 => parti.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    parti.push(cur);
    let mut nomi: Vec<String> = Vec::new();
    let mut alias = None;
    let mut chiave: Vec<String> = Vec::new();
    for p in parti {
        let t = p.trim();
        let primo = t.split_whitespace().next().unwrap_or("");
        let up = primo.to_ascii_uppercase();
        let tu = t.to_ascii_uppercase();
        if up == "PRIMARY" {
            if let (Some(a), Some(b)) = (t.find('('), t.rfind(')')) {
                chiave = t[a + 1..b]
                    .split(',')
                    .map(|n| nome_pulito(n.trim()))
                    .collect();
            }
            continue;
        }
        if matches!(up.as_str(), "FOREIGN" | "UNIQUE" | "CHECK" | "CONSTRAINT") || t.is_empty() {
            continue;
        }
        if tu.contains("INTEGER PRIMARY KEY") && !tu.contains("DESC") {
            alias = Some(nomi.len());
        }
        if tu.contains("PRIMARY KEY") {
            chiave = vec![nome_pulito(primo)];
        }
        nomi.push(nome_pulito(primo));
    }
    // a WITHOUT ROWID table stores its primary key first, then the other columns in table order
    let ordine = sql.to_ascii_uppercase().contains("WITHOUT ROWID").then(|| {
        let mut o: Vec<usize> = chiave
            .iter()
            .filter_map(|k| nomi.iter().position(|n| n.eq_ignore_ascii_case(k)))
            .collect();
        o.dedup();
        let resto: Vec<usize> = (0..nomi.len()).filter(|i| !o.contains(i)).collect();
        o.extend(resto);
        o
    });
    (nomi, alias, ordine)
}

fn nome_pulito(n: &str) -> String {
    n.trim_matches(|c| matches!(c, '"' | '\'' | '`' | '[' | ']'))
        .to_string()
}

#[cfg(test)]
mod prove {
    use super::*;

    #[test]
    fn colonne_e_alias() {
        let (c, a, _) = colonne_di(
            "CREATE TABLE Frames (Id INTEGER PRIMARY KEY, Time REAL NOT NULL, \"Polarity\" CHAR(1) CHECK (Polarity IN ('+', '-')), FOREIGN KEY (Id) REFERENCES X(Id))",
        );
        assert_eq!(c, ["Id", "Time", "Polarity"]);
        assert_eq!(a, Some(0));
    }
}
