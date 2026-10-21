//! Tutor on the Teoria: the pages cut into citable blocks (`blocchi`, schema `teoria-blocchi/1`) and the BM25 search over them (`cerca`).
//! Used by the `teoria-blocchi` binary at build time and, later, by the tutor Worker; pure and deterministic (no clock, no I/O in the library).
pub mod blocchi;
pub mod cerca;
mod html;
mod pagina;

/// Schema name written in the blocks file.
pub const SCHEMA: &str = "teoria-blocchi/1";

/// The words of a text for search: normalised like `cerca.js` (lower case, no accents), split on anything but letters, digits and `+/.`,
/// with the dots, slashes and plus signs at the edges of a word dropped ("m/z" and "m+h" stay whole, "risoluzione." becomes "risoluzione").
pub fn words(text: &str) -> Vec<String> {
    mzlab_libro::norm(text)
        .split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '.')))
        .map(|w| w.trim_matches(|c| matches!(c, '+' | '/' | '.')))
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}
