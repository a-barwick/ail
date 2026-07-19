#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiVersion {
    V1,
    V2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    Low,
    Normal,
    High,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateJobRequest {
    pub job_id: String,
    pub task: String,
    pub payload: Vec<u8>,
    pub priority: Option<Priority>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub job_id: String,
    pub task: String,
    pub payload: Vec<u8>,
    pub priority: Priority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationField {
    JobId,
    Task,
    Payload,
    Priority,
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

pub trait JobStore {
    fn insert_if_absent(&mut self, job: &Job) -> InsertOutcome;
}
