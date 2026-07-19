package fixture

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"ail.dev/job-service-baseline/v2/domain"
)

type fixtureManifest struct {
	Fixtures []struct {
		Path string `json:"path"`
	} `json:"fixtures"`
}

func repositoryRoot(t *testing.T) string {
	t.Helper()
	current, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	for {
		if _, err := os.Stat(filepath.Join(current, ".git")); err == nil {
			return current
		}
		parent := filepath.Dir(current)
		if parent == current {
			t.Fatal("test package is not inside the repository")
		}
		current = parent
	}
}

func normalizeJSON(t *testing.T, value any) any {
	t.Helper()
	data, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	var normalized any
	if err := json.Unmarshal(data, &normalized); err != nil {
		t.Fatal(err)
	}
	return normalized
}

func TestEveryPublicFixtureMatchesSharedOracle(t *testing.T) {
	root := repositoryRoot(t)
	manifestData, err := os.ReadFile(filepath.Join(root, "benchmarks/fixtures/manifest.json"))
	if err != nil {
		t.Fatal(err)
	}
	var manifest fixtureManifest
	if err := json.Unmarshal(manifestData, &manifest); err != nil {
		t.Fatal(err)
	}
	if len(manifest.Fixtures) != 37 {
		t.Fatalf("fixture count = %d, want 37", len(manifest.Fixtures))
	}

	for _, entry := range manifest.Fixtures {
		t.Run(filepath.Base(entry.Path), func(t *testing.T) {
			data, err := os.ReadFile(filepath.Join(root, entry.Path))
			if err != nil {
				t.Fatal(err)
			}
			var fixture struct {
				Expected any `json:"expected"`
			}
			if err := json.Unmarshal(data, &fixture); err != nil {
				t.Fatal(err)
			}
			result, err := RunCase(data)
			if err != nil {
				t.Fatal(err)
			}
			if got := normalizeJSON(t, result.Actual); !reflect.DeepEqual(got, fixture.Expected) {
				t.Fatalf("actual = %#v, want %#v", got, fixture.Expected)
			}
		})
	}
}

func TestUnknownPriorityIsZeroEffectBoundaryError(t *testing.T) {
	result, err := RunCase([]byte(`{
		"case_id":"unknown",
		"service_version":2,
		"operation":"create_job",
		"request":{
			"api_version":2,
			"job_id":"job-1",
			"task":"task",
			"payload_base64":"",
			"priority":"urgent"
		},
		"initial_jobs":[]
	}`))
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]any{
		"decode_error": map[string]any{
			"code": "unknown_priority", "field": "priority", "value": "urgent",
		},
		"final_jobs":  []any{},
		"store_calls": []any{},
	}
	if got := normalizeJSON(t, result.Actual); !reflect.DeepEqual(got, want) {
		t.Fatalf("actual = %#v, want %#v", got, want)
	}
}

func TestV1ResponseProjectionOmitsInternalPriority(t *testing.T) {
	response := encodeResponse(domain.APIVersionV1, domain.Created{Job: domain.Job{
		JobID: "job-1", Task: "task", Priority: domain.PriorityHigh,
	}})
	normalized := normalizeJSON(t, response).(map[string]any)
	job := normalized["result"].(map[string]any)["job"].(map[string]any)
	if _, exists := job["priority"]; exists {
		t.Fatal("V1 response projection exposed internal priority")
	}
}

func TestV1StoredDecoderSetsNormalExplicitly(t *testing.T) {
	result, err := RunCase([]byte(`{
		"case_id":"legacy",
		"operation":"decode_stored_job",
		"stored_job":{
			"record_version":1,
			"job_id":"job-legacy",
			"task":"legacy-task",
			"payload_base64":"bGVnYWN5"
		}
	}`))
	if err != nil {
		t.Fatal(err)
	}
	normalized := normalizeJSON(t, result.Actual).(map[string]any)
	job := normalized["decoded_job"].(map[string]any)
	if job["priority"] != "normal" || job["record_version"] != float64(2) {
		t.Fatalf("decoded job = %#v, want V2 with normal priority", job)
	}
}

