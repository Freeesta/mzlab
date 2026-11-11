//! The `cvParam` names and `userParam` pairs of a header block, read by text as the rest of `mzmlhead` reads the header: a generic term written
//! with a text of the file (`mzmlwrite::header_term`) gives that text in place of its CV name.

/// `name="..."` as the LAST `name="` of a tag head (the regex `<cvParam [^>]*name="([^"]*)"` is greedy), for every `<cvParam ` of `block`.
pub(super) fn names(block: &str) -> Vec<String> {
    let mut out = vec![];
    let mut rest = block;
    while let Some(p) = rest.find("<cvParam ") {
        let tag = &rest[p + 9..];
        let end = tag.find('>').unwrap_or(tag.len());
        let head = &tag[..end];
        match head.rfind("name=\"") {
            Some(q) => {
                let v = &head[q + 6..];
                match v.find('"') {
                    Some(e) => {
                        out.push(v[..e].to_string());
                        rest = &tag[q + 6 + e + 1..];
                    }
                    None => rest = &tag[end..],
                }
            }
            None => rest = &tag[end..],
        }
    }
    with_texts(block, &mut out);
    out
}

/// The `name="..." value="..."` of every `<userParam ` of `block` (unescaped).
fn user_params(block: &str) -> Vec<(String, String)> {
    let mut out = vec![];
    for t in block.split("<userParam ").skip(1) {
        let head = &t[..t.find('>').unwrap_or(t.len())];
        let get = |k: &str| {
            let v = &head[head.find(k)? + k.len()..];
            let v = &v[..v.find('"')?];
            Some(quick_xml::escape::unescape(v).map_or_else(|_| v.to_string(), |c| c.into_owned()))
        };
        if let (Some(n), Some(v)) = (get("name=\""), get("value=\"")) {
            out.push((n, v));
        }
    }
    out
}

/// The CV name of a generic term in `names` replaced by the text of its userParam (`mzmlwrite::header_term`).
pub(super) fn with_texts(block: &str, names: &mut [String]) {
    for (n, v) in user_params(block) {
        let generic = |x: &String| {
            crate::mzmlwrite::generic_name(x)
                .is_some_and(|c| crate::mzmlwrite::generic_user_name(c) == n)
        };
        if let Some(slot) = names.iter_mut().find(|x| generic(x)) {
            *slot = v;
        }
    }
}
