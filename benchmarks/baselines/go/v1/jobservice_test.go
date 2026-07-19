package v1

import (
	"bytes"
	"reflect"
	"strings"
	"testing"
)

type recordingStore struct {
	outcome InsertOutcome
	calls   []Job
}

func (store *recordingStore) InsertIfAbsent(job Job) InsertOutcome {
	job.Payload = bytes.Clone(job.Payload)
	store.calls = append(store.calls, job)
	return store.outcome
}

func validRequest() CreateJobRequest {
	return CreateJobRequest{
		JobID:   "job-1042",
		Task:    "rebuild-search-index",
		Payload: []byte(`{"tenant":"north"}`),
	}
}

func TestCreateJobInsertedOnce(t *testing.T) {
	request := validRequest()
	store := &recordingStore{outcome: Inserted}

	result := CreateJob(request, store)
	created, ok := result.(Created)
	if !ok {
		t.Fatalf("CreateJob() result type = %T, want Created", result)
	}
	want := Job{JobID: request.JobID, Task: request.Task, Payload: request.Payload}
	if !reflect.DeepEqual(created.Job, want) {
		t.Fatalf("created job = %#v, want %#v", created.Job, want)
	}
	if !reflect.DeepEqual(store.calls, []Job{want}) {
		t.Fatalf("store calls = %#v, want one exact call", store.calls)
	}
}

func TestCreateJobCollectsIssuesInContractOrderWithoutStoreAccess(t *testing.T) {
	store := &recordingStore{outcome: Inserted}
	result := CreateJob(CreateJobRequest{
		Task:    "",
		Payload: make([]byte, 4097),
	}, store)

	invalid, ok := result.(Invalid)
	if !ok {
		t.Fatalf("CreateJob() result type = %T, want Invalid", result)
	}
	want := []ValidationIssue{
		{Field: FieldJobID, Reason: ReasonMissing},
		{Field: FieldTask, Reason: ReasonMissing},
		{Field: FieldPayload, Reason: ReasonPayloadTooLarge},
	}
	if !reflect.DeepEqual(invalid.Issues, want) {
		t.Fatalf("issues = %#v, want %#v", invalid.Issues, want)
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

func TestTaskUsesUnicodeScalarLengthAndRejectsControls(t *testing.T) {
	tests := []struct {
		name       string
		task       string
		wantReason ValidationReason
	}{
		{name: "at scalar limit", task: strings.Repeat("界", 80)},
		{name: "over scalar limit", task: strings.Repeat("界", 81), wantReason: ReasonTooLong},
		{name: "control", task: "line\nbreak", wantReason: ReasonControlCharacter},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			request := validRequest()
			request.Task = test.task
			store := &recordingStore{outcome: Inserted}
			result := CreateJob(request, store)
			if test.wantReason == "" {
				if _, ok := result.(Created); !ok {
					t.Fatalf("result type = %T, want Created", result)
				}
				return
			}
			want := Invalid{Issues: []ValidationIssue{{Field: FieldTask, Reason: test.wantReason}}}
			if !reflect.DeepEqual(result, want) {
				t.Fatalf("result = %#v, want %#v", result, want)
			}
			if len(store.calls) != 0 {
				t.Fatalf("invalid task made %d store calls, want zero", len(store.calls))
			}
		})
	}
}

func TestPayloadBoundsAreBytes(t *testing.T) {
	for _, size := range []int{0, 4096, 4097} {
		t.Run(string(rune(size)), func(t *testing.T) {
			request := validRequest()
			request.Payload = make([]byte, size)
			store := &recordingStore{outcome: Inserted}
			result := CreateJob(request, store)
			if size <= 4096 {
				if _, ok := result.(Created); !ok {
					t.Fatalf("size %d result type = %T, want Created", size, result)
				}
				return
			}
			if !reflect.DeepEqual(result, Invalid{Issues: []ValidationIssue{{
				Field: FieldPayload, Reason: ReasonPayloadTooLarge,
			}}}) {
				t.Fatalf("size %d result = %#v, want payload-too-large", size, result)
			}
			if len(store.calls) != 0 {
				t.Fatalf("oversized payload made %d store calls, want zero", len(store.calls))
			}
		})
	}
}

func TestStoreOutcomesAreClosedAndFollowOneCall(t *testing.T) {
	tests := []struct {
		outcome InsertOutcome
		want    CreateJobResult
	}{
		{outcome: Duplicate, want: AlreadyExists{JobID: "job-1042"}},
		{outcome: UnavailableBeforeCommit, want: PersistenceUnavailable{}},
	}
	for _, test := range tests {
		store := &recordingStore{outcome: test.outcome}
		if got := CreateJob(validRequest(), store); !reflect.DeepEqual(got, test.want) {
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
	CreateJob(validRequest(), &recordingStore{outcome: 99})
}
