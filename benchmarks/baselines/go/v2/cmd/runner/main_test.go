package main

import (
	"bytes"
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

const highPriorityFixture = "benchmarks/fixtures/public/create_job/uc003-v2-created-priority-high.json"

func repositoryRoot(t *testing.T) string {
	t.Helper()
	root, err := findRepositoryRoot()
	if err != nil {
		t.Fatal(err)
	}
	return root
}

func TestArgumentContractRejectsMissingAndExtraArguments(t *testing.T) {
	tests := [][]string{
		nil,
		{"--case"},
		{"--case", "x", "y"},
		{"--unknown", "x"},
	}
	for _, arguments := range tests {
		if _, err := run(arguments); err == nil {
			t.Errorf("run(%q) succeeded, want error", arguments)
		}
	}
}

func TestOneCaseReturnsNormalizedResult(t *testing.T) {
	value, err := run([]string{"--case", highPriorityFixture})
	if err != nil {
		t.Fatal(err)
	}
	data, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	var result map[string]any
	if err := json.Unmarshal(data, &result); err != nil {
		t.Fatal(err)
	}
	if result["result_format"] != float64(1) ||
		result["case_id"] != "uc003-v2-created-priority-high" {
		t.Fatalf("result = %#v, want normalized high-priority case", result)
	}
}

func TestOneCaseAcceptsAbsoluteFixturePath(t *testing.T) {
	path := filepath.Join(repositoryRoot(t), highPriorityFixture)
	if _, err := run([]string{"--case", path}); err != nil {
		t.Fatal(err)
	}
}

func TestCorpusPreservesManifestDigestCountAndOrder(t *testing.T) {
	value, err := run([]string{"--corpus", "benchmarks/fixtures/manifest.json"})
	if err != nil {
		t.Fatal(err)
	}
	result := value.(corpusResult)
	if len(result.Results) != 37 {
		t.Fatalf("result count = %d, want 37", len(result.Results))
	}
	const digest = "33b8369ca3680367b5371811ce52ef639a878696698f70765def0f6c9e8c1eb5"
	if result.FixtureManifestSHA256 != digest {
		t.Fatalf("manifest digest = %s, want %s", result.FixtureManifestSHA256, digest)
	}
	if result.Results[0].CaseID != "uc001-v1-created-empty-payload" ||
		result.Results[36].CaseID != "uc003-v1-stored-job-adapted" {
		t.Fatalf(
			"first/last case = %q/%q, want manifest order",
			result.Results[0].CaseID,
			result.Results[36].CaseID,
		)
	}
}

func TestRunCLISeparatesJSONOutputAndDiagnostics(t *testing.T) {
	var stdout, stderr bytes.Buffer
	if code := runCLI([]string{"--case", highPriorityFixture}, &stdout, &stderr); code != 0 {
		t.Fatalf("runCLI() exit = %d, stderr = %s", code, stderr.String())
	}
	if stderr.Len() != 0 {
		t.Fatalf("successful stderr = %q, want empty", stderr.String())
	}
	decoder := json.NewDecoder(&stdout)
	var result map[string]any
	if err := decoder.Decode(&result); err != nil {
		t.Fatalf("stdout is not JSON: %v", err)
	}
	if decoder.Decode(&struct{}{}) == nil {
		t.Fatal("stdout contains more than one JSON value")
	}

	stdout.Reset()
	stderr.Reset()
	if code := runCLI([]string{"--unknown"}, &stdout, &stderr); code == 0 {
		t.Fatal("invalid command exited zero")
	}
	if stdout.Len() != 0 || !strings.Contains(stderr.String(), "expected exactly") {
		t.Fatalf("invalid command stdout/stderr = %q/%q", stdout.String(), stderr.String())
	}
}

func TestMissingAndMalformedInputsReturnErrors(t *testing.T) {
	if _, err := run([]string{"--case", "benchmarks/fixtures/public/missing.json"}); err == nil {
		t.Fatal("missing fixture succeeded")
	}
	if _, err := run([]string{"--corpus", "benchmarks/fixtures/missing.json"}); err == nil {
		t.Fatal("missing manifest succeeded")
	}

	root := repositoryRoot(t)
	temp, err := os.CreateTemp(root, "malformed-manifest-*.json")
	if err != nil {
		t.Fatal(err)
	}
	name := temp.Name()
	t.Cleanup(func() { _ = os.Remove(name) })
	if _, err := temp.WriteString("{"); err != nil {
		t.Fatal(err)
	}
	if err := temp.Close(); err != nil {
		t.Fatal(err)
	}
	if _, err := run([]string{"--corpus", name}); err == nil ||
		!strings.Contains(err.Error(), "could not parse") {
		t.Fatalf("malformed corpus error = %v, want parse error", err)
	}

	fixtureFile, err := os.CreateTemp(root, "malformed-fixture-*.json")
	if err != nil {
		t.Fatal(err)
	}
	fixtureName := fixtureFile.Name()
	t.Cleanup(func() { _ = os.Remove(fixtureName) })
	if _, err := fixtureFile.WriteString(`{"operation":"remove_job"}`); err != nil {
		t.Fatal(err)
	}
	if err := fixtureFile.Close(); err != nil {
		t.Fatal(err)
	}
	if _, err := run([]string{"--case", fixtureName}); err == nil ||
		!strings.Contains(err.Error(), "could not run") {
		t.Fatalf("invalid fixture error = %v, want run error", err)
	}

	corpusFile, err := os.CreateTemp(root, "missing-entry-manifest-*.json")
	if err != nil {
		t.Fatal(err)
	}
	corpusName := corpusFile.Name()
	t.Cleanup(func() { _ = os.Remove(corpusName) })
	if _, err := corpusFile.WriteString(
		`{"fixtures":[{"path":"benchmarks/fixtures/public/missing.json"}]}`,
	); err != nil {
		t.Fatal(err)
	}
	if err := corpusFile.Close(); err != nil {
		t.Fatal(err)
	}
	if _, err := run([]string{"--corpus", corpusName}); err == nil ||
		!strings.Contains(err.Error(), "could not read") {
		t.Fatalf("missing corpus entry error = %v, want read error", err)
	}
}

func TestBinaryWritesExactlyOneJSONValue(t *testing.T) {
	command := exec.Command("go", "run", ".")
	command.Args = append(command.Args, "--case", highPriorityFixture)
	command.Dir = filepath.Join(repositoryRoot(t), "benchmarks/baselines/go/v2/cmd/runner")
	output, err := command.Output()
	if err != nil {
		t.Fatal(err)
	}
	var result map[string]any
	if err := json.Unmarshal(output, &result); err != nil {
		t.Fatalf("binary stdout is not exactly one JSON value: %v", err)
	}
	if result["case_id"] != "uc003-v2-created-priority-high" {
		t.Fatalf("case_id = %v, want high-priority fixture", result["case_id"])
	}
}

type failingWriter struct{}

func (failingWriter) Write([]byte) (int, error) {
	return 0, os.ErrPermission
}

func TestRunCLIReportsJSONEncodingFailure(t *testing.T) {
	var stderr bytes.Buffer
	code := runCLI([]string{"--case", highPriorityFixture}, failingWriter{}, &stderr)
	if code == 0 || !strings.Contains(stderr.String(), "could not encode runner result") {
		t.Fatalf("runCLI() exit/stderr = %d/%q, want encoding failure", code, stderr.String())
	}
}

func TestRepositoryRootOutsideCheckoutReturnsError(t *testing.T) {
	original, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	if err := os.Chdir(t.TempDir()); err != nil {
		t.Fatal(err)
	}
	defer func() {
		if err := os.Chdir(original); err != nil {
			t.Fatalf("restore working directory: %v", err)
		}
	}()
	if _, err := findRepositoryRoot(); err == nil {
		t.Fatal("findRepositoryRoot() outside checkout succeeded")
	}
}
