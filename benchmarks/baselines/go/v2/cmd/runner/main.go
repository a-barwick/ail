package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"ail.dev/job-service-baseline/v2/fixture"
)

type fixtureManifest struct {
	Fixtures []fixtureEntry `json:"fixtures"`
}

type fixtureEntry struct {
	Path string `json:"path"`
}

type corpusResult struct {
	ResultFormat          int                        `json:"result_format"`
	FixtureManifestSHA256 string                     `json:"fixture_manifest_sha256"`
	Results               []fixture.SingleCaseResult `json:"results"`
}

func main() {
	os.Exit(runCLI(os.Args[1:], os.Stdout, os.Stderr))
}

func runCLI(arguments []string, stdout, stderr io.Writer) int {
	result, err := run(arguments)
	if err != nil {
		fmt.Fprintf(stderr, "go baseline runner: %v\n", err)
		return 1
	}
	encoder := json.NewEncoder(stdout)
	encoder.SetEscapeHTML(false)
	if err := encoder.Encode(result); err != nil {
		fmt.Fprintf(stderr, "go baseline runner: could not encode runner result: %v\n", err)
		return 1
	}
	return 0
}

func run(arguments []string) (any, error) {
	if len(arguments) != 2 {
		return nil, errors.New("expected exactly --case <fixture> or --corpus <manifest>")
	}
	switch arguments[0] {
	case "--case":
		path, _, err := resolveRepositoryPath(arguments[1])
		if err != nil {
			return nil, err
		}
		return runCaseFile(path)
	case "--corpus":
		path, root, err := resolveRepositoryPath(arguments[1])
		if err != nil {
			return nil, err
		}
		return runCorpus(path, root)
	default:
		return nil, errors.New("expected exactly --case <fixture> or --corpus <manifest>")
	}
}

func runCaseFile(path string) (fixture.SingleCaseResult, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return fixture.SingleCaseResult{}, fmt.Errorf("could not read %s: %w", path, err)
	}
	result, err := fixture.RunCase(data)
	if err != nil {
		return fixture.SingleCaseResult{}, fmt.Errorf("could not run %s: %w", path, err)
	}
	return result, nil
}

func runCorpus(path, root string) (corpusResult, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return corpusResult{}, fmt.Errorf("could not read %s: %w", path, err)
	}
	var manifest fixtureManifest
	if err := json.Unmarshal(data, &manifest); err != nil {
		return corpusResult{}, fmt.Errorf("could not parse %s: %w", path, err)
	}
	results := make([]fixture.SingleCaseResult, len(manifest.Fixtures))
	for index, entry := range manifest.Fixtures {
		results[index], err = runCaseFile(filepath.Join(root, filepath.FromSlash(entry.Path)))
		if err != nil {
			return corpusResult{}, err
		}
	}
	digest := sha256.Sum256(data)
	return corpusResult{
		ResultFormat:          1,
		FixtureManifestSHA256: hex.EncodeToString(digest[:]),
		Results:               results,
	}, nil
}

func resolveRepositoryPath(path string) (absolutePath, repositoryRoot string, err error) {
	root, err := findRepositoryRoot()
	if err != nil {
		return "", "", err
	}
	if filepath.IsAbs(path) {
		return filepath.Clean(path), root, nil
	}
	return filepath.Join(root, filepath.FromSlash(path)), root, nil
}

func findRepositoryRoot() (string, error) {
	current, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("could not determine working directory: %w", err)
	}
	for {
		if info, err := os.Stat(filepath.Join(current, ".git")); err == nil && info.IsDir() {
			return current, nil
		}
		parent := filepath.Dir(current)
		if parent == current {
			return "", errors.New("runner working directory is not inside the repository")
		}
		current = parent
	}
}
