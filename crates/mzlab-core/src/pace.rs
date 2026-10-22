//! Throttle for the `(done, total)` progress callbacks of the long jobs (centroids, index, detection): at most one call per 1 %, never going back, the last one
//! is `(total, total)`. Used by the native Flow executor and the feature steps; it only forwards, so the bytes a job makes do not depend on it.

/// Forwards `(done, total)` to `sink` when the whole percent grew since the last call it forwarded.
pub struct Pace<'a> {
    sink: &'a mut dyn FnMut(usize, usize),
    pct: i64,
}

impl<'a> Pace<'a> {
    pub fn new(sink: &'a mut dyn FnMut(usize, usize)) -> Self {
        Pace { sink, pct: -1 }
    }

    /// One report from the job; a `done` past `total` counts as `total`, an empty job as finished.
    pub fn at(&mut self, done: usize, total: usize) {
        let done = done.min(total);
        let pct = if total == 0 {
            100
        } else {
            (done as u128 * 100 / total as u128) as i64
        };
        if pct > self.pct {
            self.pct = pct;
            (self.sink)(done, total);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Pace;

    #[test]
    fn at_most_a_call_per_percent_monotone_and_ends_at_the_total() {
        let mut seen = Vec::new();
        {
            let mut sink = |d, t| seen.push((d, t));
            let mut p = Pace::new(&mut sink);
            for d in (0..=5000).chain([200, 5000, 6000]) {
                p.at(d, 5000);
            }
        }
        assert!(seen.len() <= 101, "{}", seen.len());
        assert!(seen.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(seen.first(), Some(&(0, 5000)));
        assert_eq!(seen.last(), Some(&(5000, 5000)));
    }

    #[test]
    fn an_empty_job_reports_once_as_finished() {
        let mut seen = Vec::new();
        {
            let mut sink = |d, t| seen.push((d, t));
            let mut p = Pace::new(&mut sink);
            p.at(0, 0);
            p.at(0, 0);
        }
        assert_eq!(seen, vec![(0, 0)]);
    }
}
