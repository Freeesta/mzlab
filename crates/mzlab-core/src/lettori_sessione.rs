//! Which dataset each slot of the engine reads: the rule of `Nativi::attach` (`crates/mzlab-wasm/src/nativi_motore.rs`), kept here so it is tested without a browser.
//! A dataset (and its open reader) outlives the move of a slot between the samples of the same dataset; it is dropped only when no slot is left on it.
use std::collections::HashMap;

/// Points `slot` at sample `c` of dataset `ds`. `make` builds the dataset only when `ds` is not there already. Returns the dataset the slot left when
/// that one has no slot any more (and is removed from `sets`), so the caller can drop what it cached for it; `None` when nothing was dropped.
/// On an error of `make` nothing changes.
pub fn attacca<D, E>(
    sets: &mut HashMap<String, D>,
    slots: &mut HashMap<u32, (String, usize)>,
    slot: u32,
    ds: &str,
    c: usize,
    make: impl FnOnce() -> Result<D, E>,
) -> Result<Option<String>, E> {
    if !sets.contains_key(ds) {
        sets.insert(ds.to_string(), make()?);
    }
    let left = match slots.insert(slot, (ds.to_string(), c)) {
        Some((old, _)) if old != ds && !slots.values().any(|(d, _)| *d == old) => Some(old),
        _ => None,
    };
    if let Some(old) = &left {
        sets.remove(old);
    }
    Ok(left)
}

/// Frees `slot`; the dataset it read is dropped (and returned) when no other slot reads it.
pub fn stacca<D>(
    sets: &mut HashMap<String, D>,
    slots: &mut HashMap<u32, (String, usize)>,
    slot: u32,
) -> Option<String> {
    let (ds, _) = slots.remove(&slot)?;
    if slots.values().any(|(d, _)| *d == ds) {
        return None;
    }
    sets.remove(&ds);
    Some(ds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_slot_visiting_the_samples_of_a_dataset_opens_it_once() {
        let (mut sets, mut slots) = (HashMap::new(), HashMap::new());
        let mut aperti = 0;
        for giro in 0..2 {
            for c in 0..14 {
                let _ = giro;
                let r: Result<_, ()> = attacca(&mut sets, &mut slots, 1, "ReRun", c, || {
                    aperti += 1;
                    Ok(())
                });
                assert_eq!(r, Ok(None));
            }
        }
        assert_eq!(aperti, 1);
        assert_eq!(stacca(&mut sets, &mut slots, 1).as_deref(), Some("ReRun"));
        assert!(sets.is_empty() && slots.is_empty());
    }

    #[test]
    fn a_dataset_lives_while_a_slot_reads_it() {
        let (mut sets, mut slots) = (HashMap::new(), HashMap::new());
        let mk = || Ok::<_, ()>(());
        attacca(&mut sets, &mut slots, 1, "a", 0, mk).unwrap();
        attacca(&mut sets, &mut slots, 2, "a", 1, mk).unwrap();
        assert_eq!(stacca(&mut sets, &mut slots, 1), None);
        assert!(sets.contains_key("a"));
        // moving the last slot to another dataset drops the one it left
        assert_eq!(
            attacca(&mut sets, &mut slots, 2, "b", 0, mk)
                .unwrap()
                .as_deref(),
            Some("a")
        );
        assert!(!sets.contains_key("a") && sets.contains_key("b"));
    }

    #[test]
    fn a_failed_make_changes_nothing() {
        let (mut sets, mut slots) = (HashMap::new(), HashMap::new());
        attacca(&mut sets, &mut slots, 1, "a", 0, || Ok::<_, ()>(())).unwrap();
        assert!(attacca(&mut sets, &mut slots, 1, "b", 0, || Err(())).is_err());
        assert_eq!(slots.get(&1), Some(&("a".to_string(), 0)));
        assert!(sets.contains_key("a") && !sets.contains_key("b"));
    }
}
