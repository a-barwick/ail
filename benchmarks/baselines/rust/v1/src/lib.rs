//! Version-one checkpoint for the UC-001 implementation task.

/// A decoded version-one create-job request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateJobRequest {
    pub job_id: String,
    pub task: String,
    pub payload: Vec<u8>,
}

/// The version-one record accepted by the jobs store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub job_id: String,
    pub task: String,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationField {
    JobId,
    Task,
    Payload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationReason {
    Missing,
    InvalidFormat,
    TooLong,
    ControlCharacter,
    PayloadTooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidationIssue {
    pub field: ValidationField,
    pub reason: ValidationReason,
}

/// The closed public result contract for UC-001.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreateJobResult {
    Created(Job),
    Invalid(Vec<ValidationIssue>),
    AlreadyExists(String),
    PersistenceUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertOutcome {
    Inserted,
    Duplicate,
    UnavailableBeforeCommit,
}

/// The only capability available to the handler.
pub trait JobStore {
    fn insert_if_absent(&mut self, job: &Job) -> InsertOutcome;
}

/// Validates the complete request before making exactly one store call.
pub fn create_job(request: CreateJobRequest, store: &mut impl JobStore) -> CreateJobResult {
    let issues = validate(&request);
    if !issues.is_empty() {
        return CreateJobResult::Invalid(issues);
    }

    let job = Job {
        job_id: request.job_id,
        task: request.task,
        payload: request.payload,
    };
    match store.insert_if_absent(&job) {
        InsertOutcome::Inserted => CreateJobResult::Created(job),
        InsertOutcome::Duplicate => CreateJobResult::AlreadyExists(job.job_id),
        InsertOutcome::UnavailableBeforeCommit => CreateJobResult::PersistenceUnavailable,
    }
}

fn validate(request: &CreateJobRequest) -> Vec<ValidationIssue> {
    let mut issues = Vec::with_capacity(3);

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

    issues
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

    fn request() -> CreateJobRequest {
        CreateJobRequest {
            job_id: "job-1042".into(),
            task: "rebuild-search-index".into(),
            payload: br#"{"tenant":"north"}"#.to_vec(),
        }
    }

    fn store(outcome: InsertOutcome) -> RecordingStore {
        RecordingStore {
            outcome,
            calls: Vec::new(),
        }
    }

    #[test]
    fn valid_request_is_inserted_once_and_returned() {
        let request = request();
        let expected = Job {
            job_id: request.job_id.clone(),
            task: request.task.clone(),
            payload: request.payload.clone(),
        };
        let mut store = store(InsertOutcome::Inserted);

        assert_eq!(
            create_job(request, &mut store),
            CreateJobResult::Created(expected.clone())
        );
        assert_eq!(store.calls, vec![expected]);
    }

    #[test]
    fn validation_collects_one_issue_per_field_in_contract_order() {
        let mut store = store(InsertOutcome::Inserted);
        let result = create_job(
            CreateJobRequest {
                job_id: String::new(),
                task: String::new(),
                payload: vec![0; 4097],
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
    fn task_length_counts_unicode_scalar_values() {
        let mut at_limit = request();
        at_limit.task = "界".repeat(80);
        let mut at_limit_store = store(InsertOutcome::Inserted);
        assert!(matches!(
            create_job(at_limit, &mut at_limit_store),
            CreateJobResult::Created(_)
        ));

        let mut over_limit = request();
        over_limit.task = "界".repeat(81);
        let mut over_limit_store = store(InsertOutcome::Inserted);
        assert_eq!(
            create_job(over_limit, &mut over_limit_store),
            CreateJobResult::Invalid(vec![ValidationIssue {
                field: ValidationField::Task,
                reason: ValidationReason::TooLong,
            }])
        );
        assert!(over_limit_store.calls.is_empty());
    }

    #[test]
    fn control_characters_are_rejected_without_an_effect() {
        let mut invalid = request();
        invalid.task = "line\nbreak".into();
        let mut store = store(InsertOutcome::Inserted);

        assert_eq!(
            create_job(invalid, &mut store),
            CreateJobResult::Invalid(vec![ValidationIssue {
                field: ValidationField::Task,
                reason: ValidationReason::ControlCharacter,
            }])
        );
        assert!(store.calls.is_empty());
    }

    #[test]
    fn payload_bounds_are_measured_in_bytes() {
        for size in [0, 4096] {
            let mut valid = request();
            valid.payload = vec![0; size];
            let mut store = store(InsertOutcome::Inserted);
            assert!(matches!(
                create_job(valid, &mut store),
                CreateJobResult::Created(_)
            ));
        }

        let mut invalid = request();
        invalid.payload = vec![0; 4097];
        let mut store = store(InsertOutcome::Inserted);
        assert!(matches!(
            create_job(invalid, &mut store),
            CreateJobResult::Invalid(_)
        ));
        assert!(store.calls.is_empty());
    }

    #[test]
    fn store_outcomes_map_to_the_closed_public_results() {
        let cases = [
            (
                InsertOutcome::Duplicate,
                CreateJobResult::AlreadyExists("job-1042".into()),
            ),
            (
                InsertOutcome::UnavailableBeforeCommit,
                CreateJobResult::PersistenceUnavailable,
            ),
        ];

        for (outcome, expected) in cases {
            let mut store = store(outcome);
            assert_eq!(create_job(request(), &mut store), expected);
            assert_eq!(store.calls.len(), 1);
        }
    }
}
