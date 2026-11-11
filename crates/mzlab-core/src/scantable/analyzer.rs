//! The names of the analyzers of an instrument configuration as the text of the header gives them (`Head::parse`): what `Head::da_testata` reads
//! from the `Header` without writing it.

/// The `name` of every `cvParam` and the `value` of every `userParam` of the `<analyzer>` elements of an instrument configuration.
pub(super) fn analyzer_names(block: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut a = 0;
    while let Some(p) = block[a..].find("<analyzer") {
        let st = a + p;
        let Some(e) = block[st..].find("</analyzer>") else {
            break;
        };
        let blk = &block[st..st + e];
        let mut c = 0;
        while let Some(q) = blk[c..].find("<cvParam ") {
            let ts = c + q;
            let te = blk[ts..].find('>').map_or(blk.len(), |g| ts + g);
            if let Some(n) = blk[ts..te].rfind("name=\"") {
                let ns = ts + n + 6;
                if let Some(ne) = blk[ns..].find('"') {
                    names.push(blk[ns..ns + ne].to_string());
                }
            }
            c = te;
        }
        // the text of a generic analyzer term (`mzmlwrite::header_term`), as `da_testata` sees it in the name
        for u in blk.split("<userParam ").skip(1) {
            let u = &u[..u.find('>').unwrap_or(u.len())];
            if let Some(v) = u.find("value=\"").map(|i| &u[i + 7..]) {
                names.push(v[..v.find('"').unwrap_or(v.len())].to_string());
            }
        }
        a = st + e;
    }
    names
}
