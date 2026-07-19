package service

import (
	"reflect"
	"strings"
	"testing"

	"ail.dev/job-service-baseline/v2/domain"
)

type recordingStore struct {
	outcome domain.InsertOutcome
	calls   []domain.Job
}

func (store *recordingStore) InsertIfAbsent(job domain.Job) domain.InsertOutcome {
	store.calls = append(store.calls, job)
	return store.outcome
}

func pointer[T any](value T) *T { return &value }

func validRequest(priority domain.Priority) domain.CreateJobRequest {
	return domain.CreateJobRequest{
		JobID:    "job-1042",
		Task:     "rebuild-search-index",
		Payload:  []byte(`{"tenant":"north"}`),
		Priority: pointer(priority),
	}
}

func TestEveryPriorityIsPropagatedUnchanged(t *testing.T) {
	for _, priority := range []domain.Priority{
		domain.PriorityLow, domain.PriorityNormal, domain.PriorityHigh,
	} {
		t.Run(string(priority), func(t *testing.T) {
			store := &recordingStore{outcome: domain.Inserted}
			result := CreateJob(validRequest(priority), store)
			created, ok := result.(domain.Created)
			if !ok {
				t.Fatalf("result type = %T, want domain.Created", result)
			}
			if created.Job.Priority != priority {
				t.Fatalf("created priority = %q, want %q", created.Job.Priority, priority)
			}
			if !reflect.DeepEqual(store.calls, []domain.Job{created.Job}) {
				t.Fatalf("store calls = %#v, want exact created job", store.calls)
			}
		})
	}
}

func TestAllIssuesAreOrderedAndEffectFree(t *testing.T) {
	store := &recordingStore{outcome: domain.Inserted}
	result := CreateJob(domain.CreateJobRequest{
		Payload: make([]byte, 4097),
	}, store)

	want := domain.Invalid{Issues: []domain.ValidationIssue{
		{Field: domain.FieldJobID, Reason: domain.ReasonMissing},
		{Field: domain.FieldTask, Reason: domain.ReasonMissing},
		{Field: domain.FieldPayload, Reason: domain.ReasonPayloadTooLarge},
		{Field: domain.FieldPriority, Reason: domain.ReasonMissing},
	}}
	if !reflect.DeepEqual(result, want) {
		t.Fatalf("result = %#v, want %#v", result, want)
	}
	if len(store.calls) != 0 {
		t.Fatalf("invalid request made %d store calls, want zero", len(store.calls))
	}
}

func TestJobIDBoundariesAndASCIIFormat(t *testing.T) {
	for _, value := range []string{"a", "A0", "a" + strings.Repeat("_", 63)} {
		if !validJobID(value) {
			t.Errorf("validJobID(%q) = false, want true", value)
		}
	}
	for _, value := range []string{
		"", "-starts-wrong", "_starts_wrong", "contains space", "é",
		"a" + strings.Repeat("x", 64),
	} {
		if validJobID(value) {
			t.Errorf("validJobID(%q) = true, want false", value)
		}
	}
}

func TestTaskAndPayloadBounds(t *testing.T) {
	tests := []struct {
		name       string
		task       string
		payloadLen int
		wantField  domain.ValidationField
		wantReason domain.ValidationReason
	}{
		{name: "empty payload", task: "task"},
		{name: "max payload", task: "task", payloadLen: 4096},
		{name: "max unicode task", task: strings.Repeat("界", 80)},
		{name: "task too long", task: strings.Repeat("界", 81), wantField: domain.FieldTask, wantReason: domain.ReasonTooLong},
		{name: "task control", task: "line\nbreak", wantField: domain.FieldTask, wantReason: domain.ReasonControlCharacter},
		{name: "payload too large", task: "task", payloadLen: 4097, wantField: domain.FieldPayload, wantReason: domain.ReasonPayloadTooLarge},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			request := validRequest(domain.PriorityNormal)
			request.Task = test.task
			request.Payload = make([]byte, test.payloadLen)
			store := &recordingStore{outcome: domain.Inserted}
			result := CreateJob(request, store)
			if test.wantReason == "" {
				if _, ok := result.(domain.Created); !ok {
					t.Fatalf("result type = %T, want Created", result)
				}
				return
			}
			want := domain.Invalid{Issues: []domain.ValidationIssue{{
				Field: test.wantField, Reason: test.wantReason,
			}}}
			if !reflect.DeepEqual(result, want) {
				t.Fatalf("result = %#v, want %#v", result, want)
			}
			if len(store.calls) != 0 {
				t.Fatalf("invalid request made %d store calls, want zero", len(store.calls))
			}
		})
	}
}

func TestStoreOutcomesAreClosedAndFollowOneCall(t *testing.T) {
	tests := []struct {
		outcome domain.InsertOutcome
		want    domain.CreateJobResult
	}{
		{outcome: domain.Duplicate, want: domain.AlreadyExists{JobID: "job-1042"}},
		{outcome: domain.UnavailableBeforeCommit, want: domain.PersistenceUnavailable{}},
	}
	for _, test := range tests {
		store := &recordingStore{outcome: test.outcome}
		if got := CreateJob(validRequest(domain.PriorityNormal), store); !reflect.DeepEqual(got, test.want) {
			t.Errorf("outcome %d result = %#v, want %#v", test.outcome, got, test.want)
		}
		if len(store.calls) != 1 {
			t.Errorf("outcome %d made %d calls, want one", test.outcome, len(store.calls))
		}
	}
}

func TestInvalidStoreOutcomeIsAContractFault(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("CreateJob() did not panic for an invalid store outcome")
		}
	}()
	CreateJob(validRequest(domain.PriorityNormal), &recordingStore{outcome: 99})
}
