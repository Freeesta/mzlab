//! The readers of the vendor files (`mzlab-core::lettori`) for the page: the page gives the members of a dataset (`.D` folder, `.wiff` + `.wiff.scan`,
//! `.raw`) as names, sizes and a `read(member, offset, length)` callback; the registry recognises the format and the same calls answer for all of them.
use js_sys::{Array, Float64Array, Function, Uint8Array};
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::Out;
use mzlab_core::lettori_mzml;
use mzlab_core::lettori::{self, Campione, Campo, FileSet, Lettore, Limiti, Membro, VoceScansione};
use wasm_bindgen::prelude::*;

/// An error as the JSON the page reads: `{"error_key", "params"}`.
fn error_json(e: &mzlab_core::Error) -> JsValue {
    let mut o = Out::default();
    o.raw("{\"error_key\":").str(e.key).raw(",\"params\":{");
    for (i, (k, v)) in e.params.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
    }
    o.raw("}}");
    JsValue::from_str(&o.0)
}

/// The output of the mzML: the writes are gathered in pieces of about 64 KB and each goes to the page as a `Uint8Array`.
struct Pieces(Function, Vec<u8>);

impl std::io::Write for &mut Pieces {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.1.extend_from_slice(b);
        if self.1.len() >= 1 << 16 {
            self.flush()?;
        }
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if !self.1.is_empty() {
            let piece = Uint8Array::from(&self.1[..]);
            self.1.clear();
            self.0
                .call1(&JsValue::NULL, &piece)
                .map_err(|_| std::io::Error::other("sink"))?;
        }
        Ok(())
    }
}

/// One member of the set, read by the page.
struct Dal {
    read: Function,
    membro: u32,
}

impl ReadAt for Dal {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let bad = || mzlab_core::Error::new("error.badIndex");
        let args = [
            self.membro.into(),
            (off as f64).into(),
            (buf.len() as f64).into(),
        ];
        let v = self
            .read
            .call3(&JsValue::NULL, &args[0], &args[1], &args[2])
            .map_err(|_| bad())?;
        let a: Uint8Array = v.dyn_into().map_err(|_| bad())?;
        if a.length() as usize != buf.len() {
            return Err(bad());
        }
        a.copy_to(buf);
        Ok(())
    }
}

/// `[x, x, ...]` with `one` writing each element.
fn list<T>(items: &[T], one: impl Fn(&mut Out, &T)) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, x) in items.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        one(&mut o, x);
    }
    o.raw("]");
    o.0
}

fn campione_json(o: &mut Out, c: &Campione) {
    o.raw("{\"id\":").int(c.id as i64);
    o.raw(",\"nome\":").str(&c.nome);
    o.raw(",\"scansioni\":").int(c.scansioni as i64).raw("}");
}

fn voce_json(o: &mut Out, v: &VoceScansione) {
    o.raw("{\"id\":").str(&v.id);
    o.raw(",\"rt\":").num(v.rt);
    o.raw(",\"livello\":").int(v.livello as i64);
    o.raw(",\"polarita\":").int(v.polarita as i64);
    o.raw(",\"tic\":").num(v.tic);
    o.raw(",\"picco_base\":").opt(v.picco_base);
    o.raw(",\"filtro\":").str(&v.filtro);
    o.raw(",\"precursore\":").opt(v.precursore).raw("}");
}

#[wasm_bindgen]
pub struct LettoreWasm {
    inner: Box<dyn Lettore>,
}

#[wasm_bindgen]
impl LettoreWasm {
    /// Opens the dataset made of `names[i]` (logical names, e.g. `AcqData/MSScan.bin`) of `sizes[i]` bytes; `read(i, offset, length)` gives the bytes.
    /// `err.file.notAccepted` when no reader recognises it. `anonymous`: the `metodo/1` leaves out (or pseudonymises) the people, the computer, the serial number and the paths.
    pub fn open(
        names: Vec<String>,
        sizes: Vec<f64>,
        read: Function,
        anonymous: bool,
    ) -> Result<LettoreWasm, JsValue> {
        let membri = names
            .into_iter()
            .zip(sizes)
            .enumerate()
            .map(|(i, (nome, dim))| Membro {
                nome,
                dim: dim as u64,
                dati: Box::new(Dal {
                    read: read.clone(),
                    membro: i as u32,
                }),
            })
            .collect();
        let mut inner =
            lettori::apri(FileSet { membri }, &Limiti::sito()).map_err(|e| error_json(&e))?;
        inner.imposta_anonimo(anonymous);
        Ok(LettoreWasm { inner })
    }

