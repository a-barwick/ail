# ADR 0015: `ailc check` emits located structured findings

- Status: Proposed
- Date: 2026-08-24

## Context

`ailc check` printed one line per rejection:

```
AIL.TYPE.FIELD_MISMATCH:<source-set>:164:203: expected.type=Text actual.type=Int
AIL.ARCH.BOUNDARY:group:transport:M23-POL-GROUP-DEPENDENCY
```

That is a log line, not a compiler result. Four facts were missing or wrong:

1. Type, name, and capability errors reported the path `<source-set>`. The
   linker merges every file into one unit and each file numbers its own bytes
   from zero, so the offsets did not identify a file at all.
2. No finding carried the source text at its span, so a caller had to open the
   file and guess which construct the offsets covered.
3. Parse failures produced only the cause string `<file> has parse diagnostics`.
   The parser already had a span and an expected/actual token pair.
4. Architecture failures dropped every measured fact and every contributor. The
   evaluator already had the rule, the scope, the metric values, and the unit
   identifiers.

The compiler already computed the facts. The output threw them away.

## Decision

`ailc check` and the check path of `ailc publish` emit one
[`SourceFinding`](../../compiler/ail-compiler/src/finding.rs) per error. A
finding carries:

- the source-set path, byte range, and one-based line and column of both ends;
- the source text at that range and the full text of its first line, each
  bounded at 240 bytes;
- the checker's `expected` and `actual` fact maps;
- other facts the checker already had, such as the denied architecture rule, its
  scope, its measured metrics, the modules a source set declares, and the
  capability interfaces the check environment supplies;
- related named locations, such as the record and field a type error disagrees
  with, or each contributing architecture unit;
- a `requirement`: the constraint those facts state.

`requirement` states the constraint the checker enforced and the value it
measured. It never prescribes an edit. Every requirement reads
`<subject> must <constraint>` and may add the measured value after a semicolon.
`group transport must not depend on group domain` is a requirement;
`remove the calls edge transport:dispatch -> domain:work` is a rewrite, and the
checker has no fact that chooses one repair over another. Removing that edge,
moving the callee, or widening the policy all satisfy the same constraint.

Codes with a named fact shape get a named requirement. Otherwise a shared
`expected`/`actual` key produces
`<key> must be <expected> at this span; the checker measured <actual>`, and a
lone `expected` key produces `<key> must be <expected> at this span`. A code
whose facts name no constraint reports none.

Text output is the default. `--json` emits the same findings as one JSON
document on stdout. Both render the same values, so the two views cannot drift.

### Locating a span in a merged source set

`link_source_set` merges declarations from every file into one unit. To keep
file identity exact, check links a second copy whose spans are shifted into
disjoint ranges of one virtual buffer: file `i` occupies
`[base_i, base_i + len_i]`, and `base_0` is 1 so that the empty span at offset
zero stays reserved for facts with no source position. A diagnostic span on that
copy maps back to exactly one file and one offset inside it. The stored unit
keeps unshifted spans, so interpretation, architecture analysis, and revision
handles are unchanged.

### Locating the source the caller supplied

`StoredSourceSet::build` canonicalizes each file before retaining it. Reporting
offsets against that rewrite would name lines the caller cannot open. Check now
links and checks the parse of the text the caller supplied, and retains the
canonical text as before. Canonical formatting moves whitespace only, so both
parses carry the same declarations. When the supplied text is not canonical the
finding carries the fact `source.canonical=false`.

### Architecture findings

Policy identity is architectural: a rule, a scope, and ordered contributor unit
identifiers of the form `module:function`. Those identifiers resolve to a
declared function, so the driver resolves each contributor to a file and span.
For `AIL.ARCH.BOUNDARY` the primary location is the source unit of the forbidden
edge; otherwise it is the scope when the scope is a contributor, and the first
contributor otherwise.

## Consequences

`SourceSetDiagnostic` now reports the owning file and its local span instead of
`<source-set>`. `EvolutionBuildFailure` gains `findings`, and
`CliArchitectureFailure` gains `findings`; `causes` and `diagnostics` keep their
existing meaning, so library callers are unaffected.

ADR 0013 stated that parse failures surface as `EvolutionBuildFailure` causes
and not as `AIL.PARSE.EXPECTED_TOKEN`. That still holds for `causes`. Parse
failures now also produce `AIL.PARSE.EXPECTED_TOKEN` findings, because the
parser already has the span and the token pair and the caller needs them.

Semantic diagnostic order for multi-file sets is now file order, then offset
within the file, because the checker sorts by span start and spans are shifted.
That is a total order over the source set; it was previously ambiguous across
files.

This adds no syntax, no language construct, and no project or capability
configuration file. `architecture.json` keeps its contents and its meaning.

## Alternatives considered

- Attribute a diagnostic to a file by searching for a declaration whose span
  contains it: rejected. Declarations from different files hold overlapping byte
  ranges, so the answer is not unique.
- Attribute by the qualified module name in a related handle: rejected. Related
  handles name where a declaration lives, not where the offending expression
  lives, so cross-file errors would name the wrong file.
- Shift spans in the stored unit as well: rejected. Expression handle
  identifiers embed span offsets, and interpretation and architecture analysis
  read the stored unit. Linking a second copy confines the change to
  diagnostics.
- Report offsets against canonical source and mark them as reformatted:
  rejected. A caller cannot open an offset that exists only in a rewrite.
- Caret and underline rendering: rejected. The product is the facts. The text
  view exists so a human can read them.
- A repair clause in the requirement, such as naming the edge to remove for
  `AIL.ARCH.BOUNDARY`: rejected. The checker measures a forbidden edge; it does
  not know which of the several edits that satisfy the constraint the caller
  wants. Naming one is a guessed rewrite wearing a fact's clothing.
- Emit only JSON: rejected. The existing text output is what tests and operators
  read today.

## Validation

`cargo +1.87.0 test -p ail-compiler --test ailc_findings` runs the `ailc` binary
and proves:

- a field mismatch reports `types.ail:6:27-6:28 bytes 81..82`, the snippet `1`,
  its line, `expected.type=Text`, `actual.type=Int`, and the requirement;
- a non-canonical file reports offsets that index the supplied text, plus
  `source.canonical=false`;
- a two-file field mismatch locates the error in `beta.ail` and the disagreeing
  field in `alpha.ail`;
- a missing import names the import text, the file, the module, and the modules
  the source set declares;
- `AIL.ARCH.BOUNDARY` names `M23-POL-GROUP-DEPENDENCY`, its scope, its forbidden
  edge facts, `transport.ail` as the violating source, `domain:work` as a
  contributor, and the requirement
  `group transport must not depend on group domain; the candidate has a calls
  edge transport:dispatch -> domain:work`;
- every requirement across ten failing inputs and at least eight distinct codes
  states a constraint with `must` and contains none of 22 edit verbs, so no code
  can reintroduce a prescribed rewrite;
- a parse failure reports `broken.ail:2:9-2:9` with `expected.token=:` and
  `actual.token=}`;
- an unknown capability interface names the interfaces the check environment
  supplies, which is none;
- a recursive cycle names the cycle and both declarations once each;
- a passing workspace reports zero findings;
- a rejected `publish --json` reports the same findings as `check --json` and
  writes no revision.

`cargo +1.87.0 test -p ail-compiler --test ailc_check` proves the existing check
contract still holds.
