//! The presets of `features.detect` hold the constants of the profiles and the descriptor writes the recipe `Recipe::nominal` writes.

use crate::steps::registry;
use crate::recipe::Recipe;
use crate::recipe_lr::{LR_FLOOR, LR_MIN_HEIGHT, LR_MIN_SNR};
use crate::steps::recipe_map;
use mzlab_core::step::find;

/// The two presets of `features.detect` hold the constants of the two profiles (`Params::default`, `LR_FLOOR`, `LR_MIN_HEIGHT`).
#[test]
fn presets_are_the_constants_of_the_two_profiles() {
    let reg = registry();
    let detect = find(&reg, "features.detect").unwrap();
    let preset = |id: &str, n: &str| {
        let p = &detect.presets.iter().find(|p| p.id == id).unwrap().params;
        p.iter().find(|(k, _)| *k == n).unwrap().1.clone()
    };
    let hr = Recipe::default().detect;
    assert_eq!(preset("hr", "floor").f64(), Some(hr.floor));
    assert_eq!(preset("hr", "min_height").f64(), Some(hr.min_height));
    assert_eq!(preset("hr", "axis").str(), Some("ppm"));
    let lr = Recipe::lr(0.0, LR_FLOOR, LR_MIN_HEIGHT).detect;
    assert_eq!(preset("lr", "floor").f64(), Some(lr.floor));
    assert_eq!(preset("lr", "min_height").f64(), Some(lr.min_height));
    assert!(lr.axis.is_da());
    assert_eq!(preset("lr", "min_snr").f64(), Some(LR_MIN_SNR));
    assert_eq!(preset("lr", "axis").str(), Some("Da"));
    assert_eq!(
        preset("nominal", "floor").f64(),
        Some(crate::recipe_lr::NOMINAL_FLOOR)
    );
    assert_eq!(
        preset("nominal", "grid_touch").f64(),
        Some(crate::recipe_lr::NOMINAL_GRID_TOUCH)
    );
    assert!(detect
        .presets
        .iter()
        .find(|p| p.id == "nominal")
        .unwrap()
        .when
        .resolution
        .is_empty());
    // `Recipe::nominal` (the `features` params key) and the descriptor preset (the Flow) write the same recipe
    let mut by_preset = Recipe::lr(0.0, LR_FLOOR, LR_MIN_HEIGHT);
    let values: Vec<(String, mzlab_core::json::Value)> = detect
        .presets
        .iter()
        .find(|p| p.id == "nominal")
        .unwrap()
        .params
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    recipe_map::apply(&mut by_preset, "features.detect", &values).unwrap();
    let mut by_key = Recipe::lr(0.0, LR_FLOOR, LR_MIN_HEIGHT);
    assert!(by_key.nominal());
    assert_eq!(
        format!("{:?}", by_preset.detect),
        format!("{:?}", by_key.detect)
    );
    assert!(
        !Recipe::default().nominal(),
        "a high-resolution recipe is left alone"
    );
    assert_eq!(detect.default_preset, Some("hr"));
}
