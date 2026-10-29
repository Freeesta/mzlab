//! Tests of the sibling module (loaded with `#[path]`, so they see its private items).

use super::*;
/// The character-by-character tokenizer this module used before the borrowing one: the fuzz test below holds the new one to the same records.
mod reference {
    pub fn parse_old(content: &str) -> Result<Vec<Vec<String>>, super::ParseError> {
        let mut records = Vec::new();
        let mut current_record = Vec::new();
        let mut current_field = String::new();
        let mut in_quotes = false;
        let mut chars = content.chars().peekable();
        let mut line_num = 1usize;
        let mut field_started = false;

        while let Some(c) = chars.next() {
            if in_quotes {
                if c == '"' {
                    if let Some(&'"') = chars.peek() {
                        chars.next();
                        current_field.push('"');
                    } else {
                        in_quotes = false;
                    }
                } else {
                    if c == '\n' {
                        line_num += 1;
                    }
                    current_field.push(c);
                }
            } else {
                match c {
                    '"' => {
                        if !field_started {
                            in_quotes = true;
                            field_started = true;
                        } else {
                            current_field.push('"');
                        }
                    }
                    ',' => {
                        current_record.push(std::mem::take(&mut current_field));
                        field_started = false;
                    }
                    '\r' => {
                        if let Some(&'\n') = chars.peek() {
                            chars.next();
                        }
                        line_num += 1;
                        current_record.push(std::mem::take(&mut current_field));
                        field_started = false;
                        records.push(std::mem::take(&mut current_record));
                    }
                    '\n' => {
                        line_num += 1;
                        current_record.push(std::mem::take(&mut current_field));
                        field_started = false;
                        records.push(std::mem::take(&mut current_record));
                    }
                    _ => {
                        field_started = true;
                        current_field.push(c);
                    }
                }
            }
        }

        if in_quotes {
            return Err(super::ParseError::CsvFormat {
                line: line_num,
                message: "Unclosed quote at end of CSV".to_string(),
            });
        }

        if field_started || !current_field.is_empty() || !current_record.is_empty() {
            current_record.push(current_field);
            records.push(current_record);
        }

        Ok(records)
    }
}

fn rows(s: &str) -> Vec<Vec<String>> {
    parse_rfc4180_str(s).unwrap()
}

#[test]
fn tokenizer_matches_the_reference_on_random_text() {
    let alphabet = ['a', 'b', ',', ',', '"', '"', '\r', '\n', ' ', 'é', '|'];
    let mut x = 0x2545F4914F6CDD1Du64;
    for _ in 0..40_000 {
        let n = {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % 24) as usize
        };
        let text: String = (0..n)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        match (reference::parse_old(&text), parse_rfc4180_str(&text)) {
            (Ok(a), Ok(b)) => assert_eq!(a, b, "{text:?}"),
            (Err(a), Err(b)) => assert_eq!(format!("{a}"), format!("{b}"), "{text:?}"),
            (a, b) => panic!("{text:?}: {a:?} vs {b:?}"),
        }
    }
}

#[test]
fn crlf_quotes_blank_lines_and_missing_final_newline() {
    assert_eq!(
        rows("a,b\r\n\"x,1\",\"say \"\"hi\"\"\"\r\n\r\nlast,"),
        vec![
            vec!["a", "b"],
            vec!["x,1", "say \"hi\""],
            vec![""],
            vec!["last", ""]
        ]
    );
    assert_eq!(rows("a\nb\n"), vec![vec!["a"], vec!["b"]]);
    assert_eq!(rows(""), Vec::<Vec<String>>::new());
    assert_eq!(rows("\"multi\nline\",z"), vec![vec!["multi\nline", "z"]]);
    assert!(parse_rfc4180_str("a,\"open\nrow").is_err());
}

#[test]
fn quotes_in_the_middle_of_a_field_are_kept_as_data() {
    // what the tolerant reader does with the two RFC violations (no character is lost but the structural quotes)
    assert_eq!(rows("a\"b,c"), vec![vec!["a\"b", "c"]]);
    assert_eq!(rows("5\" pipe"), vec![vec!["5\" pipe"]]);
    assert_eq!(rows("\"x\" ,y"), vec![vec!["x ", "y"]]);
    assert_eq!(rows("\"x\"y,z"), vec![vec!["xy", "z"]]);
    assert_eq!(rows("\"x\" \"y\",z"), vec![vec!["x \"y\"", "z"]]);
    assert_eq!(rows(" \"x\",z"), vec![vec![" \"x\"", "z"]]);
}

#[test]
fn a_leading_byte_order_mark_is_not_part_of_the_first_header() {
    assert_eq!(
        rows("\u{feff}id,x\n1,2"),
        vec![vec!["id", "x"], vec!["1", "2"]]
    );
    let csv = "\u{feff}id,name_en,name_it,neutral_formula,monoisotopic_mass,mz_mh,mz_mna,mz_mnh4,smiles,inchikey,inchikey_connectivity,cas,pubchem_cid,effect_class,chemical_class,role,psychoactive,synonyms\r\nx,X,X,CH4,16.0313,17.0386,39.0206,34.0652,C,,,,,,,,no,\r\n";
    let e = parse_target_csv(csv.as_bytes()).unwrap();
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].id, "x");
}
