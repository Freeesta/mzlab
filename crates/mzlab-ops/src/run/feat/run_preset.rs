//! The `mode` and `preset` keys of the recipe text of `features --params` and of the browser panel: the grouping mode, and the named profile of
//! `features.detect` (`nominal`: `Recipe::nominal`, the same recipe as the descriptor preset of the Flow). A high-resolution recipe keeps its values.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;
use mzlab_feat::recipe::{Mode, Recipe};

pub(super) fn named(r: &mut Recipe, v: &Value) -> Result<()> {
    match v.get("mode").and_then(Value::str) {
        Some("ion") => r.mode = Mode::Ion,
        Some("family") | None => {}
        Some(_) => return Err(Error::new("error.badRecipe")),
    }
    match v.get("preset").and_then(Value::str) {
        Some("nominal") => {
            r.nominal();
        }
        Some(_) => return Err(Error::new("error.badRecipe")),
        None => {}
    }
    Ok(())
}
