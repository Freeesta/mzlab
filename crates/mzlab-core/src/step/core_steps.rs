//! Descriptors of the steps whose code lives in this crate: `import.raw` (`lettori::apri`), `convert.mzml` (`lettori_mzml::scrivi`) and
//! `index.mzidx` (`upload::indicizza_lettore`, or `mzidx::build_mzml_spill` from an mzML). The defaults are literals; a test compares them with the code.

use super::{DataType, Param, Port, RunsIn, Step};

/// The steps owned by `mzlab-core`, in the order of the flagship chain.
pub fn core_steps() -> Vec<Step> {
    vec![import_raw(), convert_mzml(), index_mzidx()]
}

fn import_raw() -> Step {
    Step {
        ins: vec![Port::new("files", DataType::Raw, "Vendor files").many()],
        outs: vec![Port::new("raw", DataType::Raw, "Recognised files").many()],
        runs_in: RunsIn {
            wasm: Some("readers (one module per family)"),
            wasm_ceiling_bytes: None,
            native: true,
        },
        errors: vec!["err.file.notAccepted", "error.memoryBudget"],
        per_item: true,
        ..Step::core(
            "import.raw",
            "Import raw files",
            "Recognises each vendor file set with the reader that accepts it and checks it against the limits of the platform; it changes nothing.",
        )
    }
}

fn convert_mzml() -> Step {
    let params = vec![
        Param::integer("zlib_level", "Compression level")
            .min(0.0)
            .max(9.0)
            .default_num(1.0)
            .doc("zlib level of the binary arrays, 0 (none) to 9 (smallest)")
            .advanced(),
        Param::boolean("numpress", "MS-Numpress")
            .default_bool(false)
            .doc("Lossy MS-Numpress linear prediction of m/z and short logged float of intensities, smaller files")
            .advanced(),
    ];
    Step {
        cv: Some("MS:1000544"),
        ins: vec![Port::new("raw", DataType::Raw, "Vendor file").many()],
        outs: vec![Port::new("mzml", DataType::Mzml, "mzML file").many()],
        params,
        runs_in: RunsIn {
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.file.notAccepted", "error.memoryBudget"],
        per_item: true,
        converts: Some((DataType::Raw, DataType::Mzml)),
        cost: Some(2.0),
        ..Step::core(
            "convert.mzml",
            "Convert to mzML",
            "Writes the spectra of each sample as mzML; the version of this step names the writer.",
        )
    }
}

fn index_mzidx() -> Step {
    Step {
        ins: vec![Port::new("source", DataType::Raw, "Vendor file or mzML")
            .many()
            .accepts(&[DataType::Mzml])],
        outs: vec![Port::new("index", DataType::Mzidx, "Scan index").many()],
        params: vec![Param::integer("budget", "Working memory")
            .min(1.0)
            .default_num(67_108_864.0)
            .unit("bytes")
            .doc("Bytes of peaks held in memory while the index is built; the rest goes to a scratch file and the bytes of the index do not change")
            .noresult()
            .advanced()],
        runs_in: RunsIn {
            wasm: Some("engine"),
            wasm_ceiling_bytes: Some(1_703_936),
            native: true,
        },
        errors: vec!["err.file.index", "err.file.notAccepted", "error.memoryBudget"],
        per_item: true,
        converts: Some((DataType::Raw, DataType::Mzidx)),
        cost: Some(1.0),
        ..Step::core(
            "index.mzidx",
            "Build scan index",
            "Reads the scans of each sample once and writes the .mzidx the later steps read in slices.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mzidx::DEFAULT_BUDGET;
    use crate::mzmlwrite::{Opzioni, LIVELLO_ZLIB};

    #[test]
    fn defaults_are_the_ones_of_the_code() {
        let steps = core_steps();
        let find = |id: &str| steps.iter().find(|s| s.id == id).unwrap();
        let o = Opzioni::default();
        let conv = find("convert.mzml");
        assert_eq!(
            conv.param("zlib_level").unwrap().default_f64(),
            Some(f64::from(o.livello))
        );
        assert_eq!(o.livello, LIVELLO_ZLIB);
        assert_eq!(
            conv.param("numpress").unwrap().default,
            Some(crate::json::Value::Bool(o.numpress))
        );
        let idx = find("index.mzidx");
        assert_eq!(
            idx.param("budget").unwrap().default_f64(),
            Some(DEFAULT_BUDGET as f64)
        );
    }

    #[test]
    fn json_is_valid_and_names_every_step() {
        let text = super::super::json::registry_json(&core_steps());
        let v = crate::json::parse(text.as_bytes()).unwrap();
        let ids: Vec<&str> = v
            .arr()
            .iter()
            .map(|s| s.get("id").unwrap().str().unwrap())
            .collect();
        assert_eq!(ids, ["import.raw", "convert.mzml", "index.mzidx"]);
    }
}
