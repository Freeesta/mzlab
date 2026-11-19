//! Pretty-printer of the file `tools/1`: long lists get one element per line, short containers stay on one line.

/// Containers whose compact form is at most this long stay on one line in the exported file.
const INLINE: usize = 100;

/// End of the JSON value that starts at `i` in `s` (compact, no whitespace).
fn value_end(s: &[u8], mut i: usize) -> usize {
    match s[i] {
        b'"' => {
            i += 1;
            while s[i] != b'"' {
                i += if s[i] == b'\\' { 2 } else { 1 };
            }
            i + 1
        }
        b'[' | b'{' => {
            let close = if s[i] == b'[' { b']' } else { b'}' };
            i += 1;
            while s[i] != close {
                if s[i] == b',' || s[i] == b':' {
                    i += 1;
                } else {
                    i = value_end(s, i);
                }
            }
            i + 1
        }
        _ => {
            while !matches!(s[i], b',' | b']' | b'}') {
                i += 1;
            }
            i
        }
    }
}

/// Writes the value at `i` with one element or member per line, except short containers; returns the end of the value.
pub(super) fn spread(s: &[u8], i: usize, depth: usize, out: &mut String) -> usize {
    let end = value_end(s, i);
    let text = std::str::from_utf8(&s[i..end]).expect("UTF-8");
    if !matches!(s[i], b'[' | b'{') || end - i <= INLINE {
        out.push_str(text);
        return end;
    }
    let close = if s[i] == b'[' { ']' } else { '}' };
    out.push(s[i] as char);
    let mut at = i + 1;
    let pad = "  ".repeat(depth + 1);
    while s[at] != close as u8 {
        out.push('\n');
        out.push_str(&pad);
        if s[at] == b'"' && s[i] == b'{' {
            let key_end = value_end(s, at);
            out.push_str(std::str::from_utf8(&s[at..=key_end]).expect("UTF-8"));
            at = key_end + 1;
        }
        at = spread(s, at, depth + 1, out);
        if s[at] == b',' {
            out.push(',');
            at += 1;
        }
    }
    out.push('\n');
    out.push_str(&"  ".repeat(depth));
    out.push(close);
    at + 1
}
