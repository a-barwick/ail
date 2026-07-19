// Package v1 is the version-one checkpoint for the UC-001 implementation task.
package v1

import (
	"unicode"
	"unicode/utf8"
)

// CreateJobRequest is the decoded version-one public request.
type CreateJobRequest struct {
	JobID   string
	Task    string
	Payload []byte
}

// Job is the version-one record accepted by the jobs store.
type Job struct {
	JobID   string
	Task    string
	Payload []byte
}

type ValidationField string

const (
	FieldJobID   ValidationField = "job_id"
	FieldTask    ValidationField = "task"
	FieldPayload ValidationField = "payload"
)

type ValidationReason string

const (
	ReasonMissing          ValidationReason = "missing"
	ReasonInvalidFormat    ValidationReason = "invalid_format"
	ReasonTooLong          ValidationReason = "too_long"
	ReasonControlCharacter ValidationReason = "control_character"
	ReasonPayloadTooLarge  ValidationReason = "payload_too_large"
)

type ValidationIssue struct {
	Field  ValidationField
	Reason ValidationReason
}

// CreateJobResult is the closed public result contract for UC-001.
type CreateJobResult interface {
	isCreateJobResult()
}

type Created struct{ Job Job }
type Invalid struct{ Issues []ValidationIssue }
type AlreadyExists struct{ JobID string }
type PersistenceUnavailable struct{}

func (Created) isCreateJobResult()                {}
func (Invalid) isCreateJobResult()                {}
func (AlreadyExists) isCreateJobResult()          {}
func (PersistenceUnavailable) isCreateJobResult() {}

type InsertOutcome uint8

const (
	Inserted InsertOutcome = iota
	Duplicate
	UnavailableBeforeCommit
)

// JobStore is the only capability available to CreateJob.
type JobStore interface {
	InsertIfAbsent(Job) InsertOutcome
}

// CreateJob validates the complete request before making exactly one store call.
func CreateJob(request CreateJobRequest, store JobStore) CreateJobResult {
	issues := validate(request)
	if len(issues) != 0 {
		return Invalid{Issues: issues}
	}

	job := Job{
		JobID:   request.JobID,
		Task:    request.Task,
		Payload: request.Payload,
	}
	switch store.InsertIfAbsent(job) {
	case Inserted:
		return Created{Job: job}
	case Duplicate:
		return AlreadyExists{JobID: job.JobID}
	case UnavailableBeforeCommit:
		return PersistenceUnavailable{}
	default:
		panic("job store returned an invalid insert outcome")
	}
}

func validate(request CreateJobRequest) []ValidationIssue {
	issues := make([]ValidationIssue, 0, 3)

	switch {
	case request.JobID == "":
		issues = append(issues, ValidationIssue{Field: FieldJobID, Reason: ReasonMissing})
	case !validJobID(request.JobID):
		issues = append(issues, ValidationIssue{Field: FieldJobID, Reason: ReasonInvalidFormat})
	}

	switch {
	case request.Task == "":
		issues = append(issues, ValidationIssue{Field: FieldTask, Reason: ReasonMissing})
	case utf8.RuneCountInString(request.Task) > 80:
		issues = append(issues, ValidationIssue{Field: FieldTask, Reason: ReasonTooLong})
	case containsControl(request.Task):
		issues = append(issues, ValidationIssue{Field: FieldTask, Reason: ReasonControlCharacter})
	}

	if len(request.Payload) > 4096 {
		issues = append(issues, ValidationIssue{Field: FieldPayload, Reason: ReasonPayloadTooLarge})
	}
	return issues
}

func validJobID(jobID string) bool {
	if len(jobID) == 0 || len(jobID) > 64 || !isASCIIAlphanumeric(jobID[0]) {
		return false
	}
	for index := 1; index < len(jobID); index++ {
		value := jobID[index]
		if !isASCIIAlphanumeric(value) && value != '_' && value != '-' {
			return false
		}
	}
	return true
}

func isASCIIAlphanumeric(value byte) bool {
	return value >= 'a' && value <= 'z' ||
		value >= 'A' && value <= 'Z' ||
		value >= '0' && value <= '9'
}

func containsControl(value string) bool {
	for _, current := range value {
		if unicode.IsControl(current) {
			return true
		}
	}
	return false
}
