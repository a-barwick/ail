// Package fixture adapts the frozen JSON corpus to the Go service boundary.
package fixture

import (
	"encoding/base64"
	"encoding/json"
	"fmt"

	"ail.dev/job-service-baseline/v2/domain"
	"ail.dev/job-service-baseline/v2/service"
	"ail.dev/job-service-baseline/v2/store"
)

type SingleCaseResult struct {
	ResultFormat int    `json:"result_format"`
	CaseID       string `json:"case_id"`
	Operation    string `json:"operation"`
	Actual       any    `json:"actual"`
}

type caseHeader struct {
	Operation string `json:"operation"`
}

type createCase struct {
	CaseID         string         `json:"case_id"`
	ServiceVersion int            `json:"service_version"`
	Request        rawRequest     `json:"request"`
	InitialJobs    []rawStoredJob `json:"initial_jobs"`
	StoreOutcome   *string        `json:"store_outcome"`
}

type decodeStoredCase struct {
	CaseID    string       `json:"case_id"`
	StoredJob rawStoredJob `json:"stored_job"`
}

type rawRequest struct {
	APIVersion    int     `json:"api_version"`
	JobID         string  `json:"job_id"`
	Task          string  `json:"task"`
	PayloadBase64 string  `json:"payload_base64"`
	Priority      *string `json:"priority"`
}

type rawStoredJob struct {
	RecordVersion int     `json:"record_version"`
	JobID         string  `json:"job_id"`
	Task          string  `json:"task"`
	PayloadBase64 string  `json:"payload_base64"`
	Priority      *string `json:"priority"`
}

type decodedRequest struct {
	version domain.APIVersion
	request domain.CreateJobRequest
}

type unknownPriority struct {
	value string
}

type responseEnvelope struct {
	APIVersion int `json:"api_version"`
	Result     any `json:"result"`
}

type validationIssueJSON struct {
	Field  domain.ValidationField  `json:"field"`
	Reason domain.ValidationReason `json:"reason"`
}

type jobV1JSON struct {
	JobID         string `json:"job_id"`
	Task          string `json:"task"`
	PayloadBase64 string `json:"payload_base64"`
}

type jobV2ResponseJSON struct {
	JobID         string          `json:"job_id"`
	Task          string          `json:"task"`
	PayloadBase64 string          `json:"payload_base64"`
	Priority      domain.Priority `json:"priority"`
}

type storedJobJSON struct {
	RecordVersion int              `json:"record_version"`
	JobID         string           `json:"job_id"`
	Task          string           `json:"task"`
	PayloadBase64 string           `json:"payload_base64"`
	Priority      *domain.Priority `json:"priority,omitempty"`
}

type storeCallJSON struct {
	Operation string        `json:"operation"`
	Job       storedJobJSON `json:"job"`
}

// RunCase executes one parsed fixture through the Go service boundary.
func RunCase(data []byte) (SingleCaseResult, error) {
	var header caseHeader
	if err := json.Unmarshal(data, &header); err != nil {
		return SingleCaseResult{}, fmt.Errorf("invalid fixture header: %w", err)
	}
	switch header.Operation {
	case "create_job":
		return runCreateCase(data)
	case "decode_stored_job":
		return runDecodeStoredCase(data)
	default:
		return SingleCaseResult{}, fmt.Errorf("unsupported operation %q", header.Operation)
	}
}

