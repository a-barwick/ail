from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TOOLS = ROOT / "benchmarks" / "tools"
sys.path.insert(0, str(TOOLS))
TOOL_PATH = TOOLS / "calibration.py"
SPEC = importlib.util.spec_from_file_location("calibration_tool", TOOL_PATH)
assert SPEC and SPEC.loader
calibration = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = calibration
SPEC.loader.exec_module(calibration)


class CalibrationContractTests(unittest.TestCase):
    def test_contract_lock_and_m8a_values_match(self) -> None:
        contract = calibration.check_contract_lock()
        self.assertEqual(contract["contract_id"], calibration.CONFIGURATION_ID)
        self.assertEqual(
            contract["token_accounting"]["categories"],
            list(calibration.TOKEN_CATEGORIES),
        )
        self.assertEqual(
            contract["token_accounting"]["reconciliation_tolerance_tokens"],
            0,
        )

    def test_synthetic_campaign_outcomes_are_stable(self) -> None:
        outcomes = dict(calibration.verify_synthetic_campaigns())
        self.assertEqual(
            outcomes,
            {
                "empty": "accepted:empty",
                "pilot": "accepted:pilot",
                "partial": "accepted:partial",
                "malformed": "agent_trial_invalid",
                "complete": "accepted:complete",
                "missing-counts": "campaign_counts_missing",
                "duplicate-trial-identity": "duplicate_trial_identity",
                "changed-inputs": "changed_campaign_input",
                "invalid-hash": "evidence_hash_invalid",
                "incomplete-token-categories": "token_categories_incomplete",
                "missing-raw-events": "raw_events_missing",
                "unaccounted-exclusion": "exclusion_unaccounted",
                "incorrect-summary": "report_summary_incorrect",
                "mixed-configuration": "mixed_configuration",
            },
        )


if __name__ == "__main__":
    unittest.main()
