# AIL investor deck

Status: **Investor presentation source; claims current through M27**

This document is the editable narrative and talk track for
[`investor-deck.pptx`](investor-deck.pptx). The PowerPoint file imports directly
into Google Slides. It is intentionally VC-first: the market and workflow shift
lead; bounded compiler evidence supports the thesis; comparative economics are
clearly labeled as a hypothesis.

## Open or import

1. In Google Drive, choose **New → File upload** and select
   `docs/investor-deck.pptx`.
2. Open the uploaded file with **Google Slides**.
3. Choose **File → Save as Google Slides** if a native Slides copy is desired.
4. Check the speaker-notes pane. Every slide contains a concise talk track.

The deck uses only editable PowerPoint shapes and text. It has no external font,
image, video, or network dependency. Google Slides may make small line-wrap
adjustments; the wide 16:9 layout and conservative margins are intended to keep
those changes minor.

Regenerate it with:

```bash
python3 -m pip install --user 'python-pptx>=1.0,<2'
python3 tools/build_investor_deck.py
```

## Narrative choices

- **Category before features.** Code generation is becoming abundant; trusted
  change remains expensive.
- **Workflow before syntax.** The central visual contrasts today’s
  reconstruct-and-repair loop with query-change-prove.
- **Control layer, not another agent.** AIL provides language guarantees and a
  revisioned semantic compiler interface usable by different models.
- **Two concrete proofs.** The schema-evolution example demonstrates exact
  pre-edit impact. The architecture example demonstrates that behavior-passing
  changes can still be rejected and rolled back for explicit project-policy
  violations.
- **No invented economics.** Repository fixture counts are presented as bounded
  technical evidence. No speedup, ROI, market-size, or productivity number is
  claimed.
- **Falsifiable ask.** Funding supports a locked comparison against strong
  mainstream-language tooling, assessor-independent review, and explicit
  go/revise/stop gates.

## Slide-by-slide source and talk track

### 1. The control layer for software built by agents

AIL is an executable language and semantic compiler designed to reduce the work
between generated code and a trusted change. The mechanism is demonstrated on a
bounded slice; comparative economics remain to be proven.

### 2. Code generation is becoming abundant

The expensive work shifts to finding context, understanding consequences,
repairing failures, controlling regressions, and producing review evidence. The
chart is visibly labeled as conceptual, not measured market data.

### 3. Today’s agents repeatedly reconstruct the program

Mainstream compilers, language servers, refactors, search, and tests already
expose substantial semantics. Even with those normal strong tools, an agent
often still assembles a file-oriented view and probabilistically infers parts of
the change surface. The reconstruction repeats after diagnostics and during
review. AIL must demonstrate an improvement over that strong baseline, not raw
text editing.

### 4. AIL turns language guarantees into an agent control plane

Canonical text stays durable and auditable. The compiler exposes revision-bound
inspection, supported fixture-exact impact within declared coverage, authority,
effects, policy, and atomic publication. The language creates the guarantees;
the protocol exposes them.

### 5. From reconstruct-and-repair to query-change-prove

AIL does not remove agent judgment. It moves mechanically knowable work out of
model inference: identify affected roles, inspect authority and policy, validate
the whole candidate, and publish or roll back.

### 6. A real compiler already closes bounded change loops

The authoritative Rust compiler performs lossless parsing, canonical formatting,
static checks, deterministic interpretation, immutable revisions, semantic
impact, whole-workspace transactions, and architecture-policy enforcement.

The deck uses these repository facts:

- 37 public AIL behavior fixtures;
- 12 semantic relationship kinds in the UC-003 impact graph;
- 23 executable M24 architecture scenarios; and
- four behavior-equivalent mainstream-language reference implementations.

These are bounded conformance and mechanism evidence, not comparative
agent-efficiency or performance evidence.

### 7. Before editing, the compiler returns the bounded change surface

For the locked required-priority evolution, the compiler derives exactly 12
`must_change` locations, two reasoned review sites, and one known external
boundary. The successful transaction contains five ordered whole-path edits. It
also reports that capabilities, effects, and effect ordering are unchanged.

### 8. Passing behavior evidence is necessary. It is not sufficient.

Three frozen `CancelJob` semantic candidates carry the same accepted six-of-six
behavior evidence. The compiler transaction evaluates their validated graphs
against locked policy. The domain-owned candidate publishes with no findings.
The centralized candidate is rolled back with four findings. The superficial
helper split is rolled back with three findings because aggregate transport
authority, state, and dependency boundaries remain violated. This is a bounded
transaction proof, not yet a complete source-to-agent product loop.

The compiler reports primitive facts against explicit project policy. It does
not infer a universal architecture score.

### 9. The candidate revision is the unit of trust

The compiler validates a whole candidate against one immutable base. A success
publishes one child with completion evidence bound to that revision. A denied or
incomplete change publishes nothing and leaves the base unchanged.

### 10. Models improve. The control layer compounds.

AIL is neither a prompt language nor a model-specific encoding. Frontier, open,
or specialized agents can use the same deterministic semantic interface.
Canonical source remains portable and human-auditable.

