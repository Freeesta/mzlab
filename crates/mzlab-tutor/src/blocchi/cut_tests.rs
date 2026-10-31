//! Equivalence of the fast `cut` with the previous char-vector implementation.
use super::*;

// The previous char-vector implementation, kept as the reference the faster `cut` must match.
fn cut_reference(p: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest: Vec<char> = p.chars().collect();
    while rest.len() > TARGET {
        let sentence = (0..TARGET)
            .rev()
            .find(|&i| matches!(rest[i], '.' | '!' | '?' | ';') && rest.get(i + 1) == Some(&' '));
        let at = sentence
            .map(|i| i + 1)
            .or_else(|| (TARGET..CEILING.min(rest.len())).find(|&i| rest[i] == ' '))
            .unwrap_or(CEILING.min(rest.len()));
        let head: String = rest[..at].iter().collect();
        out.push(head.trim().to_string());
        rest = rest[at..]
            .iter()
            .copied()
            .skip_while(|c| c.is_whitespace())
            .collect();
    }
    let tail: String = rest.iter().collect();
    if !tail.trim().is_empty() {
        out.push(tail.trim().to_string());
    }
    out
}

#[test]
fn cut_matches_the_char_vector_reference() {
    let words = [
        "alfa",
        "è",
        "perché.",
        "m/z",
        "ß!",
        "ångström?",
        "x;",
        "日本語",
        "  ",
        "\u{a0}",
        "13C",
    ];
    let mut seed = 12345u64;
    let mut next = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as usize
    };
    for case in 0..400 {
        let n = [0, 5, 130, 200, 400, 900][case % 6] + next() % 60;
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(words[next() % words.len()]);
            if case % 7 != 0 || next() % 5 == 0 {
                s.push(' ');
            }
        }
        if case % 11 == 0 {
            s = s.replace(' ', "");
        }
        assert_eq!(cut(&s), cut_reference(&s), "case {case}");
    }
}
