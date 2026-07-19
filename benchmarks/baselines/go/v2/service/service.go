// Package service implements the transport-independent create-job handler.
package service

import (
	"unicode"
	"unicode/utf8"

	"ail.dev/job-service-baseline/v2/domain"
)

// CreateJob implements UC-001 with the UC-003 priority contract.
//
// Validation completes before the only capability call. Priority becomes
// non-optional only after validation succeeds.
func CreateJob(request domain.CreateJobRequest, store domain.JobStore) domain.CreateJobResult {
	priority, issues := validate(request)
	if len(issues) != 0 {
		return domain.Invalid{Issues: issues}
	}

	job := domain.Job{
		JobID:    request.JobID,
		Task:     request.Task,
		Payload:  request.Payload,
		Priority: priority,
	}
	switch store.InsertIfAbsent(job) {
	case domain.Inserted:
		return domain.Created{Job: job}
	case domain.Duplicate:
		return domain.AlreadyExists{JobID: job.JobID}
	case domain.UnavailableBeforeCommit:
		return domain.PersistenceUnavailable{}
	default:
		panic("job store returned an invalid insert outcome")
	}
}

func validate(request domain.CreateJobRequest) (domain.Priority, []domain.ValidationIssue) {
	issues := make([]domain.ValidationIssue, 0, 4)

	switch {
	case request.JobID == "":
		issues = append(issues, domain.ValidationIssue{
			Field: domain.FieldJobID, Reason: domain.ReasonMissing,
		})
	case !validJobID(request.JobID):
		issues = append(issues, domain.ValidationIssue{
			Field: domain.FieldJobID, Reason: domain.ReasonInvalidFormat,
		})
	}

	switch {
	case request.Task == "":
		issues = append(issues, domain.ValidationIssue{
			Field: domain.FieldTask, Reason: domain.ReasonMissing,
		})
	case utf8.RuneCountInString(request.Task) > 80:
		issues = append(issues, domain.ValidationIssue{
			Field: domain.FieldTask, Reason: domain.ReasonTooLong,
		})
	case containsControl(request.Task):
		issues = append(issues, domain.ValidationIssue{
			Field: domain.FieldTask, Reason: domain.ReasonControlCharacter,
		})
	}

	if len(request.Payload) > 4096 {
		issues = append(issues, domain.ValidationIssue{
			Field: domain.FieldPayload, Reason: domain.ReasonPayloadTooLarge,
		})
	}

	priority := domain.Priority("")
	if request.Priority == nil {
		issues = append(issues, domain.ValidationIssue{
			Field: domain.FieldPriority, Reason: domain.ReasonMissing,
		})
	} else {
		priority = *request.Priority
	}
	return priority, issues
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