### 11. The leverage is in the total cost of a correct change

The economic hypothesis is lower context, consequence-analysis, repair,
regression, and review work at equal correctness. Measure provider-counted
tokens, elapsed agent work, repair cycles, regressions, and reviewer effort. Do
not claim a speedup or ROI before the comparison runs.

### 12. Start where agent autonomy is already the operating model

The buyer hypothesis is an engineering organization using agents as primary
implementers for greenfield backend services and workers. The funded entry point
is bounded validation, not production deployment. Built assets include the
compiler, contracts, semantic graph, and adversarial fixtures. Held-out
workloads, design-partner policy, integrations, production runtime, adoption,
and switching costs remain to build and validate.

### 13. Mechanism first. Economics next. Production only when justified.

M27 is complete and the repository has no active successor. A proposed funded
validation program—not the authorized repository roadmap—is gate-based:

1. retain the current bounded semantic oracle, change loop, and M27 observation;
2. freeze fresh representative tasks, build only the comparator capabilities
   they require, and run locked comparative validation;
3. make an explicit go, revise, or stop decision; and
4. select broader language, runtime, lowering, or ecosystem work only when
   required by the resulting evidence or a concrete deployment requirement.

Native lowering, LLVM integration, language breadth, and self-hosting are not
success measures or automatic phases.

### 14. What is proven—and what the raise must prove

Proven today: real compiler machinery, deterministic bounded semantics,
fixture-exact supported impact, atomic transactions, bounded architecture-policy
enforcement on frozen scenarios, and the public behavior corpus. There are zero
official comparative agent trials today. Not yet proven: comparative agent
acceleration, cross-model repeatability, reviewer savings, production lowering,
broad language coverage, or ecosystem adoption.

### 15. Fund the experiment that can falsify the thesis

Use UC-003 and `CancelJob` to demonstrate the mechanism and calibrate the
pipeline, not as the decisive economic benchmark. Freeze fresh held-out
design-partner tasks before adapting AIL to them, build equivalent strong
baseline environments, and pre-register a fixed maximum number of attempts,
outcomes, exclusions, and an assessor-independent reviewer rubric. Lock AIL
success targets after baseline results and before AIL trials. Then make an
explicit go, revise, or stop decision.

### 16. Appendix — retained M27 architecture-feedback pilot

Show the recorded operator changing course from a supplied seeded centralized
candidate. The candidate already passed behavior and arrived with the compact
architecture rejection; the retained evidence does not show the operator
independently authoring that initial bad candidate. The operator drilled into
the exact contributors, repaired authority placement, reran the checks, and
published the valid child. Keep raw compiler output available for diligence
rather than making it the primary presentation.

### 17. Appendix — baseline vs AIL measurement

Use one task contract and correctness oracle. Give baseline languages their
normal strong tools. Keep the model and agent policy the same where possible.
Pre-register a fixed maximum number of attempts. Treat completion and terminal
failures as primary outcomes, and successful-run efficiency as conditional.
Measure work, provider-counted tokens, repair, correctness/regressions, and
assessor-independent review.

## Retained M27 evidence and possible presentation

The narrated fixture runner from the prior unmerged demo branch is useful
diligence evidence, but it should become the evidence service behind a visual
agent workflow rather than the main experience.

### Presentation mode: one recorded operator, visible course correction

1. Start from the locked M23 `CancelJob` task, base revision, and seeded
   centralized candidate supplied to the operator.
2. Show **6/6 behavior pass** and the compact denied-publication result.
3. Render the four centralized findings as a policy card with contributors,
   not only terminal text.
4. Show the operator requesting structured drill-down and repairing the change
   by moving datastore authority to the domain handler.
5. Revalidate. Show behavior, architecture, publication, revision identity, and
   completion evidence as one bound result.
6. End with a proof-boundary card: this is one non-official run demonstrating
   actionable feedback, not a measured comparative speedup.

The retained M27 package records that feedback-and-repair sequence and is
available in the current checkout. It shows that the feedback was actionable in
that one run; it does not establish an end-to-end product workflow, comparative
advantage, statistical repeatability, or generalization beyond the seeded
candidate.

### Measurement mode: randomized baseline vs AIL trials

Use the accepted benchmark machinery rather than inventing a second evidence
format. Extend it only where reviewer telemetry is not yet represented.

#### Calibration fixtures and held-out experimental units

- Use UC-003 priority evolution and UC-007 `CancelJob` architecture control as
  mechanism demonstrations and pipeline calibration. They helped specify and
  implement the compiler behavior, so they cannot alone establish economic
  generalization.
- Freeze fresh held-out design-partner tasks before adapting AIL to them. Build
  equivalent mainstream-language workspaces, answer-free starts, hidden checks,
  and correctness oracles.
- Use strong Rust, Go, Python, and TypeScript configurations with normal
  compilers, language servers, formatters, search, build tools, and tests.
- Use the same model, model version, agent implementation, tool-call policy,
  limits, and task text across treatments where technically possible. Record
  every difference.
- Pre-register a fixed number or maximum number of attempts per
  task/model/language cell. The accepted UC-001/UC-003 policy requires at least
  ten successful baseline trials per task and language, but that minimum is not
  by itself a stopping rule or a sufficient statistical design for new tasks.
