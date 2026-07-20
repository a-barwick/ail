# ADR 0002: Freeze the M8 agent experiment contract

- Status: Accepted
- Date: 2026-07-19
- Owners: project maintainers
- Documentation layer and scope: baseline benchmark governance for M8 agent
  trials

## Context

M8 measures the work required for an agent to complete UC-001 and UC-003 in
Rust, Go, Python, and TypeScript. The comparison is useful only if all eight
configurations receive the same agent treatment and if every model input, tool
result, edit, validation, repair, limit, and final correctness result is
reviewable.

This decision serves accepted requirements
[NFR-001 through NFR-003](../requirements/reference-slice.md#nfr-001--reproducible-comparative-benchmark).
It does not select AIL syntax, an AIL target, or a production compiler stack.

M7 supplies eight locked, answer-free
[task starts](../../benchmarks/task-starts/README.md). The frozen task text,
public tests, language tools, source tree, and later private correctness oracle
are inputs to this decision and do not change here.

### Readiness evidence

The preserved `codex/m8-baseline-calibration` pilot is non-authoritative and
counts as zero M8 trials. It nevertheless answers two configuration questions:

| Pilot | Agent treatment | Input tokens | Visible result | Use in this decision |
| --- | --- | ---: | --- | --- |
| Python UC-001/001 | Codex 0.145.0-alpha.18, GPT-5.6 Sol, High, interactive tools | 291,581 | visible checks passed | proves 100,000 cumulative input tokens is too low |
| Python UC-001/002 | Codex 0.145.0-alpha.18, GPT-5.6 Terra, Low, interactive tools | 273,018 | visible checks passed | confirms the excess was not specific to Sol reasoning |
| Later one-pass pilots | prompt-contained source with no agent tools | 13,694–17,611 | mixed | rejected because they cannot measure discovery, validation, or repair |

The first two records have SHA-256 digests
`781a707613b504d5e49f1948b8a823a1914fffb651c93f697a801b9f6bafd1ea`
and
`ff64941e801cfaeb4fa3b085758a29e1efdc1e94437e7660ccbdc6ab4cb9efa7`.
They remain pilot evidence outside the official campaign.

The measured candidate uses the installed stable Codex 0.144.6 binary rather
than the pilot's app-bundled alpha binary. The pilot establishes protocol and
limit evidence only; M8f must establish readiness for the exact selected
binary across all eight configurations.

Current OpenAI guidance identifies `gpt-5.6-sol` as the explicit flagship model
for complex agent work, recommends fixing reasoning effort, and recommends
direct tool use for adaptive workflows. The sources reviewed for this decision
were the
[GPT-5.6 Sol upgrade guide](https://developers.openai.com/api/docs/guides/upgrading-to-gpt-5p6-sol),
[GPT-5.6 prompting guide](https://developers.openai.com/api/docs/guides/prompt-guidance-gpt-5p6),
[Codex non-interactive-mode documentation](https://learn.chatgpt.com/docs/non-interactive-mode),
and
[Codex permission-profile documentation](https://learn.chatgpt.com/docs/permissions).

## Decision

### Controlled treatment

Every M8 agent trial uses this treatment:

| Variable | Frozen value |
| --- | --- |
| Provider | OpenAI through the Codex Responses transport |
| Model request | explicit `gpt-5.6-sol`, never the `gpt-5.6` family alias |
| Reasoning | `high`; standard mode, not Pro |
| Agent | one root OpenAI Codex CLI agent |
| Agent executable | `codex-cli 0.144.6` |
| Candidate executable SHA-256 | `80a3933d11a9d13ef806aa24f7bb8afc9169cfe4e9b09d6da6a92922cbde9cff` |
| Model turns | one fresh thread with adaptive local tool calls; no resume or human follow-up |
| Delegation | disabled; no subagents |
| Sampling | temperature, top-p, and seed are not exposed by this Codex surface and remain unavailable |
| Response detail | medium verbosity; no reasoning summary requested |
| Web and external tools | disabled |
| Agent wall limit | 600 seconds |
| Cumulative input-token limit | 500,000 tokens, counting cached delivery |
| Trial retries | none |

The CLI binary is an external locked artifact. M8f must recheck its version and
digest before freezing the campaign. An updated or differently hashed binary is
a new candidate configuration, not an equivalent replacement.

The requested model ID, provider-returned model ID, service tier when exposed,
reasoning effort, Codex version, and executable digest are recorded on every
trial. OpenAI does not expose an immutable backend snapshot through this Codex
surface. The manifest records that field as unavailable instead of inventing a
version. A response identifying a model outside the explicit Sol request is a
configuration rejection before official collection and incomplete evidence if
detected after a trial starts.

No temperature, top-p, or seed value is inferred. Their absence is recorded in
the run manifest. Model output is therefore stochastic, and the required
independent trials measure a distribution rather than promising identical
generated text.

### Trial state machine

One trial has seven ordered phases:

1. **Pre-start gate.** Verify the campaign configuration, host identity, agent
   executable, model request, prompt, task-start tree, protected files,
   toolchains, private-package digest, recorder, permissions, limits, and
   evidence destinations.
2. **Materialize.** Build one fresh M7 task start in a new isolated directory,
   provision its already locked offline dependencies, initialize a local Git
   repository, and verify the starting-state classification.
3. **Start.** Write the locked run manifest and `trial.started` event, then
   spawn one Codex process. This process boundary is the start boundary from
   the frozen M2 classification contract.
4. **Agent loop.** Deliver one prompt. The agent may inspect, edit, format,
   compile, statically analyze, build, and test through recorded local tools.
   It may repair its work after visible failures.
5. **Stop.** End on normal Codex exit, the wall limit, the input-token limit, a
   permission violation, or an unrecoverable process or provider failure.
6. **Verify.** Freeze the final source digest and run the public and private
   correctness oracle outside the agent-readable workspace. The agent receives
   no hidden result and gets no post-oracle repair turn.
7. **Classify.** Store raw evidence, the final tree, checks, token accounting,
   edits, validations, repair cycles, limits, permission results, and one
   terminal classification.

Trials run sequentially on the reference host. One trial never resumes another
thread, shares a workspace, or receives a human message.

### Prompt construction

The runner reads the protected root `TASK.md` from the selected task start as
UTF-8. The pre-start gate requires LF line endings and one final LF. The prompt
is the following exact prefix, the exact task bytes, and the exact suffix:

```text
Complete the frozen task below in the supplied answer-free workspace.

Work as one coding agent using the available local tools. Inspect the workspace,
make the required source changes, and run the relevant visible formatter,
static-analysis, build, and test checks. You may iterate after a failed visible
check. Do not delegate to another agent, use the network, install dependencies,
read or write outside the workspace, or change protected task, test, fixture,
contract, or tool-configuration files. There is no human follow-up during the
trial. Stop when the task is complete or a locked limit prevents further work.
In the final message, summarize the source changes and checks run; do not claim
that unavailable hidden checks passed.

--- BEGIN FROZEN TASK ---
{TASK_TEXT_BYTES}--- END FROZEN TASK ---
```

`{TASK_TEXT_BYTES}` is substitution notation, not literal prompt text. Because
the frozen task ends in LF, the suffix begins on the next line. No path,
language-specific hint, curated affected-file list, source, test result, or
reference answer is appended. M8b locks the rendered prompt digest separately
for UC-001 and UC-003.

### Initial context

The first model request contains only:

- the version-bound Codex base instructions and built-in local tool schemas;
- the exact prompt above, including the frozen task text; and
- minimal protocol metadata added by the fixed Codex transport.

No source file, test file, fixture, directory listing, prior conversation,
memory, global `AGENTS.md`, skill, plugin, MCP server, connector, browser,
image, or language-specific hint is preloaded. Source and test content enters
model context only when the agent reads it through a recorded tool.

The trial uses an isolated `CODEX_HOME` containing only the frozen trial
configuration and ephemeral authentication material. Personal configuration,
instructions, history, rules, memories, plugins, skills, and MCP configuration
are absent. Authentication secrets are never copied into agent-readable paths
or evidence.

### Interactive Codex protocol

The runner invokes the pinned executable directly, without a shell, with the
semantic equivalent of:

```text
codex exec
  --json
  --ephemeral
  --strict-config
  --color never
  -C <fresh-task-workspace>
  -m gpt-5.6-sol
  -
```

The exact prompt is supplied on standard input. There is no output schema and
no one-pass generated-file response. JSONL events, the final message, process
output, and process-group status are captured separately.

The isolated configuration fixes:

- `model_reasoning_effort = "high"`;
- `model_reasoning_summary = "none"`;
- `model_verbosity = "medium"`;
- `personality = "none"`;
- `approval_policy = "never"`;
- a generated least-privilege permission profile;
- `web_search = "disabled"`;
- no history persistence or startup update check;
- one agent thread and no child-agent depth;
- a Responses provider routed through the loopback recorder;
- zero request and stream retries; and
- a 300-second stream idle timeout inside the 600-second trial limit.

M8b and M8c materialize and validate the exact configuration. Unknown keys,
unavailable permissions, an enabled extra tool, a recorder bypass, or a
non-zero internal retry setting reject the configuration.

### Available local tools

All configurations receive the same Codex shell and file-edit interface,
read-only discovery utilities, Git status and diff, and their locked normal
language tools. Every invocation and complete output is recorded.

| Language | Locked normal tools |
| --- | --- |
| Rust | rustc 1.88.0, Cargo 1.88.0, rustfmt 1.8.0, Clippy 0.1.88, rust-analyzer 1.88.0 |
| Go | Go 1.26.0, gofmt from Go 1.26.0, `go vet` 1.26.0, gopls 0.21.1 |
| Python | CPython 3.13.5, uv 0.7.12, mypy 1.17.0, Ruff 0.12.4, pytest 8.4.1 |
| TypeScript | Node.js 23.10.0, npm 10.9.2, TypeScript 5.8.3, ESLint 9.31.0, Prettier 3.6.2, tsx 4.20.3, c8 10.1.3 |

Dependencies are provisioned before the trial and used offline. The agent
cannot install, update, or retrieve a dependency. A missing dependency or
unusable locked tool discovered before Codex starts is a configuration
rejection. The same problem after start is a failed trial and remains evidence.

Language-server output requested only for inspection is semantic tool output.
A compiler, static-analysis, build, or test action that can establish
completion is a validation attempt under the frozen M2 definition.

### Filesystem, subprocess, and network permissions

The generated permission profile grants:

- read access to the minimal platform and locked toolchain runtime paths;
- read access to the complete task workspace;
- write access only to task-start files marked `editable` and workspace-local
  build, cache, and temporary paths derived from the task-start manifest;
- no read access to the parent AIL repository, other task starts, reference
  implementations, private package, evidence, recorder state, credentials, or
  another trial;
- no write access to protected task, test, fixture, contract, dependency-lock,
  or tool-configuration files; and
- no network access for agent-created subprocesses.

Subprocesses are recorded rather than silently treated as allowlisted. The
fixed permission profile and isolated environment remain the authority: an
attempt to execute an unavailable program, read outside the granted roots,
write a protected path, open a network connection, or bypass the recorder is a
`permission_violation`.

The Codex control process may reach OpenAI only through a loopback recording
proxy. That control-plane connection is not exposed to agent tools. The proxy
redacts authorization, cookies, and transport secrets before hashing or
storing evidence. Any other external access is denied and recorded.

The subprocess environment starts from an allowlist rather than the host
environment. It contains only fixed locale and time-zone values, the locked
tool `PATH`, workspace-local home/cache/temp paths, and variables required by
the selected offline toolchain. No host secret, user configuration, proxy,
credential, or unrelated package path is inherited.

### Model-input capture and token accounting

`codex exec --json` is necessary but not sufficient because its terminal usage
record does not expose every model request or support categorical attribution.
The Codex Responses connection therefore passes through a loopback recorder
that captures every redacted request body, streamed response item, usage
record, tool delivery, and retry boundary before forwarding.

Each trial uses a unique prompt-cache key derived from its locked trial ID.
This permits normal within-trial caching while preventing cross-trial cache
reuse from depending on campaign order. Cached input tokens are recorded as a
subset and are never deducted from total input.

Provider-reported request usage is authoritative for total input tokens. The
recorder also tokenizes every delivered input item with the frozen tokenizer
for the provider-returned model and assigns it once to these top-level
categories:

- `initial_context`: Codex instructions, tool schemas, prompt, task text,
  protocol metadata, and model-produced conversation state replayed on later
  requests;
- `source_reads`: file contents and search results from workspace source,
  tests, and public fixtures;
- `semantic_tool_output`: non-validating language-server, type, symbol, or
  dependency inspection;
- `diagnostics`: standalone tool, sandbox, permission, or structured
  diagnostic output that is not part of a validation action;
- `build_and_test_output`: all output from a validation attempt, including
  diagnostics emitted by that action; and
- `other_tool_output`: directory listings, Git inspection, edit
  acknowledgements, formatter-only output, and other recorded local-tool
  results.

M8b may add subcategories, including fixed instructions, task text, agent
history, and individual tool classes, but those roll up without overlap to the
six categories above. Model-produced history is counted when the provider
receives it again. Repeated instructions, task text, source, or tool output are
counted on every delivery. Provider cached-input counts remain a separate
directional measure.

For each model request, categorical tokenizer totals plus protocol overhead
must reconcile to provider-reported input usage under a frozen rule. Missing
requests, opaque unaccounted input, a tokenizer mismatch, or an unreconciled
total produces `incomplete_evidence`; it cannot be a successful trial. The
exact tokenizer implementation and reconciliation tolerance must pass M8b
synthetic checks and M8f readiness before official collection.

### Limits and termination

The accepted NFR-002 input safety limit is amended from 100,000 to **500,000
cumulative delivered input tokens per trial**. The total includes cached input
and every repeated delivery. It is a runaway-work bound, not an AIL success
target.

Before forwarding each model request, the recorder uses the frozen tokenizer
to determine whether that request would raise the cumulative total above
500,000. If so, it does not forward the request, terminates the agent, and
classifies the trial `input_token_limit`. A tokenizer or recorder that cannot
enforce this check rejects the configuration before the trial starts.

The wall limit is 600 seconds measured with the host monotonic clock from
`trial.started` through Codex process exit. Post-run correctness verification
has its separately locked functional limits and does not consume agent wall
time.

On a wall timeout or enforced stop, the runner sends `SIGTERM` to the complete
trial process group, waits five seconds, then sends `SIGKILL` if any process
remains. Partial events, files, output, and usage remain evidence.

There is no automatic trial retry, response regeneration, human follow-up, or
session resume. The recorder provider sets request and stream retries to zero.
A configuration rejection may be corrected and rerun under the same scheduled
trial identity because no trial started. A started failure or timeout is
append-only; any replacement uses a new trial identity under the balanced
make-up procedure.

### Terminal classifications

The terminal classes are mutually exclusive:

| Class | Start boundary crossed? | Meaning |
| --- | --- | --- |
| `configuration_rejection` | no | A lock, required field, tool, host, credential, recorder, tokenizer, permission, model, or starting-state gate failed before Codex was spawned. It is not a trial. |
| `failed` | yes | The run ended without complete success for a non-timeout reason, including non-zero agent exit, input-token limit, permission violation, protected change, incomplete evidence, or any public/private correctness failure. |
| `timed_out` | yes | The 600-second agent wall limit or a separately locked functional timeout expired. It is retained as a failed trial and reported separately. |
| `successful` | yes | The agent ended within limits, permissions and protected artifacts remained valid, evidence is complete, and every public/private final-revision check passed. |

Stable detailed causes remain available beneath `failed` and `timed_out`.
`input_token_limit` is a failed cause, not a timeout. A Codex zero exit, a
plausible final message, visible tests, or a low token count cannot establish
success without the complete post-run oracle and matching final-revision
evidence.

Agent-requested visible validation attempts and repair cycles are counted under
the frozen
[run-classification contract](../../benchmarks/contracts/run-classification.md).
Harness setup, automatic packaging, and the post-run public/private oracle are
not agent edits, validation attempts, or repair cycles.

### Reference environment

The candidate reference environment is the dedicated native macOS host
`ail-m8-reference-mac-01`:

- macOS 26.5.2, build 25F84;
- Apple arm64;
- no container;
- the four M7-locked toolchains and dependency trees;
- the pinned Codex executable described above; and
- sequential trials with no concurrent M8 work.

M8f records and locks the non-secret hardware profile, OS build, executable
and package digests, environment allowlist, tool versions, authentication
method, recorder version, and balanced order. Serial numbers, credentials, and
personal paths are excluded. A host, OS, tool, agent, or recorder mismatch
rejects the configuration.

### M8f readiness gate

This is the selected candidate contract, but no official evidence may exist
before M8f. M8f must demonstrate at least one non-official provider-backed
successful trial with zero-difference token reconciliation and at least one
provider-backed safety-limit classification. The pinned-agent fake-upstream
integration, all eight task-start gates, all eight M8e warm/cold pilots, and the
calibration verifier must also pass.

Per-configuration agent success is not duplicated as a readiness prerequisite.
The official campaign records every success, failure, timeout, and limit
classification and uses the accepted make-up rules to reach the required
successful counts. Token mismatch, incomplete isolation, an unverifiable model
or agent identity, or a failing deterministic evidence path still blocks M8f.

This paragraph is the reviewed 2026-07-19 amendment authorized by the benchmark
owner after live readiness showed that UC-003 could correctly reach the
500,000-token safety limit while the provider, recorder, permissions, and
evidence paths remained healthy.

## Consequences

- The benchmark measures normal source discovery, validation, and repair
  instead of measuring one-pass file generation.
- Sol at High reasoning is a quality-first strong baseline. It is slower and
  more expensive than Terra or lower reasoning, but one fixed treatment avoids
  model selection becoming a language-specific confound.
- Exact request capture and a unique per-trial cache key make repeated context,
  cached input, and categorical token accounting auditable.
- Least-privilege filesystem rules keep completed references and private cases
  outside the agent's readable boundary instead of relying on prompt
  compliance.
- The 500,000-token cap increases the permitted work over the original pilot
  rule while retaining a deterministic runaway bound.
- Hosted model output cannot be bit-reproduced and the immutable backend
  snapshot is unavailable. The campaign records this limitation, locks every
  controllable input, and measures distributions.
- The capture proxy, exact tokenizer reconciliation, and permission profile are
  implementation obligations for M8b and M8c. This decision does not implement
  them.

## Alternatives considered

- **Keep the 100,000-token limit:** rejected because both ordinary interactive
  readiness runs exceeded it while completing visible work.
- **Use the one-pass pilot protocol:** rejected because it preloads selected
  context, prevents tool-driven discovery and validation, and cannot count
  normal repair cycles.
- **Use GPT-5.6 Terra or lower reasoning:** credible for a cost-oriented
  baseline, but rejected because this first control group is intended to be a
  strong quality baseline and the accepted execution plan recommended Sol at
  High.
- **Use Max or Ultra:** rejected because Max increases work without readiness
  evidence and Ultra changes the treatment to multi-agent execution.
- **Allow personal Codex configuration and approvals:** rejected because
  instructions, plugins, user decisions, and approval latency would vary across
  trials.
- **Use `codex exec --json` without a request recorder:** rejected because the
  terminal aggregate cannot prove complete model-input capture or categorical
  token attribution.
- **Give the agent read access to the full AIL checkout:** rejected because it
  exposes other languages, reference implementations, hidden metadata, and
  pilot artifacts.
- **Retry transient failures automatically:** rejected because hidden retry
  policy changes time, token, and success distributions. Every started attempt
  remains visible.

## Validation

M8a is validated by:

```bash
python3 tools/check_docs.py
```

M8b must turn this decision into schemas, canonical configuration artifacts,
digest locks, and synthetic acceptance/rejection fixtures. M8c must prove the
agent loop, recorder, token reconciliation, limits, process termination,
permissions, edit/validation/repair accounting, and classifications with fake
or dry streams. M8d must prove final public/private correctness and replay.

M8f is the empirical infrastructure acceptance gate for this decision.
Official collection cannot begin until the amended representative live gate
and every deterministic calibration verifier check pass.
