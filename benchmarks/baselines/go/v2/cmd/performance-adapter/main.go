// Command performance-adapter exposes the frozen Go V2 fixture boundary to M8.
package main

import (
	"bufio"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"ail.dev/job-service-baseline/v2/fixture"
)

type manifest struct {
	Fixtures []struct {
		Path string `json:"path"`
	} `json:"fixtures"`
}

type command struct {
	Command      string `json:"command"`
	Iterations   int    `json:"iterations"`
	DurationNS   int64  `json:"duration_ns"`
	SampleStride int    `json:"sample_stride"`
}

func emit(value any) error {
	return json.NewEncoder(os.Stdout).Encode(value)
}

func loadCases(manifestPath string) ([][]byte, error) {
	content, err := os.ReadFile(manifestPath)
	if err != nil {
		return nil, err
	}
	var value manifest
	if err := json.Unmarshal(content, &value); err != nil {
		return nil, err
	}
	root := filepath.Clean(filepath.Join(filepath.Dir(manifestPath), "..", ".."))
	cases := make([][]byte, len(value.Fixtures))
	for index, entry := range value.Fixtures {
		cases[index], err = os.ReadFile(filepath.Join(root, entry.Path))
		if err != nil {
			return nil, err
		}
	}
	return cases, nil
}

func run(cases [][]byte) ([]fixture.SingleCaseResult, error) {
	results := make([]fixture.SingleCaseResult, len(cases))
	for index, content := range cases {
		result, err := fixture.RunCase(content)
		if err != nil {
			return nil, err
		}
		results[index] = result
	}
	return results, nil
}

func main() {
	manifestPath := flag.String("manifest", "", "absolute shared fixture manifest")
	flag.Parse()
	cases, err := loadCases(*manifestPath)
	if err != nil {
		panic(err)
	}
	if err := emit(map[string]any{"type": "ready", "case_count": len(cases)}); err != nil {
		panic(err)
	}
	scanner := bufio.NewScanner(os.Stdin)
	for scanner.Scan() {
		var request command
		if err := json.Unmarshal(scanner.Bytes(), &request); err != nil {
			panic(err)
		}
		switch request.Command {
		case "verify":
			results, err := run(cases)
			if err != nil {
				panic(err)
			}
			if err := emit(map[string]any{"type": "verified", "results": results}); err != nil {
				panic(err)
			}
		case "warmup":
			checksum := 0
			for iteration := 0; iteration < request.Iterations; iteration++ {
				results, err := run(cases)
				if err != nil {
					panic(err)
				}
				for _, result := range results {
					checksum ^= len(result.CaseID)
				}
			}
			if err := emit(map[string]any{
				"type": "warmed", "iterations": request.Iterations,
				"request_count": request.Iterations * len(cases), "checksum": checksum,
			}); err != nil {
				panic(err)
			}
		case "measure":
			started := time.Now()
			samples := make([]int64, 0)
			requestCount := 0
			checksum := 0
			for requestCount == 0 || time.Since(started).Nanoseconds() < request.DurationNS {
				for _, content := range cases {
					before := time.Now()
					result, err := fixture.RunCase(content)
					if err != nil {
						panic(err)
					}
					elapsed := time.Since(before).Nanoseconds()
					if requestCount%request.SampleStride == 0 {
						samples = append(samples, elapsed)
					}
					requestCount++
					checksum ^= len(result.CaseID)
				}
			}
			if err := emit(map[string]any{
				"type": "measured", "clock": "time.Since-monotonic",
				"elapsed_ns":    time.Since(started).Nanoseconds(),
				"request_count": requestCount, "sample_stride": request.SampleStride,
				"samples_ns": samples, "checksum": checksum,
			}); err != nil {
				panic(err)
			}
		case "shutdown":
			if err := emit(map[string]string{"type": "stopped"}); err != nil {
				panic(err)
			}
			return
		default:
			panic(fmt.Sprintf("unsupported command %q", request.Command))
		}
	}
	if err := scanner.Err(); err != nil {
		panic(err)
	}
}
