//! politicas round robin, lottery y tiempo real edf

use rand::Rng;
use crate::thread::{MyThread, ThreadId};
use crate::runtime::ThreadRuntime;

impl ThreadRuntime {
    pub(crate) fn schedule_roundrobin(&mut self) -> Option<ThreadId> {
        self.ready.pop_front()
    }

    pub(crate) fn schedule_lottery(&mut self) -> Option<ThreadId> {
        let ready_threads: Vec<&MyThread> = self
            .ready
            .iter()
            .filter_map(|tid| self.threads.get(tid))
            .collect();

        if ready_threads.is_empty() {
            return None;
        }

        let total_tickets: u32 = ready_threads.iter().map(|t| t.tickets).sum();
        if total_tickets == 0{
            return self.schedule_roundrobin();
        }
        let mut rng = rand::rng();
        let mut pick: u32 = rng.random_range(0..total_tickets);

        for t in &ready_threads {
            if pick < t.tickets {
                self.ready.retain(|&tid| tid != t.id);
                return Some(t.id);
            }
            pick -= t.tickets;
        }

        let t = ready_threads[0];
        self.ready.retain(|&tid| tid != t.id);
        Some(t.id)
    }

    pub(crate) fn schedule_realtime(&mut self) -> Option<ThreadId> {
        let now = self.now();

        let ready_threads: Vec<&MyThread> = self
            .ready
            .iter()
            .filter_map(|tid| self.threads.get(tid))
            .collect();

        if ready_threads.is_empty() {
            return None;
        }

        let candidate = ready_threads
            .iter()
            .filter_map(|t| t.deadline.map(|d| (t.id, d.saturating_sub(now))))
            .min_by_key(|&(_, remaining)| remaining)
            .map(|(id, _)| id);

        if let Some(tid) = candidate {
            self.ready.retain(|&id| id != tid);
            Some(tid)
        } else {
            self.schedule_roundrobin()
        }
    }
}