func TestClosedResponseVariantsEncodeStableIdentities(t *testing.T) {
	tests := []struct {
		name  string
		value domain.CreateJobResult
		kind  string
	}{
		{name: "created", value: domain.Created{Job: domain.Job{
			JobID: "job-1", Task: "task", Priority: domain.PriorityLow,
		}}, kind: "created"},
		{name: "invalid", value: domain.Invalid{Issues: []domain.ValidationIssue{{
			Field: domain.FieldPriority, Reason: domain.ReasonMissing,
		}}}, kind: "invalid"},
		{name: "already exists", value: domain.AlreadyExists{JobID: "job-1"}, kind: "already_exists"},
		{name: "unavailable", value: domain.PersistenceUnavailable{}, kind: "persistence_unavailable"},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			response := normalizeJSON(t, encodeResponse(domain.APIVersionV2, test.value)).(map[string]any)
			result := response["result"].(map[string]any)
			if result["kind"] != test.kind {
				t.Fatalf("kind = %v, want %q", result["kind"], test.kind)
			}
		})
	}
}

func TestInvalidFixtureBoundariesReturnErrors(t *testing.T) {
	tests := []struct {
		name    string
		fixture string
		message string
	}{
		{name: "malformed", fixture: `{`, message: "invalid fixture header"},
		{name: "unsupported operation", fixture: `{"operation":"remove_job"}`, message: "unsupported operation"},
		{name: "malformed create", fixture: `{"operation":"create_job","request":"wrong"}`, message: "invalid create_job fixture"},
		{name: "malformed decode", fixture: `{"operation":"decode_stored_job","stored_job":"wrong"}`, message: "invalid decode_stored_job fixture"},
		{name: "service version", fixture: createFixture(`"service_version":3`), message: "unsupported service version 3"},
		{name: "API version", fixture: createFixture(`"request":{"api_version":3}`), message: "unsupported request API version 3"},
		{name: "V2 request on V1 service", fixture: createFixture(`"service_version":1,"request":{"api_version":2}`), message: "accepts only API version 1"},
		{name: "request Base64", fixture: createFixture(`"request":{"api_version":1,"payload_base64":"!"}`), message: "invalid payload Base64"},
		{name: "store outcome", fixture: createFixture(`"store_outcome":"maybe"`), message: "unsupported store outcome"},
		{name: "initial stored Base64", fixture: createFixture(`"initial_jobs":[{"record_version":1,"payload_base64":"!"}]`), message: "invalid stored payload Base64"},
		{name: "stored version", fixture: decodeFixture(`"record_version":3`), message: "unsupported stored record version 3"},
		{name: "V2 stored missing priority", fixture: decodeFixture(`"record_version":2`), message: "missing priority"},
		{name: "V2 stored unknown priority", fixture: decodeFixture(`"record_version":2,"priority":"urgent"`), message: "unknown priority"},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			_, err := RunCase([]byte(test.fixture))
			if err == nil || !strings.Contains(err.Error(), test.message) {
				t.Fatalf("RunCase() error = %v, want containing %q", err, test.message)
			}
		})
	}
}

func createFixture(fields string) string {
	return `{
		"case_id":"case",
		"operation":"create_job",
		"service_version":2,
		"request":{
			"api_version":1,
			"job_id":"job-1",
			"task":"task",
			"payload_base64":""
		},
		"store_outcome":"inserted",
		` + fields + `
	}`
}

func decodeFixture(storedFields string) string {
	return `{
		"case_id":"case",
		"operation":"decode_stored_job",
		"stored_job":{
			"record_version":1,
			"job_id":"job-1",
			"task":"task",
			"payload_base64":"",
			` + storedFields + `
		}
	}`
}
