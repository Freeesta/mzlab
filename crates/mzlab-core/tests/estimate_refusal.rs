//! A load that does not fit in the budget ends in order: `error.memoryBudget` with the phase, the estimate and the budget, before the rows are kept;
//! a budget that fits gives the same table as the load without a budget.
use mzlab_core::budget::MemBudget;
use mzlab_core::scantable;

fn mzml(scans: usize) -> Vec<u8> {
    let mut s = String::from("<?xml version=\"1.0\"?>\n<mzML><run id=\"r\"><spectrumList count=\"");
    s += &format!("{scans}\">\n");
    for i in 0..scans {
        s += &format!(
            "<spectrum index=\"{i}\" id=\"scan={}\" defaultArrayLength=\"0\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"1\"/>\
<cvParam cvRef=\"MS\" accession=\"MS:1000130\" name=\"positive scan\" value=\"\"/><scanList count=\"1\"><scan>\
<cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"minute\"/></scan></scanList></spectrum>\n",
            i + 1,
            i as f64 / 100.0
        );
    }
    s += "</spectrumList></run></mzML>\n";
    s.into_bytes()
}

#[test]
fn a_budget_that_does_not_fit_refuses_in_order_with_the_numbers() {
    // 8200 scans: the walk looks at the multiples of 4096, so the second look sees a table of 8192 scans (about 13 MB)
    let b = mzml(8200);
    let err =
        scantable::read_in(&mut b.clone(), b.len() as u64, Some(&MemBudget::mb(4.0))).unwrap_err();
    assert_eq!(err.key, "error.memoryBudget");
    let get = |k: &str| {
        err.params
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(get("fase").as_deref(), Some("table"));
    assert_eq!(get("budget_mb").as_deref(), Some("4"));
    assert!(get("stima_mb").unwrap().parse::<usize>().unwrap() > 4);
}

#[test]
fn a_budget_that_fits_gives_the_same_table_as_no_budget() {
    let b = mzml(300);
    let size = b.len() as u64;
    let free = scantable::read(&mut b.clone(), size).unwrap();
    let fits = scantable::read_in(&mut b.clone(), size, Some(&MemBudget::mb(192.0))).unwrap();
    assert_eq!(free.rows.len(), 300);
    assert_eq!(free.rows_json(), fits.rows_json());
}