func runCreateCase(data []byte) (SingleCaseResult, error) {
	var raw createCase
	if err := json.Unmarshal(data, &raw); err != nil {
		return SingleCaseResult{}, fmt.Errorf("invalid create_job fixture: %w", err)
	}
	insertVersion, err := parseServiceVersion(raw.ServiceVersion)
	if err != nil {
		return SingleCaseResult{}, err
	}
	initialJobs := make([]store.StoredJob, len(raw.InitialJobs))
	for index, value := range raw.InitialJobs {
		initialJobs[index], err = parseStoredJob(value)
		if err != nil {
			return SingleCaseResult{}, err
		}
	}

	decoded, unknown, err := decodeRequest(raw.ServiceVersion, raw.Request)
	if err != nil {
		return SingleCaseResult{}, err
	}
	var actual any
	if unknown != nil {
		actual = struct {
			DecodeError struct {
				Code  string `json:"code"`
				Field string `json:"field"`
				Value string `json:"value"`
			} `json:"decode_error"`
			FinalJobs  []storedJobJSON `json:"final_jobs"`
			StoreCalls []storeCallJSON `json:"store_calls"`
		}{
			DecodeError: struct {
				Code  string `json:"code"`
				Field string `json:"field"`
				Value string `json:"value"`
			}{
				Code: "unknown_priority", Field: "priority", Value: unknown.value,
			},
			FinalJobs:  encodeStoredJobs(initialJobs),
			StoreCalls: []storeCallJSON{},
		}
	} else {
		outcome, err := parseStoreOutcome(raw.StoreOutcome)
		if err != nil {
			return SingleCaseResult{}, err
		}
		jobStore := store.New(initialJobs, outcome, insertVersion)
		result := service.CreateJob(decoded.request, jobStore)
		actual = struct {
			Response   responseEnvelope `json:"response"`
			FinalJobs  []storedJobJSON  `json:"final_jobs"`
			StoreCalls []storeCallJSON  `json:"store_calls"`
		}{
			Response:   encodeResponse(decoded.version, result),
			FinalJobs:  encodeStoredJobs(jobStore.Jobs()),
			StoreCalls: encodeStoreCalls(jobStore.Calls()),
		}
	}

	return SingleCaseResult{
		ResultFormat: 1,
		CaseID:       raw.CaseID,
		Operation:    "create_job",
		Actual:       actual,
	}, nil
}

func runDecodeStoredCase(data []byte) (SingleCaseResult, error) {
	var raw decodeStoredCase
	if err := json.Unmarshal(data, &raw); err != nil {
		return SingleCaseResult{}, fmt.Errorf("invalid decode_stored_job fixture: %w", err)
	}
	stored, err := parseStoredJob(raw.StoredJob)
	if err != nil {
		return SingleCaseResult{}, err
	}
	return SingleCaseResult{
		ResultFormat: 1,
		CaseID:       raw.CaseID,
		Operation:    "decode_stored_job",
		Actual: struct {
			DecodedJob storedJobJSON `json:"decoded_job"`
		}{
			DecodedJob: encodeJobV2(stored.AdaptToV2()),
		},
	}, nil
}

func parseServiceVersion(version int) (store.RecordVersion, error) {
	switch version {
	case 1:
		return store.RecordVersionV1, nil
	case 2:
		return store.RecordVersionV2, nil
	default:
		return 0, fmt.Errorf("unsupported service version %d", version)
	}
}

func decodeRequest(
	serviceVersion int,
	raw rawRequest,
) (decodedRequest, *unknownPriority, error) {
	var version domain.APIVersion
	switch raw.APIVersion {
	case 1:
		version = domain.APIVersionV1
	case 2:
		version = domain.APIVersionV2
	default:
		return decodedRequest{}, nil, fmt.Errorf(
			"unsupported request API version %d",
			raw.APIVersion,
		)
	}
	if serviceVersion == 1 && version != domain.APIVersionV1 {
		return decodedRequest{}, nil, fmt.Errorf(
			"service version 1 accepts only API version 1",
		)
	}

	var priority *domain.Priority
	if version == domain.APIVersionV1 {
		normal := domain.PriorityNormal
		priority = &normal
	} else if raw.Priority != nil {
		parsed, ok := domain.ParsePriority(*raw.Priority)
		if !ok {
			return decodedRequest{}, &unknownPriority{value: *raw.Priority}, nil
		}
		priority = &parsed
	}
	payload, err := base64.StdEncoding.DecodeString(raw.PayloadBase64)
	if err != nil {
		return decodedRequest{}, nil, fmt.Errorf("invalid payload Base64: %w", err)
	}
	return decodedRequest{
		version: version,
		request: domain.CreateJobRequest{
			JobID:    raw.JobID,
			Task:     raw.Task,
			Payload:  payload,
			Priority: priority,
		},
	}, nil, nil
}

