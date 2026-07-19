// Package domain defines the public contracts for the evolved job service.
package domain

type APIVersion uint8

const (
	APIVersionV1 APIVersion = 1
	APIVersionV2 APIVersion = 2
)

type Priority string

const (
	PriorityLow    Priority = "low"
	PriorityNormal Priority = "normal"
	PriorityHigh   Priority = "high"
)

// ParsePriority recognizes the complete closed priority contract.
func ParsePriority(value string) (Priority, bool) {
	switch Priority(value) {
	case PriorityLow, PriorityNormal, PriorityHigh:
		return Priority(value), true
	default:
		return "", false
	}
}

type CreateJobRequest struct {
	JobID    string
	Task     string
	Payload  []byte
	Priority *Priority
}

type Job struct {
	JobID    string
	Task     string
	Payload  []byte
	Priority Priority
}

type ValidationField string

const (
	FieldJobID    ValidationField = "job_id"
	FieldTask     ValidationField = "task"
	FieldPayload  ValidationField = "payload"
	FieldPriority ValidationField = "priority"
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

// CreateJobResult is the closed public result contract for UC-001 and UC-003.
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

// JobStore is the only external capability available to the handler.
type JobStore interface {
	InsertIfAbsent(Job) InsertOutcome
}
