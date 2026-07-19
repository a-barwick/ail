// Package store provides the deterministic jobs capability used by the benchmark.
package store

import (
	"bytes"
	"fmt"

	"ail.dev/job-service-baseline/v2/domain"
)

type RecordVersion uint8

const (
	RecordVersionV1 RecordVersion = 1
	RecordVersionV2 RecordVersion = 2
)

// StoredJob preserves the declared persisted schema version.
type StoredJob struct {
	RecordVersion RecordVersion
	JobID         string
	Task          string
	Payload       []byte
	Priority      *domain.Priority
}

// AdaptToV2 explicitly maps a V1 persisted record to normal priority.
func (stored StoredJob) AdaptToV2() domain.Job {
	priority := domain.PriorityNormal
	if stored.RecordVersion == RecordVersionV2 && stored.Priority != nil {
		priority = *stored.Priority
	}
	return domain.Job{
		JobID:    stored.JobID,
		Task:     stored.Task,
		Payload:  bytes.Clone(stored.Payload),
		Priority: priority,
	}
}

type StoreCall struct {
	Job StoredJob
}

// DeterministicJobStore records every call and applies a supplied outcome.
type DeterministicJobStore struct {
	jobs          []StoredJob
	outcome       domain.InsertOutcome
	insertVersion RecordVersion
	calls         []StoreCall
}

func New(
	jobs []StoredJob,
	outcome domain.InsertOutcome,
	insertVersion RecordVersion,
) *DeterministicJobStore {
	return &DeterministicJobStore{
		jobs:          cloneStoredJobs(jobs),
		outcome:       outcome,
		insertVersion: insertVersion,
	}
}

func (store *DeterministicJobStore) Jobs() []StoredJob {
	return cloneStoredJobs(store.jobs)
}

func (store *DeterministicJobStore) Calls() []StoreCall {
	result := make([]StoreCall, len(store.calls))
	for index, call := range store.calls {
		result[index] = StoreCall{Job: cloneStoredJob(call.Job)}
	}
	return result
}

func (store *DeterministicJobStore) InsertIfAbsent(job domain.Job) domain.InsertOutcome {
	stored := store.encodeForInsert(job)
	store.calls = append(store.calls, StoreCall{Job: cloneStoredJob(stored)})
	if store.outcome == domain.Inserted {
		for _, current := range store.jobs {
			if current.JobID == job.JobID {
				panic(fmt.Sprintf(
					"inserted outcome violates insert-if-absent postcondition for %q",
					job.JobID,
				))
			}
		}
		store.jobs = append(store.jobs, stored)
	}
	return store.outcome
}

func (store *DeterministicJobStore) encodeForInsert(job domain.Job) StoredJob {
	stored := StoredJob{
		RecordVersion: store.insertVersion,
		JobID:         job.JobID,
		Task:          job.Task,
		Payload:       bytes.Clone(job.Payload),
	}
	if store.insertVersion == RecordVersionV2 {
		priority := job.Priority
		stored.Priority = &priority
	}
	return stored
}

func cloneStoredJobs(jobs []StoredJob) []StoredJob {
	result := make([]StoredJob, len(jobs))
	for index, job := range jobs {
		result[index] = cloneStoredJob(job)
	}
	return result
}

func cloneStoredJob(job StoredJob) StoredJob {
	job.Payload = bytes.Clone(job.Payload)
	if job.Priority != nil {
		priority := *job.Priority
		job.Priority = &priority
	}
	return job
}
