//! `tp.dig_lr` up to `lr::dig`, shared by the native executor (`mzlab-cli`, `flow_digl.rs`) and the browser (the export `tp_dig_inputs` below): the polarity, the hypotheses searched in
//! the stored peaks of the series and the JSON of the files and of the parent that the Dig reads.

use super::super::run;
use super::LinkParent;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::Stored;

/// What the Dig of a unit-resolution series (`lr::dig`) is given besides the table: the files and samples, the parent and the `ipotesi/1` the search in the series annotated.
pub struct DigInputs {
    pub files: String,
    pub who: String,
    pub registry: String,
}

/// The polarity the Dig looks in: the recipe's, else the sign of the parent adduct, else what the MS1 scans of the files say (a mix or no scans read as positive).
fn dig_polarity<I: ReadAt>(adduct: &str, recipe: &Recipe, indexes: &[Index<I>]) -> i8 {
    recipe
        .detect
        .polarity
        .filter(|&p| p != 0)
        .or_else(|| adduct.ends_with('-').then_some(-1))
        .or_else(|| mzlab_feat::nature::of_indexes(indexes).polarity)
        .unwrap_or(1)
}

/// `tp.dig_lr` up to `lr::dig`, shared by the native executor (`flow_digl.rs`) and the browser (`tp_dig_inputs`): the polarity, the hypotheses searched in the stored peaks
/// of the series, and the JSON of the files (`names`, in the order of the table) and of the parent that the Dig reads. `[M+H]+` meeting a negative series is read as `[M-H]-`.
#[allow(clippy::too_many_arguments)]
pub fn dig_inputs<I: ReadAt>(
    rules: &[u8],
    p: &LinkParent,
    adduct: &str,
    names: &[String],
    samples: &str,
    recipe: &mut Recipe,
    indexes: &mut [Index<I>],
    peaks: &mut dyn Stored,
) -> Result<DigInputs> {
    let polarity = dig_polarity(adduct, recipe, indexes);
    recipe.detect.polarity = Some(polarity);
    let hypo = run::hypotheses_stored(indexes, peaks, rules, p.structure, p.formula, None, recipe)?;
    let mut files = Out::default();
    files.raw("{\"files\":[");
    for (i, n) in names.iter().enumerate() {
        files
            .raw(if i == 0 { "" } else { "," })
            .raw("{\"name\":")
            .str(n)
            .raw(",\"kind\":\"full\"}");
    }
    let samples = mzlab_core::samples::order_like(samples.trim(), names)?;
    files.raw("],\"campioni\":").raw(&samples).raw("}");
    let adduct = if adduct == "[M+H]+" && polarity < 0 {
        "[M-H]-"
    } else {
        adduct
    };
    let mut who = Out::default();
    who.raw("{\"name\":\"parent\",\"neutral\":")
        .str(p.formula)
        .raw(",\"adduct\":")
        .str(adduct)
        .raw("}");
    Ok(DigInputs {
        files: files.0,
        who: who.0,
        registry: hypo.registry.to_json(),
    })
}
