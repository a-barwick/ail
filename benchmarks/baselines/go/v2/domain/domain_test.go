package domain

import "testing"

func TestParsePriorityRecognizesOnlyClosedIdentities(t *testing.T) {
	for _, value := range []Priority{PriorityLow, PriorityNormal, PriorityHigh} {
		got, ok := ParsePriority(string(value))
		if !ok || got != value {
			t.Errorf("ParsePriority(%q) = (%q, %t), want (%q, true)", value, got, ok, value)
		}
	}
	for _, value := range []string{"", "urgent", "HIGH", "normal "} {
		if got, ok := ParsePriority(value); ok || got != "" {
			t.Errorf("ParsePriority(%q) = (%q, %t), want (\"\", false)", value, got, ok)
		}
	}
}
