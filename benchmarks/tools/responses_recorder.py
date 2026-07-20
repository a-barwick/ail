#!/usr/bin/env python3
"""Loopback-only Responses proxy with exact input-token reconciliation."""

from __future__ import annotations

import copy
import json
import socketserver
import threading
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any


TOKEN_CATEGORIES = (
    "initial_context",
    "source_reads",
    "semantic_tool_output",
    "diagnostics",
    "build_and_test_output",
    "other_tool_output",
)
TOKEN_BASELINE_INPUT = [{"role": "user", "content": ""}]
TOKEN_COUNT_FIELDS = (
    "conversation",
    "input",
    "instructions",
    "model",
    "parallel_tool_calls",
    "previous_response_id",
    "reasoning",
    "style",
    "text",
    "tool_choice",
    "tools",
    "truncation",
)


class RecorderFailure(Exception):
    pass


class LoopbackHTTPServer(ThreadingHTTPServer):
    daemon_threads = True

    def server_bind(self) -> None:
        # HTTPServer's reverse-DNS lookup is unnecessary for a fixed loopback bind.
        socketserver.TCPServer.server_bind(self)
        host, port = self.server_address[:2]
        self.server_name = str(host)
        self.server_port = int(port)


def _json_bytes(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def _input_token_count(payload: dict[str, Any]) -> int:
    value = payload.get("input_tokens")
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        raise RecorderFailure("input-token response omitted a nonnegative input_tokens")
    return value


def _tool_commands(items: list[Any]) -> dict[str, str]:
    commands: dict[str, str] = {}
    for item in items:
        if not isinstance(item, dict) or item.get("type") not in {
            "function_call",
            "custom_tool_call",
        }:
            continue
        call_id = item.get("call_id")
        if not isinstance(call_id, str):
            continue
        if item.get("type") == "custom_tool_call":
            custom_input = item.get("input")
            if isinstance(custom_input, str):
                commands[call_id] = custom_input
            continue
        arguments = item.get("arguments")
        parsed: Any = {}
        if isinstance(arguments, str):
            try:
                parsed = json.loads(arguments)
            except json.JSONDecodeError:
                parsed = {}
        if isinstance(parsed, dict):
            command = parsed.get("cmd") or parsed.get("command")
            if isinstance(command, str):
                commands[call_id] = command
    return commands


def _tool_output_category(command: str) -> str:
    lowered = command.lower()
    validation_markers = (
        "cargo test",
        "cargo check",
        "cargo clippy",
        "go test",
        "go vet",
        "pytest",
        "mypy",
        "ruff check",
        "npm test",
        "npm run",
        "npx tsc",
        "tsc ",
        "check_docs.py",
        "verify-all",
    )
    if any(marker in lowered for marker in validation_markers):
        return "build_and_test_output"
    semantic_markers = ("rust-analyzer", "gopls", "pyright", "typescript-language")
    if any(marker in lowered for marker in semantic_markers):
        return "semantic_tool_output"
    stripped = lowered.lstrip()
    if stripped.startswith(("cat ", "sed ", "rg ", "grep ", "head ", "tail ")):
        return "source_reads"
    if any(marker in lowered for marker in ("--diagnostic", "diagnostics", "explain")):
        return "diagnostics"
    return "other_tool_output"


def classify_input_item(item: Any, commands: dict[str, str]) -> str:
    """Assign one wire input item to the frozen top-level category."""

    if not isinstance(item, dict):
        return "initial_context"
    if item.get("type") in {"function_call_output", "custom_tool_call_output"}:
        call_id = item.get("call_id")
        command = commands.get(call_id, "") if isinstance(call_id, str) else ""
        return _tool_output_category(command)
    # User/task input and model-produced replay are both initial context.
    return "initial_context"


def _input_groups(
    items: list[Any], commands: dict[str, str]
) -> list[tuple[list[Any], str]]:
    """Keep serial tool calls and their required outputs in valid count prefixes."""

    groups: list[tuple[list[Any], str]] = []
    index = 0
    while index < len(items):
        item = items[index]
        if not isinstance(item, dict) or item.get("type") not in {
            "function_call",
            "custom_tool_call",
        }:
            groups.append(([item], classify_input_item(item, commands)))
            index += 1
            continue

        call_id = item.get("call_id")
        output_type = (
            "custom_tool_call_output"
            if item.get("type") == "custom_tool_call"
            else "function_call_output"
        )
        if (
            not isinstance(call_id, str)
            or index + 1 >= len(items)
            or not isinstance(items[index + 1], dict)
            or items[index + 1].get("type") != output_type
            or items[index + 1].get("call_id") != call_id
        ):
            raise RecorderFailure(
                "serial tool call is not followed by its required output"
            )
        output = items[index + 1]
        groups.append(([item, output], classify_input_item(output, commands)))
        index += 2
    return groups


class LoopbackResponsesRecorder:
    """Record, preflight, enforce, and forward one serial Codex Responses stream."""

    def __init__(
        self,
        *,
        upstream_base_url: str,
        upstream_api_key: str,
        client_token: str,
        prompt_cache_key: str | None = None,
        token_limit: int = 500_000,
    ):
        if not upstream_api_key:
            raise RecorderFailure("upstream API key is empty")
        if not client_token:
            raise RecorderFailure("recorder client token is empty")
        self.upstream_base_url = upstream_base_url.rstrip("/")
        self.upstream_api_key = upstream_api_key
        self.client_token = client_token
        self.prompt_cache_key = prompt_cache_key
        self.token_limit = token_limit
        self.total_input_tokens = 0
        self.requests: list[dict[str, Any]] = []
        self.failure: str | None = None
        self.limit_reached = False
        self._lock = threading.Lock()
        self._server: LoopbackHTTPServer | None = None
        self._thread: threading.Thread | None = None

    @property
    def url(self) -> str:
        if self._server is None:
            raise RecorderFailure("recorder is not running")
        return f"http://127.0.0.1:{self._server.server_port}/v1"

    def __enter__(self) -> "LoopbackResponsesRecorder":
        owner = self

        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def log_message(self, _format: str, *_args: Any) -> None:
                return

            def do_POST(self) -> None:
                owner._handle(self)

        self._server = LoopbackHTTPServer(("127.0.0.1", 0), Handler)
        self._thread = threading.Thread(
            target=self._server.serve_forever,
            name="m8-responses-recorder",
            daemon=True,
        )
        self._thread.start()
        return self

    def __exit__(self, *_args: Any) -> None:
        if self._server is not None:
            self._server.shutdown()
            self._server.server_close()
        if self._thread is not None:
            self._thread.join(timeout=5)

    def _authorize(self, handler: BaseHTTPRequestHandler) -> bool:
        expected = f"Bearer {self.client_token}"
        if handler.headers.get("Authorization") == expected:
            return True
        self._send_json(
            handler, 401, {"error": {"message": "recorder authorization failed"}}
        )
        return False

    def _read_body(self, handler: BaseHTTPRequestHandler) -> dict[str, Any]:
        raw_length = handler.headers.get("Content-Length")
        try:
            length = int(raw_length or "")
        except ValueError as error:
            raise RecorderFailure("invalid Content-Length") from error
        if length <= 0 or length > 32 * 1024 * 1024:
            raise RecorderFailure("request body length is outside recorder bounds")
        try:
            value = json.loads(handler.rfile.read(length))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise RecorderFailure(f"invalid request JSON: {error}") from error
        if not isinstance(value, dict):
            raise RecorderFailure("request JSON must be an object")
        return value

    def _upstream(
        self, endpoint: str, body: dict[str, Any]
    ) -> tuple[int, dict[str, str], bytes]:
        request = self._upstream_request(endpoint, body)
        try:
            response = urllib.request.urlopen(request, timeout=320)
        except urllib.error.HTTPError as error:
            return error.code, dict(error.headers.items()), error.read()
        with response:
            return response.status, dict(response.headers.items()), response.read()

    def _upstream_request(
        self, endpoint: str, body: dict[str, Any]
    ) -> urllib.request.Request:
        return urllib.request.Request(
            f"{self.upstream_base_url}{endpoint}",
            data=_json_bytes(body),
            method="POST",
            headers={
                "Authorization": f"Bearer {self.upstream_api_key}",
                "Content-Type": "application/json",
                "Accept": "text/event-stream, application/json",
            },
        )

    def _count(self, body: dict[str, Any]) -> int:
        # The public input-token endpoint intentionally accepts only the
        # documented token-count request schema, which is narrower than the
        # Responses create schema. Response-only controls such as include,
        # store, stream, and prompt_cache_key remain on the forwarded request.
        count_body = {
            key: copy.deepcopy(body[key])
            for key in TOKEN_COUNT_FIELDS
            if key in body
        }
        status, _headers, raw = self._upstream(
            "/responses/input_tokens", count_body
        )
        if status != 200:
            raise RecorderFailure(
                f"input-token preflight returned HTTP {status}: "
                f"{raw[:512].decode('utf-8', 'replace')}"
            )
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError as error:
            raise RecorderFailure(
                "input-token preflight returned invalid JSON"
            ) from error
        if not isinstance(payload, dict):
            raise RecorderFailure("input-token preflight returned a non-object")
        return _input_token_count(payload)

    def probe(self, model: str) -> int:
        """Prove that the live input-token endpoint is reachable before spawn."""

        return self._count({"model": model, "input": TOKEN_BASELINE_INPUT})

    def _account(self, body: dict[str, Any]) -> dict[str, Any]:
        items = body.get("input", [])
        if isinstance(items, str):
            items = [{"role": "user", "content": items}]
        if not isinstance(items, list):
            raise RecorderFailure("Responses input must be a string or array")

        # The live endpoint rejects an empty input array. A single empty user
        # item is therefore the frozen protocol baseline; cumulative prefix
        # deltas still telescope exactly to the delivered request total.
        skeleton = {"model": body.get("model"), "input": TOKEN_BASELINE_INPUT}
        protocol_overhead = self._count(skeleton)
        fixed = copy.deepcopy(body)
        fixed["input"] = TOKEN_BASELINE_INPUT
        previous = self._count(fixed)
        if previous < protocol_overhead:
            raise RecorderFailure(
                "fixed request token count is below protocol overhead"
            )
        categories = {category: 0 for category in TOKEN_CATEGORIES}
        categories["initial_context"] = previous - protocol_overhead
        commands = _tool_commands(items)
        prefix_items: list[Any] = []
        for group, category in _input_groups(items, commands):
            prefix_items.extend(group)
            prefix = copy.deepcopy(body)
            prefix["input"] = prefix_items
            current = self._count(prefix)
            if current < previous:
                raise RecorderFailure("cumulative prefix token count decreased")
            categories[category] += current - previous
            previous = current
        return {
            "preflight_input_tokens": previous,
            "protocol_overhead_tokens": protocol_overhead,
            "categories": categories,
        }

    def _handle(self, handler: BaseHTTPRequestHandler) -> None:
        response_started = False
        try:
            if not self._authorize(handler):
                return
            if handler.path != "/v1/responses":
                self._send_json(
                    handler, 404, {"error": {"message": "unsupported recorder path"}}
                )
                return
            body = self._read_body(handler)
            # Pinned Codex adds local client metadata understood by its hosted
            # transport but rejected by the public Responses API. It is not
            # model input, so remove it before both preflight and forwarding.
            body.pop("client_metadata", None)
            if self.prompt_cache_key is not None:
                body["prompt_cache_key"] = self.prompt_cache_key
            accounting = self._account(body)
            with self._lock:
                projected = (
                    self.total_input_tokens + accounting["preflight_input_tokens"]
                )
                if projected > self.token_limit:
                    self.limit_reached = True
                    self._send_json(
                        handler,
                        429,
                        {
                            "error": {
                                "message": "M8 cumulative input-token limit reached"
                            }
                        },
                    )
                    return
                request_id = f"request-{len(self.requests) + 1}"

            status, completed = self._forward_response(handler, body)
            response_started = True
            provider_input = self._provider_input_tokens(completed)
            cached_input = self._cached_input_tokens(completed)
            record = {
                "request_id": request_id,
                **accounting,
                "provider_input_tokens": provider_input,
                "cached_input_tokens": cached_input,
                "body": body,
                "response": completed,
                "http_status": status,
                "redaction": "authorization-and-cookies-removed",
            }
            with self._lock:
                self.requests.append(record)
                if provider_input != accounting["preflight_input_tokens"]:
                    self.failure = (
                        f"{request_id} preflight/provider mismatch: "
                        f"{accounting['preflight_input_tokens']} != {provider_input}"
                    )
                else:
                    self.total_input_tokens += provider_input
        except RecorderFailure as error:
            self.failure = str(error)
            if not response_started and not handler.close_connection:
                self._send_json(handler, 502, {"error": {"message": str(error)}})
        except (OSError, urllib.error.URLError) as error:
            self.failure = f"recorder transport failed: {error}"
            if not response_started and not handler.close_connection:
                self._send_json(handler, 502, {"error": {"message": self.failure}})

    def _forward_response(
        self, handler: BaseHTTPRequestHandler, body: dict[str, Any]
    ) -> tuple[int, dict[str, Any]]:
        request = self._upstream_request("/responses", body)
        try:
            response = urllib.request.urlopen(request, timeout=320)
        except urllib.error.HTTPError as error:
            raw = error.read()
            self._send_raw(handler, error.code, dict(error.headers.items()), raw)
            raise RecorderFailure(f"Responses upstream returned HTTP {error.code}")

        headers = dict(response.headers.items())
        content_type = next(
            (value for key, value in headers.items() if key.lower() == "content-type"),
            "",
        )
        if "text/event-stream" not in content_type:
            with response:
                raw = response.read()
            self._send_raw(handler, response.status, headers, raw)
            return response.status, self._completed_response(raw, headers)

        handler.send_response(response.status)
        handler.send_header("Content-Type", content_type)
        handler.send_header("Cache-Control", "no-cache")
        handler.send_header("Connection", "close")
        handler.end_headers()
        handler.close_connection = True
        completed: dict[str, Any] | None = None
        with response:
            while True:
                line = response.readline()
                if not line:
                    break
                handler.wfile.write(line)
                handler.wfile.flush()
                if not line.startswith(b"data:"):
                    continue
                data = line[5:].strip()
                if not data or data == b"[DONE]":
                    continue
                try:
                    event = json.loads(data)
                except json.JSONDecodeError:
                    continue
                if (
                    isinstance(event, dict)
                    and event.get("type") == "response.completed"
                    and isinstance(event.get("response"), dict)
                ):
                    completed = event["response"]
        if completed is None:
            raise RecorderFailure("Responses stream omitted response.completed")
        return response.status, completed

    @staticmethod
    def _completed_response(raw: bytes, headers: dict[str, str]) -> dict[str, Any]:
        content_type = next(
            (value for key, value in headers.items() if key.lower() == "content-type"),
            "",
        )
        if "text/event-stream" not in content_type:
            try:
                payload = json.loads(raw)
            except json.JSONDecodeError as error:
                raise RecorderFailure(
                    "Responses upstream returned invalid JSON"
                ) from error
            if not isinstance(payload, dict):
                raise RecorderFailure("Responses upstream returned a non-object")
            return payload
        completed: dict[str, Any] | None = None
        for line in raw.splitlines():
            if not line.startswith(b"data:"):
                continue
            data = line[5:].strip()
            if data == b"[DONE]":
                continue
            try:
                event = json.loads(data)
            except json.JSONDecodeError:
                continue
            if (
                isinstance(event, dict)
                and event.get("type") == "response.completed"
                and isinstance(event.get("response"), dict)
            ):
                completed = event["response"]
        if completed is None:
            raise RecorderFailure("Responses stream omitted response.completed")
        return completed

    @staticmethod
    def _provider_input_tokens(response: dict[str, Any]) -> int:
        usage = response.get("usage")
        value = usage.get("input_tokens") if isinstance(usage, dict) else None
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            raise RecorderFailure("provider response omitted input token usage")
        return value

    @staticmethod
    def _cached_input_tokens(response: dict[str, Any]) -> int:
        usage = response.get("usage")
        details = usage.get("input_tokens_details") if isinstance(usage, dict) else None
        value = details.get("cached_tokens", 0) if isinstance(details, dict) else 0
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            raise RecorderFailure("provider response has invalid cached token usage")
        return value

    @staticmethod
    def _send_json(
        handler: BaseHTTPRequestHandler, status: int, payload: dict[str, Any]
    ) -> None:
        raw = _json_bytes(payload)
        handler.send_response(status)
        handler.send_header("Content-Type", "application/json")
        handler.send_header("Content-Length", str(len(raw)))
        handler.end_headers()
        handler.wfile.write(raw)

    @staticmethod
    def _send_raw(
        handler: BaseHTTPRequestHandler,
        status: int,
        headers: dict[str, str],
        raw: bytes,
    ) -> None:
        handler.send_response(status)
        for key, value in headers.items():
            if key.lower() in {
                "connection",
                "content-encoding",
                "content-length",
                "transfer-encoding",
            }:
                continue
            handler.send_header(key, value)
        handler.send_header("Content-Length", str(len(raw)))
        handler.send_header("Connection", "close")
        handler.end_headers()
        handler.close_connection = True
        handler.wfile.write(raw)
