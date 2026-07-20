from __future__ import annotations

import importlib.util
import json
import os
import shutil
import sys
import tempfile
import threading
import unittest
import urllib.request
from http.server import BaseHTTPRequestHandler
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
import responses_recorder


class FakeResponsesUpstream:
    def __init__(self) -> None:
        owner = self

        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def log_message(self, _format: str, *_args: object) -> None:
                return

            def do_POST(self) -> None:
                length = int(self.headers["Content-Length"])
                body = json.loads(self.rfile.read(length))
                count = 7 + len(body.get("instructions", "")) + 11 * len(
                    body.get("input", [])
                )
                if self.path == "/v1/responses/input_tokens":
                    raw = json.dumps({"input_tokens": count}).encode()
                    content_type = "application/json"
                elif self.path == "/v1/responses":
                    completed = {
                        "id": "resp_fake",
                        "object": "response",
                        "model": body["model"],
                        "output": [],
                        "usage": {
                            "input_tokens": count,
                            "input_tokens_details": {"cached_tokens": 0},
                            "output_tokens": 0,
                            "total_tokens": count,
                        },
                    }
                    event = {
                        "type": "response.completed",
                        "sequence_number": 1,
                        "response": completed,
                    }
                    raw = (
                        f"event: response.completed\ndata: {json.dumps(event)}\n\n"
                        "data: [DONE]\n\n"
                    ).encode()
                    content_type = "text/event-stream"
                else:
                    self.send_error(404)
                    return
                self.send_response(200)
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", str(len(raw)))
                self.end_headers()
                self.wfile.write(raw)

        self.server = responses_recorder.LoopbackHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    @property
    def url(self) -> str:
        return f"http://127.0.0.1:{self.server.server_port}/v1"

    def __enter__(self) -> "FakeResponsesUpstream":
        self.thread.start()
        return self

    def __exit__(self, *_args: object) -> None:
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=5)


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

    def test_uc003_can_create_only_language_source(self) -> None:
        expected = {
            "rust": "benchmarks/baselines/rust/v2/src/lib.rs",
            "go": "benchmarks/baselines/go/v2/domain/domain.go",
            "python": "benchmarks/baselines/python/v2/domain.py",
            "typescript": "benchmarks/baselines/typescript/v2/domain.ts",
        }
        with tempfile.TemporaryDirectory(prefix="ail-m8c-source-test-") as raw:
            for language, source_path in expected.items():
                prepared = agent_runner.prepare_trial(
                    language, "UC-003", Path(raw) / language
                )
                self.assertFalse((prepared.workspace / source_path).exists())
                self.assertTrue(
                    agent_runner.check_write_permission(prepared, source_path)
                )
                self.assertFalse(
                    agent_runner.check_write_permission(
                        prepared, "benchmarks/fixtures/manifest.json"
                    )
                )
                self.assertFalse(
                    agent_runner.check_write_permission(
                        prepared, "benchmarks/calibration/evidence.json"
                    )
                )

    def test_uc001_cannot_create_unlisted_source(self) -> None:
        with tempfile.TemporaryDirectory(prefix="ail-m8c-uc001-test-") as raw:
            prepared = agent_runner.prepare_trial(
                "python", "UC-001", Path(raw) / "workspace"
            )
            self.assertFalse(
                agent_runner.check_write_permission(
                    prepared, "benchmarks/baselines/python/v2/domain.py"
                )
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

    def test_loopback_recorder_preflights_forwards_and_reconciles(self) -> None:
        body = {
            "model": "gpt-5.6-sol",
            "instructions": "locked",
            "input": [
                {"role": "user", "content": "task"},
                {
                    "type": "function_call",
                    "call_id": "call-1",
                    "name": "exec_command",
                    "arguments": json.dumps({"cmd": "pytest -q"}),
                },
                {
                    "type": "function_call_output",
                    "call_id": "call-1",
                    "output": "passed",
                },
            ],
            "stream": True,
        }
        with FakeResponsesUpstream() as upstream:
            with responses_recorder.LoopbackResponsesRecorder(
                upstream_base_url=upstream.url,
                upstream_api_key="upstream-secret",
                client_token="client-secret",
                prompt_cache_key="trial-cache",
            ) as recorder:
                request = urllib.request.Request(
                    f"{recorder.url}/responses",
                    data=json.dumps(body).encode(),
                    method="POST",
                    headers={
                        "Authorization": "Bearer client-secret",
                        "Content-Type": "application/json",
                    },
                )
                with urllib.request.urlopen(request, timeout=10) as response:
                    self.assertIn(b"response.completed", response.read())
        self.assertIsNone(recorder.failure)
        self.assertEqual(len(recorder.requests), 1)
        recorded = recorder.requests[0]
        self.assertEqual(
            recorded["preflight_input_tokens"],
            recorded["provider_input_tokens"],
        )
        self.assertEqual(
            sum(recorded["categories"].values())
            + recorded["protocol_overhead_tokens"],
            recorded["provider_input_tokens"],
        )
        self.assertGreater(recorded["categories"]["build_and_test_output"], 0)
        self.assertEqual(recorded["redaction"], "authorization-and-cookies-removed")

    def test_loopback_recorder_denies_before_forwarding_at_token_limit(self) -> None:
        with FakeResponsesUpstream() as upstream:
            with responses_recorder.LoopbackResponsesRecorder(
                upstream_base_url=upstream.url,
                upstream_api_key="upstream-secret",
                client_token="client-secret",
                token_limit=1,
            ) as recorder:
                request = urllib.request.Request(
                    f"{recorder.url}/responses",
                    data=json.dumps(
                        {"model": "gpt-5.6-sol", "input": [], "stream": True}
                    ).encode(),
                    method="POST",
                    headers={
                        "Authorization": "Bearer client-secret",
                        "Content-Type": "application/json",
                    },
                )
                with self.assertRaises(Exception) as caught:
                    urllib.request.urlopen(request, timeout=10)
                self.assertIn("HTTP Error 429", str(caught.exception))
        self.assertTrue(recorder.limit_reached)
        self.assertEqual(recorder.requests, [])

    def test_pinned_codex_uses_strict_config_and_live_recorder_path(self) -> None:
        codex = shutil.which("codex")
        if codex is None:
            self.skipTest("pinned Codex executable is unavailable")
        codex_path = Path(codex)
        try:
            agent_runner._verify_codex_binary(codex_path)
        except agent_runner.RunnerError:
            self.skipTest("installed Codex executable does not match the M8 pin")
        with tempfile.TemporaryDirectory(prefix="ail-m8-live-path-") as raw:
            with FakeResponsesUpstream() as upstream:
                bundle = agent_runner.run_live_trial(
                    language="python",
                    task="UC-001",
                    destination=Path(raw) / "trial",
                    api_key="fake-upstream-key",
                    codex_path=codex_path,
                    tool_path=os.environ.get("PATH", ""),
                    upstream_base_url=upstream.url,
                )
                verified = agent_runner.verify_live_trial_bundle(
                    Path(raw) / "trial" / "live-trial.json"
                )
        self.assertEqual(
            bundle["terminal"], {"class": "successful", "cause": "complete"}
        )
        self.assertEqual(verified, bundle)
        self.assertEqual(bundle["recorder"]["request_count"], 1)
        self.assertIsNone(bundle["recorder"]["failure"])


if __name__ == "__main__":
    unittest.main()
