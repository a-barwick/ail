use crate::domain::{
    CreateJobRequest, CreateJobResult, InsertOutcome, Job, JobStore, ValidationField,
    ValidationIssue, ValidationReason,
};

/// Implements the UC-001 handler with the UC-003 priority contract.
///
/// Validation is completed before the only capability call. The priority
/// becomes non-optional only after validation succeeds.
pub fn create_job(request: CreateJobRequest, store: &mut impl JobStore) -> CreateJobResult {
    let priority = match validate(&request) {
        Ok(priority) => priority,
        Err(issues) => return CreateJobResult::Invalid(issues),
    };
    let job = Job {
        job_id: request.job_id,
        task: request.task,
        payload: request.payload,
        priority,
    };
    match store.insert_if_absent(&job) {
        InsertOutcome::Inserted => CreateJobResult::Created(job),
        InsertOutcome::Duplicate => CreateJobResult::AlreadyExists(job.job_id),
        InsertOutcome::UnavailableBeforeCommit => CreateJobResult::PersistenceUnavailable,
    }
}

fn validate(request: &CreateJobRequest) -> Result<crate::domain::Priority, Vec<ValidationIssue>> {
    let mut issues = Vec::with_capacity(4);

    if request.job_id.is_empty() {
        issues.push(ValidationIssue {
            field: ValidationField::JobId,
            reason: ValidationReason::Missing,
        });
    } else if !valid_job_id(&request.job_id) {
        issues.push(ValidationIssue {
            field: ValidationField::JobId,
            reason: ValidationReason::InvalidFormat,
        });
    }

    if request.task.is_empty() {
        issues.push(ValidationIssue {
            field: ValidationField::Task,
            reason: ValidationReason::Missing,
        });
    } else if request.task.chars().count() > 80 {
        issues.push(ValidationIssue {
            field: ValidationField::Task,
            reason: ValidationReason::TooLong,
        });
    } else if request.task.chars().any(char::is_control) {
        issues.push(ValidationIssue {
            field: ValidationField::Task,
            reason: ValidationReason::ControlCharacter,
        });
    }

    if request.payload.len() > 4096 {
        issues.push(ValidationIssue {
            field: ValidationField::Payload,
            reason: ValidationReason::PayloadTooLarge,
        });
    }

    if request.priority.is_none() {
        issues.push(ValidationIssue {
            field: ValidationField::Priority,
            reason: ValidationReason::Missing,
        });
    }

    match request.priority {
        Some(priority) if issues.is_empty() => Ok(priority),
        Some(_) | None => Err(issues),
    }
}

fn valid_job_id(job_id: &str) -> bool {
    let bytes = job_id.as_bytes();
    (1..=64).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Priority;

    #[derive(Debug)]
    struct RecordingStore {
        outcome: InsertOutcome,
        calls: Vec<Job>,
    }

    impl JobStore for RecordingStore {
        fn insert_if_absent(&mut self, job: &Job) -> InsertOutcome {
            self.calls.push(job.clone());
            self.outcome
        }
    }

    fn request(priority: Option<Priority>) -> CreateJobRequest {
        CreateJobRequest {
            job_id: "job-1042".into(),
            task: "rebuild-search-index".into(),
            payload: br#"{"tenant":"north"}"#.to_vec(),
            priority,
        }
    }

    fn store(outcome: InsertOutcome) -> RecordingStore {
        RecordingStore {
            outcome,
            calls: Vec::new(),
        }
    }

    #[test]
    fn every_priority_is_propagated_unchanged() {
        for priority in [Priority::Low, Priority::Normal, Priority::High] {
            let mut store = store(InsertOutcome::Inserted);
            let result = create_job(request(Some(priority)), &mut store);
            let CreateJobResult::Created(job) = result else {
                panic!("expected created result");
            };
            assert_eq!(job.priority, priority);
            assert_eq!(store.calls, vec![job]);
        }
    }

    #[test]
    fn all_issues_are_reported_in_field_order_without_store_access() {
        let mut store = store(InsertOutcome::Inserted);
        let result = create_job(
            CreateJobRequest {
                job_id: String::new(),
                task: String::new(),
                payload: vec![0; 4097],
                priority: None,
            },
            &mut store,
        );

        assert_eq!(
            result,
            CreateJobResult::Invalid(vec![
                ValidationIssue {
                    field: ValidationField::JobId,
                    reason: ValidationReason::Missing,
                },
                ValidationIssue {
                    field: ValidationField::Task,
                    reason: ValidationReason::Missing,
                },
                ValidationIssue {
                    field: ValidationField::Payload,
                    reason: ValidationReason::PayloadTooLarge,
                },
                ValidationIssue {
                    field: ValidationField::Priority,
                    reason: ValidationReason::Missing,
                },
            ])
        );
        assert!(store.calls.is_empty());
    }

    #[test]
    fn job_id_boundaries_and_ascii_format_are_enforced() {
        for valid in ["a", "A0", &format!("a{}", "_".repeat(63))] {
            assert!(valid_job_id(valid), "{valid:?} should be valid");
        }
        for invalid in [
            "",
            "-starts-wrong",
            "_starts_wrong",
            "contains space",
            "é",
            &format!("a{}", "x".repeat(64)),
        ] {
            assert!(!valid_job_id(invalid), "{invalid:?} should be invalid");
        }
    }

    #[test]
    fn task_uses_unicode_scalar_length_and_rejects_controls() {
        let mut at_limit = request(Some(Priority::Normal));
        at_limit.task = "界".repeat(80);
        let mut at_limit_store = store(InsertOutcome::Inserted);
        assert!(matches!(
            create_job(at_limit, &mut at_limit_store),
            CreateJobResult::Created(_)
        ));

        for (task, reason) in [
            ("界".repeat(81), ValidationReason::TooLong),
            ("line\nbreak".into(), ValidationReason::ControlCharacter),
        ] {
            let mut invalid = request(Some(Priority::Normal));
            invalid.task = task;
            let mut invalid_store = store(InsertOutcome::Inserted);
            assert_eq!(
                create_job(invalid, &mut invalid_store),
                CreateJobResult::Invalid(vec![ValidationIssue {
                    field: ValidationField::Task,
                    reason,
                }])
            );
            assert!(invalid_store.calls.is_empty());
        }
    }

    #[test]
    fn payload_bounds_are_bytes_and_invalid_payload_has_no_effect() {
        for size in [0, 4096] {
            let mut valid = request(Some(Priority::Normal));
            valid.payload = vec![0; size];
            let mut store = store(InsertOutcome::Inserted);
            assert!(matches!(
                create_job(valid, &mut store),
                CreateJobResult::Created(_)
            ));
        }

        let mut invalid = request(Some(Priority::Normal));
        invalid.payload = vec![0; 4097];
        let mut store = store(InsertOutcome::Inserted);
        assert!(matches!(
            create_job(invalid, &mut store),
            CreateJobResult::Invalid(_)
        ));
        assert!(store.calls.is_empty());
    }

    #[test]
    fn store_outcomes_are_closed_and_each_follows_one_call() {
        for (outcome, expected) in [
            (
                InsertOutcome::Duplicate,
                CreateJobResult::AlreadyExists("job-1042".into()),
            ),
            (
                InsertOutcome::UnavailableBeforeCommit,
                CreateJobResult::PersistenceUnavailable,
            ),
        ] {
            let mut store = store(outcome);
            assert_eq!(
                create_job(request(Some(Priority::Normal)), &mut store),
                expected
            );
            assert_eq!(store.calls.len(), 1);
        }
    }
}