- Make completion rate and terminal-failure distribution primary outcomes.
  Preserve and report every attempt and pre-registered exclusion.
- Lock quantitative AIL success targets after the baseline campaign and before
  any comparative AIL result is inspected.
- Replicate across at least two model families before claiming cross-model
  repeatability. A single same-model comparison produces only a first signal.

#### Measures

| Dimension | Primary measures | Evidence source |
| --- | --- | --- |
| Time | wall time to final correct revision; active model/tool time | monotonic runner events |
| Tokens | provider-counted cumulative input; output; cache subset; category attribution | provider usage plus input-token preflight |
| Search/context | source reads; bytes read; search calls; semantic queries; repeated context | normalized tool events |
| Retries | validation attempts; repair cycles; model turns; terminal failure | accepted run-classification rules |
| Regressions | public/private failures; seeded-role misses; authority/effect growth; architecture denial | locked correctness and compiler oracles |
| Diff/evidence | changed paths; textual diff; semantic diff; evidence completeness | final-revision bundle |
| Reviewer effort | assessor-independent review time; questions; defects found/missed; confidence; approval decision | separate fixed reviewer rubric |

Correctness is an admission gate, not a metric that can be traded for speed.
Pre-register summaries, uncertainty intervals, exclusions, and the effect size
that would change the go/revise/stop decision. Efficiency comparisons should
identify the successful-run estimand explicitly; completion rates, terminal
failures, and excluded runs must still be reported alongside it.

#### Randomized assessor-independent reviewer study

1. Produce normalized review packages for each successful final revision. Do
   not reveal the hypothesis, candidate provenance, measured outcome, or raw
   agent chain-of-thought where those can be concealed. Do not claim treatment
   blinding when the programming language or treatment-specific evidence is
   visible; record each reviewer’s treatment guess.
2. Randomize package order within reviewers and balance language/task exposure.
3. Give every reviewer the same task contract, diff, test output, and allowed
   evidence budget. AIL semantic evidence should be counted and sized, not
   silently added for free.
4. Ask reviewers to decide approve/reject, identify known seeded defects, list
   blocking questions, and rate confidence.
5. Record active review time and evidence opened. Predefine timeout and
   incomplete-review handling.
6. Report reviewer-level and task-level distributions; do not collapse them into
   one unsupported composite score.

#### Go, revise, or stop gate

- **Go:** the locked AIL targets are met without lower correctness, hidden
  authority growth, or increased reviewer defect misses.
- **Revise:** compiler evidence is actionable but workflow, protocol, or task
  coverage prevents a fair or repeatable comparison.
- **Stop or reposition:** the advantage disappears against strong tools, depends
  on one model/task, or shifts cost to reviewers or runtime guarantees.

## External-use gaps requiring founder input

The artifact does not invent facts that are absent from the repository. Before
external fundraising use, founders should add or decide:

- financing amount, runway, tranche or milestone structure, and hiring/spend
  plan;
- verified team credentials relevant to compilers, agent evaluation, and
  developer-tool adoption;
- customer-discovery evidence, named design partners, or an explicit statement
  that none are committed; and
- any supported market-size, pricing, or adoption evidence.

Until those inputs exist, this is an investor-ready technical thesis and funded
validation deck, not a complete company fundraising deck.

## Claim ledger

| Claim | Status | Evidence or next proof |
| --- | --- | --- |
| AIL has an authoritative executable compiler | Proven on the bounded accepted core | Rust workspace and M14–M26 focused tests |
| AIL executes the public job-service behavior | Proven on the bounded fixture corpus | 37 public fixtures through the AIL runner |
| AIL can return exact pre-edit impact | Proven for the locked UC-003 evolution | 12 `must_change`, 2 `review`, 1 `unchecked` |
| AIL can block a behavior-passing architecture regression | Proven for locked UC-007 candidates | valid publishes; centralized and helper-split roll back |
| AIL makes agents faster or cheaper | Hypothesis | locked comparative trials required |
| AIL reduces regressions and reviewer effort | Hypothesis | hidden checks plus assessor-independent reviewer study required |
| AIL is production-ready | Not claimed | production lowering, runtime, I/O, concurrency, and operations remain future work |
| AIL is model-independent | Architectural property; operational breadth unproven | protocol is model-agnostic; multi-model trials required |

## Diligence pointers

- Project thesis: [`project-intent.md`](project-intent.md)
- Application and benchmark comparison: [`application-vision.md`](application-vision.md)
- Active milestone and proof boundary: [`STATUS.md`](STATUS.md)
- Benchmark policy: [`benchmarks/README.md`](benchmarks/README.md)
- Locked impact fixture: [`../specs/evolution-fixtures/workspace.json`](../specs/evolution-fixtures/workspace.json)
- Locked architecture outcomes: [`../specs/architecture-acceptance-fixtures/expected.json`](../specs/architecture-acceptance-fixtures/expected.json)
- Architecture compiler results: [`../specs/architecture-fixtures/results.json`](../specs/architecture-fixtures/results.json)
