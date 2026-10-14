//! `metodo/1` for the page (`mzlab-core::metodo`): the method of an item from the mzML that carries it, from the `.dam` preselected for it, or from the `.wiff` it was made from
//! (found by the SHA-1 that ProteoWizard writes in the mzML); the `.wiff` is only read, never converted here.
use super::{error_json, Engine, JsReader};
use js_sys::Function;
use mzlab_core::idxread::ReadAt;
use mzlab_core::labmethods;
use mzlab_core::metodo::{self, Source, SrcFile};
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::ole::Ole;
use wasm_bindgen::prelude::*;

/// A `.wiff` is read whole to open its container: more than this is refused with the memory budget error (the files of the 3200 are tens of MB).
const WIFF_MAX: u64 = 192 << 20;
/// How much of the mzML after `<run ` is read for the `metodo/1` document it carries (the texts of a `.raw` method and its `campi` take ~100 kB).
const DOC_MAX: u64 = 1 << 20;
/// How much of the start of an mzML is searched for the source files and the first `sample=N`.
const HEAD: usize = 1 << 20;

#[wasm_bindgen]
impl Engine {
    /// SHA-1 (hex) of a file read through `read(offset, length)`.
    pub fn sha1_file(&mut self, size: f64, read: Function) -> Result<String, JsValue> {
        metodo::sha1_hex(&mut JsReader { read }, size as u64).map_err(|e| error_json(&e))
    }

    /// `{"sources":[{"name","sha1"}],"sample":N|null}` from the start of an mzML: the source files it says it was made from and the sample of its first nativeID.
    pub fn mzml_sources(&mut self, size: f64, read: Function) -> Result<String, JsValue> {
        let n = (size as usize).min(HEAD);
        let mut b = vec![0u8; n];
        JsReader { read }
            .read_at(0, &mut b)
            .map_err(|e| error_json(&e))?;
        let t = String::from_utf8_lossy(&b);
        let mut o = mzlab_core::json::Out::default();
        o.raw("{\"sources\":[");
        for (i, (name, sha)) in metodo::source_sha1s(&t).iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" })
                .raw("{\"name\":")
                .str(name)
                .raw(",\"sha1\":")
                .str(sha)
                .raw("}");
        }
        o.raw("],\"sample\":");
        match t
            .find("sample=")
            .and_then(|p| metodo::sample_of_id(&t[p..]))
        {
            Some(s) => o.int(s as i64),
            None => o.raw("null"),
        };
        Ok(o.raw("}").0.to_string())
    }

    /// The `metodo/1` document an mzML carries in its `<run>` (`""` when it has none).
    pub fn mzml_metodo(&mut self, size: f64, read: Function) -> Result<String, JsValue> {
        let mut m = Mzml::new(JsReader { read }, size as u64);
        let head = m.run_head(DOC_MAX).map_err(|e| error_json(&e))?;
        Ok(metodo::from_userparam(&head).unwrap_or_default())
    }

    /// `metodo/1` of one sample of a `.wiff` (`size` bytes read through `read`); `sha1` is its SHA-1 when known.
    pub fn wiff_metodo(
        &mut self,
        name: &str,
        size: f64,
        read: Function,
        sample: u32,
        sha1: &str,
    ) -> Result<String, JsValue> {
        if size as u64 > WIFF_MAX {
            return Err(error_json(&mzlab_core::Error::memory_budget()));
        }
        let mut data = vec![0u8; size as usize];
        JsReader { read }
            .read_at(0, &mut data)
            .map_err(|e| error_json(&e))?;
        let ole = Ole::open(data, name).map_err(|e| error_json(&e))?;
        let file = SrcFile {
            name: name.to_string(),
            sha1: (!sha1.is_empty()).then(|| sha1.to_string()),
            bytes: size as u64,
        };
        let src = Source {
            format: "sciex-wiff",
            files: vec![file],
            sample: Some(sample),
        };
        Ok(mzlab_core::metodo_wiff::from_wiff(&ole, sample, &src))
    }

    /// `metodo/1` of an Agilent method folder `*.m` outside any run (`mzlab-core::agilent::quant`): `names` = the paths inside the folder, one per line,
    /// `lens` = the length of each file, `data` = the bytes of all of them one after the other.
    pub fn agilent_m_metodo(
        &mut self,
        name: &str,
        names: &str,
        lens: &[u32],
        data: &[u8],
    ) -> String {
        let mut files = Vec::new();
        let mut at = 0usize;
        for (n, len) in names.split('\n').zip(lens) {
            let end = (at + *len as usize).min(data.len());
            files.push((n, &data[at..end]));
            at = end;
        }
        mzlab_core::agilent::quant::metodo_m(name, &files)
    }

    /// `metodo/1` of the `.dam` preselected for the item `item` of the file at `slot`; `""` when no method is loaded.
    pub fn lab_metodo(
        &mut self,
        slot: u32,
        item: &str,
        size: f64,
        read: Function,
    ) -> Result<String, JsValue> {
        let data = self.item_data(slot, item, size, read)?;
        let labs = self.labs();
        let Some(i) = labmethods::pick(&data, &labs, item).map_err(|e| error_json(&e))? else {
            return Ok(String::new());
        };
        let Some(Ok(m)) = labs.get(i).map(|l| &l.res) else {
            return Ok(String::new());
        };
        let src = Source {
            format: "sciex-dam",
            files: vec![SrcFile {
                name: labs[i].name.clone(),
                ..Default::default()
            }],
            ..Default::default()
        };
        Ok(metodo::from_dam(m, &src))
    }
}
