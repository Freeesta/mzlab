//! `metodo/1` for the page (`mzlab-core::metodo`): the method of an item from the mzML that carries it, from the `.dam` preselected for it, or from the `.wiff` it was made from
//! (found by the SHA-1 that ProteoWizard writes in the mzML); the `.wiff` is only read, never converted here.
use super::{error_json, Engine, JsReader};
use crate::natives::Src;
use js_sys::Function;
use mzlab_core::idxread::ReadAt;
use mzlab_core::labmethods;
use mzlab_core::method::{self, Source, SrcFile};
use mzlab_core::mzmlhead::Mzml;
use wasm_bindgen::prelude::*;

/// How much of the mzML after `<run ` is read for the `metodo/1` document it carries (the texts of a `.raw` method and its `campi` take ~100 kB).
const DOC_MAX: u64 = 1 << 20;
/// How much of the start of an mzML is searched for the source files and the first `sample=N`.
const HEAD: usize = 1 << 20;

#[wasm_bindgen]
impl Engine {
    /// SHA-1 (hex) of a file read through `read(offset, length)`.
    pub fn sha1_file(&mut self, size: f64, read: Function) -> Result<String, JsValue> {
        method::sha1_dyn(&mut JsReader { read }, size as u64).map_err(|e| error_json(&e))
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
        for (i, (name, sha)) in method::source_sha1s(&t).iter().enumerate() {
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
            .and_then(|p| method::sample_of_id(&t[p..]))
        {
            Some(s) => o.int(s as i64),
            None => o.raw("null"),
        };
        Ok(o.raw("}").0.clone())
    }

    /// The `metodo/1` document an mzML carries in its `<run>` (`""` when it has none).
    pub fn mzml_metodo(&mut self, size: f64, read: Function) -> Result<String, JsValue> {
        // through `Src`, as the routes read an mzML: one copy of `Mzml` in the engine
        let mut m = Mzml::new(Src::Js(JsReader { read }), size as u64);
        let head = m.run_head(DOC_MAX).map_err(|e| error_json(&e))?;
        Ok(method::from_userparam(&head).unwrap_or_default())
    }

    /// `metodo/1` of an mzML from its header and first scans (`""` when it has no header); a document the file carries wins.
    pub fn mzml_head_metodo(
        &mut self,
        name: &str,
        size: f64,
        read: Function,
    ) -> Result<String, JsValue> {
        let mut m = Mzml::new(Src::Js(JsReader { read }), size as u64);
        let doc = m.method(name, DOC_MAX).map_err(|e| error_json(&e))?;
        Ok(doc.unwrap_or_default())
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
        Ok(method::from_dam(m, &src))
    }
}
