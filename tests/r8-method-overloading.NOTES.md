# R8 Method Overloading fixtures: failures & required behavior changes

Ported from upstream R8 retrace fixtures/tests:
- `src/test/java/com/android/tools/r8/retrace/stacktraces/OverloadedWithAndWithoutRangeStackTrace.java`
- `src/test/java/com/android/tools/r8/retrace/stacktraces/OverloadSameLineTest.java`
- `src/test/java/com/android/tools/r8/retrace/RetraceMappingWithOverloadsTest.java`

This doc lists **only the failing tests** and explains, one-by-one, what would need to change in `rust-proguard` to match upstream R8 retrace behavior.

## `test_retrace_mapping_with_overloads_api_has_3_candidates`

- **Upstream behavior** (`RetraceMappingWithOverloadsTest`): `lookupMethod("a")` on class `A` yields **3** method elements for the residual name `a`:
  - `select(java.util.List)` (no line info)
  - `sync()` (line-mapped range)
  - `cancel(java.lang.String[])` (no line info)
- **Current crate behavior**: `ProguardMapper::remap_frame(StackFrame::new("A","a",0))` yields **2** candidates.
- **Why it fails**:
  - This crate treats `line = 0` as “no line information” and may be **filtering out** line-ranged candidates (like `sync()`) unless a concrete line is provided.
  - Alternatively, it may be collapsing/choosing a subset when multiple candidates exist without a line.
- **What needs fixing**:
  - **API semantics**: align “method lookup with no line” behavior with R8 retrace’s `lookupMethod`, which includes both:
    - base/no-line mappings, and
    - line-ranged mappings (as candidates) even when the position is unknown.

