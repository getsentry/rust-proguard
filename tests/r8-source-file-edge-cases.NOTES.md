# R8 Retrace: Source File Edge Cases — Remaining Fixes

After normalizing fixture indentation (member mappings use 4 spaces), `tests/r8-source-file-edge-cases.rs` has **4 passing** and **3 failing** tests.

This note documents, **one-by-one**, what still needs fixing in the crate (not in the fixtures) to make the remaining tests pass.

## 1) `test_colon_in_file_name_stacktrace`

- **Symptom**: Frames are emitted unchanged and do not get retraced:
  - Input stays as `at a.s(:foo::bar:1)` / `at a.t(:foo::bar:)`.
- **Root cause**: `src/stacktrace.rs::parse_frame` splits `(<file>:<line>)` using the **first** `:`.
  - For `(:foo::bar:1)`, the first split produces `file=""` and `line="foo::bar:1"`, so parsing the line number fails and the whole frame is rejected.
- **Fix needed**:
  - In `parse_frame`, split `file:line` using the **last** colon (`rsplit_once(':')`) so file names can contain `:` (Windows paths and this fixture).
  - Treat an empty or non-numeric suffix after the last colon as “no line info” (line `0`) instead of rejecting the frame.

## 2) `test_file_name_extension_stacktrace`

This failure is due to two independent gaps.

### 2a) Weird location forms aren’t parsed/normalized consistently

- **Symptom**: Output contains things like `Main.java:` and `R8.foo:0` instead of normalized `R8.java:0` for “no line” cases.
- **Root cause**: `parse_frame` only supports:
  - `(<file>:<number>)`, or
  - `(<file>)` (treated as line `0`),
  and it currently rejects or mis-interprets inputs like:
  - `(Native Method)`, `(Unknown Source)`, `(Unknown)`, `()`
  - `(Main.java:)` (empty “line” part)
  - `(Main.foo)` (no `:line`, but also not a normal source file extension)
- **Fix needed**:
  - Make `parse_frame` permissive for these Java stacktrace forms and interpret them as a parsed frame with **line `0`** so remapping can then replace the file with the mapping’s source file (here: `R8.java`).
  - Also apply the “split on last colon” rule from (1) so `file:line` parsing is robust.

### 2b) `Suppressed:` throwables are not remapped

- **Symptom**: The throwable in the `Suppressed:` line remains obfuscated:
  - Actual: `Suppressed: a.b.c: You have to write the program first`
  - Expected: `Suppressed: foo.bar.baz: You have to write the program first`
- **Root cause**: `src/mapper.rs::remap_stacktrace` remaps:
  - the first-line throwable, and
  - `Caused by: ...`,
  but it does **not** detect/handle `Suppressed: ...`.
- **Fix needed**:
  - Add handling for the `Suppressed: ` prefix analogous to `Caused by: `:
    - parse the throwable after the prefix,
    - remap it,
    - emit with the same prefix.

## 3) `test_class_with_dash_stacktrace`

- **Symptom**: An extra frame appears:
  - Actual includes `Unused.staticMethod(Unused.java:0)` in addition to `I.staticMethod(I.java:66)`.
- **Root cause**: The mapping includes synthesized metadata (`com.android.tools.r8.synthesized`) and multiple plausible remapped frames, including synthesized “holder/bridge” frames.
  - Today we emit all candidates rather than preferring non-synthesized frames.
- **Fix needed**:
  - Propagate the synthesized marker into `StackFrame.method_synthesized` during mapping.
  - When multiple candidate remapped frames exist for one obfuscated frame, **filter synthesized frames** if any non-synthesized frames exist (or apply an equivalent preference rule).


