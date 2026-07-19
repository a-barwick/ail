package store

import (
	"reflect"
	"testing"

	"ail.dev/job-service-baseline/v2/domain"
)

func pointer[T any](value T) *T { return &value }

func testJob(priority domain.Priority) domain.Job {
	return domain.Job{
		JobID:    "job-1",
		Task:     "task",
		Payload:  []byte{1, 2, 3},
		Priority: priority,
	}
}

func TestV1RecordAdaptsExplicitlyToNormal(t *testing.T) {
	stored := StoredJob{
		RecordVersion: RecordVersionV1,
		JobID:         "legacy",
		Task:          "task",
		Payload:       []byte{1},
	}
	job := stored.AdaptToV2()
	if job.Priority != domain.PriorityNormal {
		t.Fatalf("adapted priority = %q, want normal", job.Priority)
	}
}

func TestV2RecordPreservesPriority(t *testing.T) {
	stored := StoredJob{
		RecordVersion: RecordVersionV2,
		JobID:         "current",
		Task:          "task",
		Payload:       []byte{1},
		Priority:      pointer(domain.PriorityHigh),
	}
	if got := stored.AdaptToV2().Priority; got != domain.PriorityHigh {
		t.Fatalf("adapted priority = %q, want high", got)
	}
}

func TestInsertedJobIsRecordedAndPersistedExactlyOnce(t *testing.T) {
	store := New(nil, domain.Inserted, RecordVersionV2)
	job := testJob(domain.PriorityHigh)

	if got := store.InsertIfAbsent(job); got != domain.Inserted {
		t.Fatalf("InsertIfAbsent() = %d, want Inserted", got)
	}
	if calls := store.Calls(); len(calls) != 1 || !reflect.DeepEqual(calls[0].Job.AdaptToV2(), job) {
		t.Fatalf("calls = %#v, want one exact job", calls)
	}
	if jobs := store.Jobs(); len(jobs) != 1 || !reflect.DeepEqual(jobs[0].AdaptToV2(), job) {
		t.Fatalf("jobs = %#v, want one exact job", jobs)
	}
}

func TestV1InsertProjectionOmitsPriority(t *testing.T) {
	store := New(nil, domain.Inserted, RecordVersionV1)
	store.InsertIfAbsent(testJob(domain.PriorityHigh))
	stored := store.Jobs()[0]
	if stored.RecordVersion != RecordVersionV1 || stored.Priority != nil {
		t.Fatalf("V1 stored job = %#v, want version 1 without priority", stored)
	}
}

func TestDuplicateAndUnavailablePreserveInitialState(t *testing.T) {
	initial := StoredJob{
		RecordVersion: RecordVersionV2,
		JobID:         "existing",
		Task:          "old",
		Priority:      pointer(domain.PriorityLow),
	}
	for _, outcome := range []domain.InsertOutcome{
		domain.Duplicate, domain.UnavailableBeforeCommit,
	} {
		store := New([]StoredJob{initial}, outcome, RecordVersionV2)
		if got := store.InsertIfAbsent(testJob(domain.PriorityHigh)); got != outcome {
			t.Errorf("InsertIfAbsent() = %d, want %d", got, outcome)
		}
		if jobs := store.Jobs(); !reflect.DeepEqual(jobs, []StoredJob{initial}) {
			t.Errorf("outcome %d jobs = %#v, want unchanged initial state", outcome, jobs)
		}
		if len(store.Calls()) != 1 {
			t.Errorf("outcome %d made %d calls, want one", outcome, len(store.Calls()))
		}
	}
}

func TestReturnedStateAndCallsAreDefensiveCopies(t *testing.T) {
	store := New(nil, domain.Inserted, RecordVersionV2)
	store.InsertIfAbsent(testJob(domain.PriorityNormal))

	jobs := store.Jobs()
	calls := store.Calls()
	jobs[0].Payload[0] = 99
	*calls[0].Job.Priority = domain.PriorityHigh

	if store.Jobs()[0].Payload[0] != 1 {
		t.Fatal("mutating Jobs() result changed store state")
	}
	if *store.Calls()[0].Job.Priority != domain.PriorityNormal {
		t.Fatal("mutating Calls() result changed recorded call")
	}
}

func TestInsertedOutcomeRejectsDuplicateIdentifier(t *testing.T) {
	store := New([]StoredJob{{
		RecordVersion: RecordVersionV2,
		JobID:         "job-1",
		Priority:      pointer(domain.PriorityNormal),
	}}, domain.Inserted, RecordVersionV2)
	defer func() {
		if recover() == nil {
			t.Fatal("InsertIfAbsent() did not panic for violated inserted postcondition")
		}
	}()
	store.InsertIfAbsent(testJob(domain.PriorityNormal))
}
