//! The one glob of the engine (`*` any run, `?` one character) over characters: the column selectors of `table compare` and the blank files of
//! `quant.targets`.

/// `*` and `?` over characters; the caller lowers both sides for a case-insensitive match.
pub fn glob(pat: &[char], s: &[char]) -> bool {
    let (mut p, mut i, mut star, mut mark) = (0, 0, None, 0);
    while i < s.len() {
        if p < pat.len() && (pat[p] == '?' || pat[p] == s[i]) {
            p += 1;
            i += 1;
        } else if p < pat.len() && pat[p] == '*' {
            star = Some(p);
            mark = i;
            p += 1;
        } else if let Some(sp) = star {
            p = sp + 1;
            mark += 1;
            i = mark;
        } else {
            return false;
        }
    }
    pat[p..].iter().all(|&c| c == '*')
}
