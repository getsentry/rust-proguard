# R8 Retrace: Line Number Handling — Current Failures & Needed Fixes

This note accompanies `tests/r8-line-number-handling.rs`.

Status when this note was written:

- **10 tests total**
- **2 passing**: `test_obfuscated_range_to_single_line_stacktrace`, `test_preamble_line_number_stacktrace`
- **8 failing**: listed below

Like other ported suites, these tests:

- **Omit upstream `<OR>` markers** and list alternatives as duplicate frames.
- **Normalize expected indentation** to this crate’s output (`"    at ..."`).
- **Use `:0`** for “no line info” since this crate represents missing line numbers as `0`.

Below is a **one-by-one** explanation of the remaining failures and what behavior in the crate likely needs fixing.

## 1) `test_no_obfuscation_range_mapping_with_stacktrace`

- **Expected**:
  - `foo.a(…:0)` retraces to `foo(long):1:1` → `Main.foo(Main.java:1)`
  - `foo.b(…:2)` retraces to `bar(int):3` → `Main.bar(Main.java:3)`
  - For `0:0` and `0` mappings, upstream expects “use original line info semantics” (see upstream fixture comment).
- **Actual**:
  - `Main.foo(Main.java:0)` (lost the `:1`)
  - `Main.bar(Main.java:2)` (seems to preserve the **minified** line `2` rather than mapping to `3`)
  - `baz` and `main` keep minified lines `8`/`7` rather than dropping/normalizing.
- **What needs fixing**:
  - The crate’s “base mapping” (`0`, `0:0`) line-number semantics don’t match R8:
    - Some cases should map to the **original** line (e.g. `:1` for `foo`)
    - Some cases should prefer the **method’s declared original line** even when minified line is present (e.g. `bar(int):3`)
    - Some `0:0` entries should use the **stacktrace line** (R8’s special-case behavior).
  - The logic likely lives in member selection / line translation in `src/mapper.rs` / cache iteration paths.

## 2) `test_multiple_lines_no_line_number_stacktrace`

- **Expected** (no line in stacktrace):
  - Choose the `0:0` entries (base mappings) and emit their original lines:
    - `method1(Main.java:42)` and `main(Main.java:28)`
- **Actual**:
  - Emits both with `:0`.
- **What needs fixing**:
  - When the stacktrace has “no line” (`Unknown Source`), and the mapping provides `0:0:…:origLine:origLine` (or explicit original line metadata), we should be able to emit those original lines instead of forcing `0`.
  - Today we are collapsing “unknown line” to numeric `0` too early and then losing the mapping’s original line information.

## 3) `test_single_line_no_line_number_stacktrace`

- **Expected**:
  - Base mappings (`0:0`) for `a` and `b` should expand into multiple original methods (`method1` + `main`, etc.) with specific original lines where available.
  - For `c`, upstream emits two alternatives (`main3` and `method3`) and preserves their source context.
  - `main4` should preserve its declared original line `153`.
- **Actual**:
  - Everything ends up as `:0` (e.g. `method1(Main.java:0)`, `main4(Main.java:0)`).
- **What needs fixing**:
  - Same core issue as (3), but more visible:
    - Preserve/emit mapping-derived original lines for `0:0` entries.
    - Don’t convert “unknown” into `0` in a way that prevents later line reconstruction.

## 4) `test_no_obfuscated_line_number_with_override`

- **Expected**:
  - `main(Unknown Source)` still maps to `Main.main(Main.java:3)` because the mapping has a single `main(...):3`.
  - `overload(Unknown Source)` yields both overloads but without line suffixes in the non-verbose output.
  - `mainPC(:3)` should map to `Main.java:42` (mapping line is `42`).
- **Actual**:
  - Most frames show `:0`, and `mainPC` shows `:3` (minified line preserved) instead of `:42`.
- **What needs fixing**:
  - When obfuscated line numbers are missing (`Unknown Source`) but mapping provides a concrete original line, we should emit it (e.g. `main:3`).
  - For `mainPC(:3)`, we’re not translating minified `3` to original `42` even though the mapping is unambiguous.
  - This points to incorrect or missing “no obfuscated line number override” behavior in remapping.

## 5) `test_different_line_number_span_stacktrace`

- **Expected**:
  - The mapping says `method1(...):42:44 -> a` and the stacktrace is `a.a(…:1)`.
  - Upstream expands this to **three** possible original lines `42`, `43`, `44` (span).
- **Actual**:
  - Only one frame, and it uses the minified line `1` as the output line.
- **What needs fixing**:
  - For mappings that define a span of original lines for a single minified line (or ambiguous mapping within a span), we need to expand into the full set of candidate original lines rather than carrying through the minified line.
  - This is core “line span expansion” logic (member lookup + line translation).

## 6) `test_outside_line_range_stacktrace`

- **Expected**:
  - `a.a(:2)` and `a.a(Unknown Source)` both map to `some.other.Class.method1(Class.java:42)`
  - `b.a(:27)` maps to `some.Class.a(Class.java:27)` (outside any range → fall back to the “unmapped member name” for that class, per fixture)
  - `b.a(Unknown Source)` maps to `some.Class.method2(Class.java)` (no line)
- **Actual**:
  - `some.other.Class.method1(Class.java:2)` and `...:0` (line propagation wrong)
  - One line remains unparsed and unchanged: `at b.a(:27)` (it is emitted verbatim when not remapped)
  - Last frame becomes `method2(Class.java:0)` instead of `method2(Class.java)`
- **What needs fixing**:
  - **Parsing / fallback**: the `(:27)` location should be parsed and then remapped (or at least “best-effort” remapped), but currently it falls back to printing the original frame line.
  - **Outside-range semantics**: when the minified line is outside any mapped range, decide how to choose:
    - either fall back to a “best effort” member name remap,
    - or keep obfuscated, but the expected behavior is a best-effort remap.
  - **No-line formatting**: `Class.java` vs `Class.java:0` (same as (1)).

## 7) `test_invalid_minified_range_stacktrace`

- **Expected**:
  - Even though the mapping has an invalid minified range (`5:3`), upstream still retraces the method and produces `Main.java:3`.
- **Actual**:
  - The input line is emitted unchanged (not retraced).
- **What needs fixing**:
  - The mapping parser / remapper currently rejects or ignores invalid minified ranges entirely.
  - Upstream seems to treat this as recoverable and still uses the information to retrace.
  - Implement more tolerant handling of invalid minified ranges (or normalize them) so retrace still occurs.

## 8) `test_invalid_original_range_stacktrace`

- **Expected**:
  - For an invalid original range (`:5:2`), upstream still retraces and emits `Main.java:3`.
- **Actual**:
  - Emits `Main.java:6` (wrong translation).
- **What needs fixing**:
  - The translation logic from minified line → original line is not handling inverted original ranges correctly.
  - Needs clamping / normalization rules consistent with R8 (e.g. treat as single line, or swap, or ignore original span and use minified).