func parseStoredJob(raw rawStoredJob) (store.StoredJob, error) {
	payload, err := base64.StdEncoding.DecodeString(raw.PayloadBase64)
	if err != nil {
		return store.StoredJob{}, fmt.Errorf("invalid stored payload Base64: %w", err)
	}
	stored := store.StoredJob{
		JobID:   raw.JobID,
		Task:    raw.Task,
		Payload: payload,
	}
	switch raw.RecordVersion {
	case 1:
		stored.RecordVersion = store.RecordVersionV1
	case 2:
		if raw.Priority == nil {
			return store.StoredJob{}, fmt.Errorf("version-two stored job is missing priority")
		}
		priority, ok := domain.ParsePriority(*raw.Priority)
		if !ok {
			return store.StoredJob{}, fmt.Errorf(
				"version-two stored job has unknown priority %q",
				*raw.Priority,
			)
		}
		stored.RecordVersion = store.RecordVersionV2
		stored.Priority = &priority
	default:
		return store.StoredJob{}, fmt.Errorf(
			"unsupported stored record version %d",
			raw.RecordVersion,
		)
	}
	return stored, nil
}

func parseStoreOutcome(value *string) (domain.InsertOutcome, error) {
	if value == nil {
		return domain.UnavailableBeforeCommit, nil
	}
	switch *value {
	case "inserted":
		return domain.Inserted, nil
	case "duplicate":
		return domain.Duplicate, nil
	case "unavailable_before_commit":
		return domain.UnavailableBeforeCommit, nil
	default:
		return 0, fmt.Errorf("unsupported store outcome %q", *value)
	}
}

func encodeResponse(version domain.APIVersion, result domain.CreateJobResult) responseEnvelope {
	var encoded any
	switch result := result.(type) {
	case domain.Created:
		if version == domain.APIVersionV1 {
			encoded = struct {
				Kind string    `json:"kind"`
				Job  jobV1JSON `json:"job"`
			}{Kind: "created", Job: encodeJobV1(result.Job)}
		} else {
			encoded = struct {
				Kind string            `json:"kind"`
				Job  jobV2ResponseJSON `json:"job"`
			}{Kind: "created", Job: encodeJobV2Response(result.Job)}
		}
	case domain.Invalid:
		issues := make([]validationIssueJSON, len(result.Issues))
		for index, issue := range result.Issues {
			issues[index] = validationIssueJSON{Field: issue.Field, Reason: issue.Reason}
		}
		encoded = struct {
			Kind   string                `json:"kind"`
			Issues []validationIssueJSON `json:"issues"`
		}{Kind: "invalid", Issues: issues}
	case domain.AlreadyExists:
		encoded = struct {
			Kind  string `json:"kind"`
			JobID string `json:"job_id"`
		}{Kind: "already_exists", JobID: result.JobID}
	case domain.PersistenceUnavailable:
		encoded = struct {
			Kind string `json:"kind"`
		}{Kind: "persistence_unavailable"}
	default:
		panic(fmt.Sprintf("unhandled create-job result type %T", result))
	}
	return responseEnvelope{APIVersion: int(version), Result: encoded}
}

func encodeJobV1(job domain.Job) jobV1JSON {
	return jobV1JSON{
		JobID:         job.JobID,
		Task:          job.Task,
		PayloadBase64: base64.StdEncoding.EncodeToString(job.Payload),
	}
}

func encodeJobV2Response(job domain.Job) jobV2ResponseJSON {
	return jobV2ResponseJSON{
		JobID:         job.JobID,
		Task:          job.Task,
		PayloadBase64: base64.StdEncoding.EncodeToString(job.Payload),
		Priority:      job.Priority,
	}
}

func encodeJobV2(job domain.Job) storedJobJSON {
	priority := job.Priority
	return storedJobJSON{
		RecordVersion: 2,
		JobID:         job.JobID,
		Task:          job.Task,
		PayloadBase64: base64.StdEncoding.EncodeToString(job.Payload),
		Priority:      &priority,
	}
}

func encodeStoredJobs(jobs []store.StoredJob) []storedJobJSON {
	result := make([]storedJobJSON, len(jobs))
	for index, job := range jobs {
		result[index] = storedJobJSON{
			RecordVersion: int(job.RecordVersion),
			JobID:         job.JobID,
			Task:          job.Task,
			PayloadBase64: base64.StdEncoding.EncodeToString(job.Payload),
			Priority:      job.Priority,
		}
	}
	return result
}

func encodeStoreCalls(calls []store.StoreCall) []storeCallJSON {
	result := make([]storeCallJSON, len(calls))
	for index, call := range calls {
		result[index] = storeCallJSON{
			Operation: "insert_if_absent",
			Job:       encodeStoredJobs([]store.StoredJob{call.Job})[0],
		}
	}
	return result
}
