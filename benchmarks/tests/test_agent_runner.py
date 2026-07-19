from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TOOLS = ROOT / "benchmarks" / "tools"
sys.path.insert(0, str(TOOLS))
TOOL_PATH = TOOLS / "agent_runner.py"
SPEC = importlib.util.spec_from_file_location("agent_runner_tool", TOOL_PATH)
assert SPEC and SPEC.loader
agent_runner = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = agent_runner
SPEC.loader.exec_module(agent_runner)


class InteractiveAgentRunnerTests(unittest.TestCase):
    def test_fake_and_dry_outcomes_are_stable(self) -> None:
        self.assertEqual(
            agent_runner.verify_fake_and_dry_streams(),
            [
                ("success", "complete"),
                ("failure", "nonzero_exit"),
                ("timeout", "timed_out"),
                ("permission", "permission_violation"),
                ("token-limit", "input_token_limit"),
                ("incomplete-evidence", "incomplete_evidence"),
            ],
        )

    def test_permission_profile_is_task_start_derived(self) -> None:
        with tempfile.TemporaryDirectory(prefix="ail-m8c-test-") as raw:
            prepared = agent_runner.prepare_trial(
                "go", "UC-003", Path(raw) / "workspace"
            )
            self.assertTrue(
                agent_runner.check_write_permission(
                    prepared, "benchmarks/baselines/go/v1/jobservice.go"
                )
            )
            self.assertFalse(
                agent_runner.check_write_permission(
                    prepared, "benchmarks/fixtures/manifest.json"
                )
            )
            self.assertFalse(
                agent_runner.check_write_permission(prepared, "/etc/passwd")
            )

    def test_invalid_token_reconciliation_is_incomplete_evidence(self) -> None:
        stream = agent_runner.InteractiveStream(
            "dry.tokens", clock=lambda: 1_000_000_000
        )
        stream.start()
        categories = {category: 0 for category in agent_runner.TOKEN_CATEGORIES}
        categories["initial_context"] = 8
        forwarded = stream.model_request(
            "request-1",
            preflight_input_tokens=11,
            provider_input_tokens=12,
            cached_input_tokens=0,
            protocol_overhead_tokens=2,
            categories=categories,
            body={"model": "gpt-5.6-sol", "input": []},
        )
        self.assertFalse(forwarded)
        result = stream.finish()
        self.assertEqual(result.terminal_class, "failed")
        self.assertEqual(result.terminal_cause, "incomplete_evidence")


if __name__ == "__main__":
    unittest.main()
