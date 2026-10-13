//! Stable pseudonyms for what identifies a person, a computer, an instrument or a path in an anonymous export:
//! `anon:<8 hex>` = the first four bytes of HMAC-SHA256(secret, kind 0x00 value). The secret is made by the page, lives only in that
//! browser (never exported) and reaches the engine through `set_current`: the same value gives the same pseudonym in every export
//! of that browser (so the runs of one operator or one instrument still group), and nothing can be computed back without the secret.
//! Without a secret the value is simply left out.

use sha2::{Digest, Sha256};
use std::cell::RefCell;

const BLOCK: usize = 64;

/// HMAC-SHA256 (RFC 2104).
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let pad = |b: u8| k.map(|x| x ^ b);
    let mut inner = Sha256::new();
    inner.update(pad(0x36));
    inner.update(msg);
    let mut outer = Sha256::new();
    outer.update(pad(0x5c));
    outer.update(inner.finalize());
    outer.finalize().into()
}

/// The pseudonym of `value` as a `kind` ("user", "computer", "serial", "path"); case and surrounding spaces do not matter.
pub fn pseudonym(secret: &[u8], kind: &str, value: &str) -> String {
    let mut msg = Vec::with_capacity(kind.len() + value.len() + 1);
    msg.extend_from_slice(kind.as_bytes());
    msg.push(0);
    msg.extend_from_slice(value.trim().to_lowercase().as_bytes());
    let mac = hmac_sha256(secret, &msg);
    let hex: String = mac[..4].iter().map(|b| format!("{b:02x}")).collect();
    format!("anon:{hex}")
}

thread_local! {
    static CURRENT: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
}

/// The secret the engine uses for the exports of this instance (an empty one clears it).
pub fn set_current(secret: &[u8]) {
    CURRENT.with(|c| *c.borrow_mut() = (!secret.is_empty()).then(|| secret.to_vec()));
}

/// The secret set by `set_current`, if any.
pub fn current() -> Option<Vec<u8>> {
    CURRENT.with(|c| c.borrow().clone())
}

/// The value to write for `value` in an anonymous document: its pseudonym, or `None` (left out) with no secret or no value.
pub fn alias(secret: Option<&[u8]>, kind: &str, value: &str) -> Option<String> {
    let v = value.trim();
    (!v.is_empty()).then_some(())?;
    secret.map(|s| pseudonym(s, kind, v))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn hmac_rfc4231_case_2() {
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex(&mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hmac_long_key_rfc4231_case_6() {
        let mac = hmac_sha256(
            &[0xaa; 131],
            b"Test Using Larger Than Block-Size Key - Hash Key First",
        );
        assert_eq!(
            hex(&mac),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn pseudonym_is_stable_and_separated() {
        let a = pseudonym(b"s1", "user", "Rossi");
        assert_eq!(a, pseudonym(b"s1", "user", " rossi "));
        assert!(a.starts_with("anon:") && a.len() == 13);
        assert_ne!(a, pseudonym(b"s2", "user", "Rossi"));
        assert_ne!(a, pseudonym(b"s1", "computer", "Rossi"));
    }

    #[test]
    fn alias_without_secret_or_value_is_none() {
        assert_eq!(alias(None, "user", "x"), None);
        assert_eq!(alias(Some(b"k"), "user", "  "), None);
        assert!(alias(Some(b"k"), "user", "x").is_some());
    }
}