    /// The id of the format (`thermo-raw`, `sciex-wiff`, `agilent-d`).
    pub fn formato(&self) -> String {
        self.inner.formato().id().to_string()
    }

    /// `[{"id","nome","scansioni"}]`.
    pub fn campioni(&self) -> String {
        list(self.inner.campioni(), campione_json)
    }

    /// The index of sample `c`: `[{"id","rt","livello","polarita","tic","picco_base","filtro","precursore"}]`.
    pub fn indice(&mut self, c: usize) -> Result<String, JsValue> {
        Ok(list(
            self.inner.indice(c).map_err(|e| error_json(&e))?,
            voce_json,
        ))
    }

    /// Scan `k` of sample `c` as `[header JSON, m/z Float64Array, intensity Float64Array]`.
    pub fn scansione(&mut self, c: usize, k: usize) -> Result<Array, JsValue> {
        let s = self.inner.scansione(c, k).map_err(|e| error_json(&e))?;
        let mut o = Out::default();
        o.raw("{\"id\":").str(&s.id);
        o.raw(",\"rt\":").num(s.rt);
        o.raw(",\"livello\":").int(s.level as i64);
        o.raw(",\"polarita\":").int(s.polarity as i64);
        o.raw(",\"profilo\":")
            .raw(if s.profile { "true" } else { "false" });
        o.raw(",\"filtro\":").str(&s.filter);
        o.raw(",\"precursore\":").opt(s.precursor).raw("}");
        let inten: Vec<f64> = s.intensity.iter().map(|&x| x as f64).collect();
        Ok(Array::of3(
            &JsValue::from_str(&o.0),
            &Float64Array::from(&s.mz[..]),
            &Float64Array::from(&inten[..]),
        ))
    }
}

fn campo_json(o: &mut Out, f: &Campo) {
    o.raw("{\"nome\":").str(&f.nome);
    o.raw(",\"valore\":").str(&f.valore);
    for (k, v) in [("unita", &f.unita), ("cv", &f.cv)] {
        o.raw(&format!(",\"{k}\":"));
        match v {
            Some(x) => o.str(x),
            None => o.raw("null"),
        };
    }
    o.raw("}");
}

#[wasm_bindgen]
impl LettoreWasm {
    /// The header fields of scan `k`: `[{"nome","valore","unita","cv"}]`.
    pub fn intestazione(&mut self, c: usize, k: usize) -> Result<String, JsValue> {
        Ok(list(
            &self.inner.intestazione(c, k).map_err(|e| error_json(&e))?,
            campo_json,
        ))
    }

    /// Writes sample `c` as mzML: `sink(bytes)` gets the pieces in order, `progress(done, total)` counts the spectra.
    pub fn mzml(
        &mut self,
        c: usize,
        sink: Function,
        progress: Function,
    ) -> Result<(), JsValue> {
        let mut out = Pieces(sink, Vec::new());
        let mut tick = |done: usize, all: usize| {
            let _ = progress.call2(&JsValue::NULL, &done.into(), &all.into());
        };
        let mut w = lettori_mzml::scrivi(&mut *self.inner, c, &mut out, &mut tick)
            .map_err(|e| error_json(&e))?;
        std::io::Write::flush(&mut w).map_err(|_| JsValue::from_str("{\"error_key\":\"err.file.agilent\",\"params\":{}}"))
    }

    /// The `metodo/1` document of sample `c`.
    pub fn metodo(&mut self, c: usize) -> Result<String, JsValue> {
        self.inner.metodo(c).map_err(|e| error_json(&e))
    }

    /// The chromatograms of sample `c`: `[{"id","unita"}]`.
    pub fn cromatogrammi(&mut self, c: usize) -> Result<String, JsValue> {
        let v = self.inner.cromatogrammi(c).map_err(|e| error_json(&e))?;
        Ok(list(&v, |o, x| {
            o.raw("{\"id\":").str(&x.id);
            o.raw(",\"unita\":").str(&x.unita).raw("}");
        }))
    }

    /// The chromatogram `id` of sample `c` as `[time Float64Array (min), intensity Float64Array]`.
    pub fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Array, JsValue> {
        let x = self
            .inner
            .cromatogramma(c, id)
            .map_err(|e| error_json(&e))?;
        Ok(Array::of2(
            &Float64Array::from(&x.time[..]),
            &Float64Array::from(&x.intensity[..]),
        ))
    }
}
