//! Read side of the live scheduler: states and cards as the page shows them, and the `flowlive/1` event log with its writer.
//! A child of `sched` so that it sees the private state of `Sched`.

use super::events::{job_event, node_event};
use super::{Job, JobId, NodeView, Sched, State, LOG_CAP};

impl Sched {
    /// The state of a job as the page shows it (a ready job is `paused` while the scheduler is).
    pub fn state(&self, id: &JobId) -> Option<State> {
        self.at(id).map(|p| self.view(&self.jobs[p]))
    }

    pub(super) fn view(&self, j: &Job) -> State {
        match (&j.state, self.paused) {
            (State::Ready, true) => State::Paused,
            (s, _) => s.clone(),
        }
    }

    /// The `plan_key` of a job.
    pub fn key(&self, id: &JobId) -> Option<&str> {
        self.at(id).map(|p| self.jobs[p].node.key.as_str())
    }

    /// Every job with its state, in dependency order.
    pub fn jobs(&self) -> Vec<(JobId, State)> {
        self.jobs
            .iter()
            .map(|j| (j.node.id.clone(), self.view(j)))
            .collect()
    }

    /// The cards, one per instance of the plan in the order of the file.
    pub fn nodes(&self) -> Vec<NodeView> {
        self.plan
            .steps
            .iter()
            .filter_map(|inst| {
                let mine: Vec<&Job> = self
                    .jobs
                    .iter()
                    .filter(|j| j.node.id.instance == inst.id)
                    .collect();
                let views: Vec<State> = mine.iter().map(|j| self.view(j)).collect();
                let worst = views.iter().max_by_key(|s| s.rank())?.clone();
                let state = match worst {
                    State::Reused if views.contains(&State::Done) => State::Done,
                    s => s,
                };
                let fracs: Vec<Option<f64>> = views.iter().map(State::frac).collect();
                let frac = if fracs.iter().any(Option::is_none) {
                    None
                } else {
                    Some(fracs.iter().flatten().sum::<f64>() / fracs.len() as f64)
                };
                let parte = views
                    .iter()
                    .filter(|s| matches!(s, State::Done | State::Reused))
                    .count();
                let ms = mine
                    .iter()
                    .filter(|j| matches!(j.state, State::Done))
                    .map(|j| j.ms)
                    .sum();
                Some(NodeView {
                    instance: inst.id.clone(),
                    state,
                    frac,
                    parte,
                    di: views.len(),
                    ms,
                })
            })
            .collect()
    }

    pub(super) fn push<F: FnOnce(u64, u64) -> String>(&mut self, make: F) {
        self.seq += 1;
        self.log.push((self.seq, make(self.seq, self.rev)));
        if self.log.len() > LOG_CAP {
            self.log.remove(0);
        }
    }

    /// Writes an event for every job and card that differs from what was last written.
    pub(super) fn flush(&mut self) {
        for (id, key) in std::mem::take(&mut self.gone) {
            self.push(|s, r| job_event(s, r, &id, &State::Cancelled, None, &key));
        }
        for p in 0..self.jobs.len() {
            let state = self.view(&self.jobs[p]);
            let j = &self.jobs[p];
            let shown = (state.clone(), j.node.key.clone(), j.ms.to_bits());
            if j.shown.as_ref() == Some(&shown) {
                continue;
            }
            let (id, ms) = (
                j.node.id.clone(),
                matches!(state, State::Done).then_some(j.ms),
            );
            self.push(|s, r| job_event(s, r, &id, &state, ms, &shown.1));
            self.jobs[p].shown = Some(shown);
        }
        let nodes = self.nodes();
        for n in &nodes {
            if !self.shown_nodes.contains(n) {
                self.push(|s, r| node_event(s, r, n));
            }
        }
        self.shown_nodes = nodes;
    }

    /// The events with `seq` above the given one, as a JSON array of `flowlive/1` objects (`seq` grows by one, `rev` = number of `update` calls so far).
    pub fn events_since(&self, seq: u64) -> String {
        let texts: Vec<&str> = self
            .log
            .iter()
            .filter(|e| e.0 > seq)
            .map(|e| e.1.as_str())
            .collect();
        format!("[{}]", texts.join(","))
    }

    /// The same events, one text and its `seq` each.
    pub fn event_texts(&self, seq: u64) -> Vec<(u64, String)> {
        let after = self.log.iter().filter(|e| e.0 > seq);
        after.map(|e| (e.0, e.1.clone())).collect()
    }

    /// Number of `update` calls so far.
    pub fn rev(&self) -> u64 {
        self.rev
    }
}
