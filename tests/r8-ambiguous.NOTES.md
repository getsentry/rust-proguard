# r8-ambiguous.rs failures

This doc summarizes the current failures from running `cargo test --test r8-ambiguous`.

## `test_ambiguous_method_verbose_stacktrace`

- **Failure**: The frame lines are emitted verbatim (no remapping at all).
- **Why**:
  - The input frames have **no line numbers** (e.g. `(Foo.java)`), which means `frame.line == 0`.
  - The current remapper does not produce any remapped candidates for these frames (so `format_frames` falls back to printing the original line). This indicates a gap in the “no-line” / best-effort ambiguous member mapping behavior for `line == 0` frames.

## `test_ambiguous_stacktrace`

- **Failure**: No remapping occurs; frames like `at a.a.a(Unknown Source)` are preserved.
- **Why**:
  - These frames have `Unknown Source` with **no line number**, so `frame.line == 0`.
  - For `line == 0`, the current implementation ends up with **no remapped candidates** and prints the original frame line unchanged (instead of emitting ambiguous alternatives `foo`/`bar`).

## `test_ambiguous_missing_line_stacktrace`

- **Failure**: Output uses `R8.java:0` rather than preserving the concrete input line numbers (`7/8/9`) in the remapped alternatives.
- **Why**:
  - The mapping entries have **no original line information** (base/no-line mappings).
  - The current mapping logic uses the mapping’s “original start line” (which defaults to `0`) rather than propagating the **caller-provided minified line** when available.

## `test_ambiguous_with_multiple_line_mappings_stacktrace`

- **Failure**: Last frame stays obfuscated (`com.android.tools.r8.Internal.zza(Unknown)`), expected a deobfuscated `Internal.foo(Internal.java:0)`-style frame.
- **Why**:
  - `(...(Unknown))` parses as a frame with `file = "Unknown"` and `line = 0`.
  - All available member mappings are **line-ranged** (e.g. `10:10`, `11:11`, `12:12`), so with `line == 0` they do not match and the remapper produces **no candidates**, falling back to the original frame line.

## `test_ambiguous_with_signature_stacktrace`

- **Failure**: Same symptom as above (`Internal.zza(Unknown)` remains), expected deobfuscated member.
- **Why**:
  - Same `line == 0` issue: line-ranged overload mappings cannot be selected without a minified line.
  - The remapper currently has no fallback that returns “best effort” candidates for `line == 0` frames (e.g., returning all overloads, or preferring a base mapping if present).

## `test_inline_no_line_assume_no_inline_ambiguous_stacktrace`

- **Failure**: Expected retraced output, but actual output is unchanged (`at a.foo(Unknown Source)`).
- **Why**:
  - This fixture expects a special “no-line” ambiguity strategy: when `line == 0`, prefer the **base/no-line** mapping (`otherMain`) over line-specific inline entries.
  - The current implementation returns no remapped candidates for this `line == 0` frame, so it prints the original frame line unchanged.

