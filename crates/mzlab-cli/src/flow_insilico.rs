//! The native executor of `spectra.insilico@1`: each query MS2 spectrum against the candidate structures of a local CSV list, ranked by the peaks their
//! bond-wise fragments explain (`ms2_engine::insilico`). The `candidates` file is JSON, like `spectra.match`; its hits are `source: insilico` hypotheses.

use crate::flow::{read_one, write_made, FileRunner};
use crate::flow_match::{num, query_id, read_spectra, Named};
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::{round, Out};
use mzlab_core::step::DataType;
use mzlab_feat::attributio::ms2_engine::insilico::{rank, read_candidates, InSilicoOpts};

/// Runs the step of `job` on the query spectra and the suspect list (CSV) and returns the text of the `candidates` file: queries in file order, each
/// query's candidates best score first, at most `top` of them.
pub fn insilico_text(job: &Job, spectra: Named, suspects: &[u8]) -> Result<String> {
    let list = read_candidates(&String::from_utf8_lossy(suspects));
    if list.is_empty() {
        return Err(Error::new("err.flow.insilico.suspects"));
    }
    let terms = job
        .resolved
        .params
        .iter()
        .any(|(k, v)| k == "score_terms" && v.bool() == Some(true));
    let opts = InSilicoOpts {
        db_ppm: num(job, "db_ppm"),
        frag_ppm: num(job, "frag_ppm"),
        max_bonds: num(job, "max_bonds").max(1.0) as usize,
        top: num(job, "top").max(1.0) as usize,
        terms,
    };
    let mut o = Out::default();
    o.raw("{\"format\":\"candidates/1\",\"step\":\"spectra.insilico@1\",\"algorithm\":\"fragmenter\",\"hits\":[");
    let mut first = true;
    for (k, s) in read_spectra(spectra)?.iter().enumerate() {
        if s.prec <= 0.0 || s.mz.is_empty() {
            continue;
        }
        let qid = query_id(k, s);
        let pol = if s.pol < 0 { "NEG" } else { "POS" };
        let peaks: Vec<(f64, f64)> = s.mz.iter().copied().zip(s.it.iter().copied()).collect();
        for h in rank(s.prec, pol, &peaks, &list, &opts) {
            if !first {
                o.raw(",");
            }
            first = false;
            let c = &h.candidate;
            o.raw("{\"query\":").str(&qid);
            o.raw(",\"id\":").str(&c.id);
            o.raw(",\"name\":").str(&c.name);
            o.raw(",\"inchikey\":\"\",\"smiles\":").str(&c.smiles);
            o.raw(",\"formula\":").str(&c.formula);
            o.raw(",\"score\":").num(round(h.score, 6));
            if let Some(t) = &h.terms {
                o.raw(",\"score_terms\":");
                t.write(&mut o);
            }
            o.raw(",\"shared\":").int(h.explained.len() as i64);
            o.raw(",\"explained\":").rounded(&h.explained, 4);
            o.raw(",\"mass_ppm\":").num(round(h.mass_ppm, 3));
            o.raw(",\"precursor_delta\":0,\"source\":\"insilico\",\"algorithm\":\"fragmenter\"}");
        }
    }
    o.raw("]}");
    Ok(o.0)
}

pub(crate) fn insilico_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (spectra, suspects) = (read_one(r, job, "spectra")?, read_one(r, job, "suspects")?);
    let text = insilico_text(job, (&spectra.0, &spectra.1), &suspects.1)?;
    write_made(r, job, text, DataType::Candidates)
}
