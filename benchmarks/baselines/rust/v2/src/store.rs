use crate::domain::{InsertOutcome, Job, JobStore, Priority};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordVersion {
    V1,
    V2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoredJob {
    V1 {
        job_id: String,
        task: String,
        payload: Vec<u8>,
    },
    V2 {
        job_id: String,
        task: String,
        payload: Vec<u8>,
        priority: Priority,
    },
}

impl StoredJob {
    #[must_use]
    pub fn job_id(&self) -> &str {
        match self {
            Self::V1 { job_id, .. } | Self::V2 { job_id, .. } => job_id,
        }
    }

    #[must_use]
    pub fn adapt_to_v2(&self) -> Job {
        match self {
            Self::V1 {
                job_id,
                task,
                payload,
            } => Job {
                job_id: job_id.clone(),
                task: task.clone(),
                payload: payload.clone(),
                priority: Priority::Normal,
            },
            Self::V2 {
                job_id,
                task,
                payload,
                priority,
            } => Job {
                job_id: job_id.clone(),
                task: task.clone(),
                payload: payload.clone(),
                priority: *priority,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreCall {
    pub job: StoredJob,
}

#[derive(Debug)]
pub struct DeterministicJobStore {
    jobs: Vec<StoredJob>,
    outcome: InsertOutcome,
    insert_version: RecordVersion,
    calls: Vec<StoreCall>,
}

impl DeterministicJobStore {
    #[must_use]
    pub fn new(
        jobs: Vec<StoredJob>,
        outcome: InsertOutcome,
        insert_version: RecordVersion,
    ) -> Self {
        Self {
            jobs,
            outcome,
            insert_version,
            calls: Vec::new(),
        }
    }

    #[must_use]
    pub fn jobs(&self) -> &[StoredJob] {
        &self.jobs
    }

    #[must_use]
    pub fn calls(&self) -> &[StoreCall] {
        &self.calls
    }

    fn encode_for_insert(&self, job: &Job) -> StoredJob {
        match self.insert_version {
            RecordVersion::V1 => StoredJob::V1 {
                job_id: job.job_id.clone(),
                task: job.task.clone(),
                payload: job.payload.clone(),
            },
            RecordVersion::V2 => StoredJob::V2 {
                job_id: job.job_id.clone(),
                task: job.task.clone(),
                payload: job.payload.clone(),
                priority: job.priority,
            },
        }
    }
}

impl JobStore for DeterministicJobStore {
    fn insert_if_absent(&mut self, job: &Job) -> InsertOutcome {
        let stored = self.encode_for_insert(job);
        self.calls.push(StoreCall {
            job: stored.clone(),
        });
        if self.outcome == InsertOutcome::Inserted {
            assert!(
                self.jobs
                    .iter()
                    .all(|current| current.job_id() != job.job_id),
                "inserted outcome violates insert-if-absent postcondition"
            );
            self.jobs.push(stored);
        }
        self.outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(priority: Priority) -> Job {
        Job {
            job_id: "job-1".into(),
            task: "task".into(),
            payload: vec![1, 2, 3],
            priority,
        }
    }

    #[test]
    fn v1_records_adapt_explicitly_to_normal() {
        let record = StoredJob::V1 {
            job_id: "legacy".into(),
            task: "task".into(),
            payload: vec![1],
        };
        assert_eq!(record.adapt_to_v2().priority, Priority::Normal);
    }

    #[test]
    fn inserted_job_is_recorded_and_persisted_exactly_once() {
        let mut store =
            DeterministicJobStore::new(Vec::new(), InsertOutcome::Inserted, RecordVersion::V2);
        let job = job(Priority::High);

        assert_eq!(store.insert_if_absent(&job), InsertOutcome::Inserted);
        assert_eq!(store.calls().len(), 1);
        assert_eq!(store.jobs().len(), 1);
        assert_eq!(store.jobs()[0].adapt_to_v2(), job);
    }

    #[test]
    fn duplicate_and_unavailable_outcomes_preserve_initial_state() {
        let initial = StoredJob::V2 {
            job_id: "existing".into(),
            task: "old".into(),
            payload: Vec::new(),
            priority: Priority::Low,
        };
        for outcome in [
            InsertOutcome::Duplicate,
            InsertOutcome::UnavailableBeforeCommit,
        ] {
            let mut store =
                DeterministicJobStore::new(vec![initial.clone()], outcome, RecordVersion::V2);
            assert_eq!(store.insert_if_absent(&job(Priority::High)), outcome);
            assert_eq!(store.jobs(), &[initial.clone()]);
            assert_eq!(store.calls().len(), 1);
        }
    }
}
