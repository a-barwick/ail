package contract

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

type checkpointManifest struct {
	Checkpoints []struct {
		ID               string   `json:"id"`
		SourceTreeSHA256 string   `json:"source_tree_sha256"`
		Files            []string `json:"files"`
	} `json:"checkpoints"`
}

type seedLocations struct {
	Language  string `json:"language"`
	Locations []struct {
		SeedID            string   `json:"seed_id"`
		SemanticLocations []string `json:"semantic_locations"`
	} `json:"locations"`
}

type hiddenContract struct {
	SeedCategories []struct {
		ID string `json:"id"`
	} `json:"seed_categories"`
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

func readJSON(t *testing.T, path string, destination any) {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(data, destination); err != nil {
		t.Fatal(err)
	}
}

func TestCheckpointSourceTreesMatchFrozenDigests(t *testing.T) {
	root := repositoryRoot(t)
	var manifest checkpointManifest
	readJSON(t, filepath.Join(root, "benchmarks/baselines/go/checkpoints.json"), &manifest)
	if len(manifest.Checkpoints) != 2 {
		t.Fatalf("checkpoint count = %d, want 2", len(manifest.Checkpoints))
	}

	for _, checkpoint := range manifest.Checkpoints {
		var records strings.Builder
		for _, path := range checkpoint.Files {
			data, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(path)))
			if err != nil {
				t.Fatal(err)
			}
			digest := sha256.Sum256(data)
			records.WriteString(hex.EncodeToString(digest[:]))
			records.WriteString("  ")
			records.WriteString(path)
			records.WriteByte('\n')
		}
		treeDigest := sha256.Sum256([]byte(records.String()))
		actual := hex.EncodeToString(treeDigest[:])
		if actual != checkpoint.SourceTreeSHA256 {
			t.Errorf(
				"%s source tree digest = %s, want %s",
				checkpoint.ID,
				actual,
				checkpoint.SourceTreeSHA256,
			)
		}
	}
}

func TestGoSeedLocationsCoverEveryFrozenCategoryOnce(t *testing.T) {
	root := repositoryRoot(t)
	var hidden hiddenContract
	readJSON(t, filepath.Join(root, "benchmarks/contracts/hidden-contract.json"), &hidden)
	var locations seedLocations
	readJSON(t, filepath.Join(root, "benchmarks/baselines/go/seed-locations.json"), &locations)

	if locations.Language != "go" {
		t.Fatalf("seed language = %q, want go", locations.Language)
	}
	if len(locations.Locations) != len(hidden.SeedCategories) {
		t.Fatalf(
			"seed location count = %d, want %d",
			len(locations.Locations),
			len(hidden.SeedCategories),
		)
	}
	for index, category := range hidden.SeedCategories {
		location := locations.Locations[index]
		if location.SeedID != category.ID {
			t.Errorf("seed %d = %q, want %q", index, location.SeedID, category.ID)
		}
		if len(location.SemanticLocations) == 0 {
			t.Errorf("seed %q has no semantic locations", location.SeedID)
		}
	}
}
