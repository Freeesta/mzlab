//! The unit of a chromatogram of an mzML: the one on its intensity array or, when that is missing or only the generic `number of detector counts`
//! (what our own mzML writer puts on every trace but the TIC), the one the writer keeps in the `userParam` `unità`. Used by `mzmlhead::Mzml::chromatogram_unit`.

const GENERIC: &str = "number of detector counts";
const DECLARED: &str = "<userParam name=\"unità\" value=\"";

/// `array_unit` (from the intensity array) or, when it says nothing, the `unità` userParam of the chromatogram `chunk`.
pub(crate) fn resolve(array_unit: String, chunk: &[u8]) -> String {
    if !(array_unit.is_empty() || array_unit == GENERIC) {
        return array_unit;
    }
    let text = String::from_utf8_lossy(chunk);
    let head = text
        .find("<binaryDataArrayList")
        .map_or(&*text, |e| &text[..e]);
    let declared = head.find(DECLARED).and_then(|p| {
        let v = &head[p + DECLARED.len()..];
        v.find('"').map(|e| {
            v[..e]
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&amp;", "&")
        })
    });
    declared.unwrap_or(array_unit)
}
