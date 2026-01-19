# R8 Exception Handling fixtures: failures & required behavior changes

Ported from upstream R8 retrace fixtures under:
- `src/test/java/com/android/tools/r8/retrace/stacktraces/`

This doc lists **only the failing tests** and explains, one-by-one, what would need to change in `rust-proguard` to match upstream R8 retrace behavior. (We keep expectations as-is; no behavior fixes here.)

## `test_suppressed_stacktrace`

- **Upstream behavior**: Throwable lines prefixed with `Suppressed:` still have their exception class retraced (e.g. `Suppressed: a.b.c: ...` → `Suppressed: foo.bar.baz: ...`).
- **Current crate behavior**: Leaves the suppressed exception class as `a.b.c`.
- **Why it fails**: `stacktrace::parse_throwable` recognizes normal throwables and `Caused by:`, but the `Suppressed:` prefix requires special-case parsing/stripping and then re-emitting with the same prefix.
- **What needs fixing**:
  - **Throwable parsing**: treat `Suppressed:` lines as throwables (similar to `Caused by:`) and remap their class name.

## `test_circular_reference_stacktrace`

- **Upstream behavior**: Retrace rewrites `[CIRCULAR REFERENCE: X]` tokens by remapping `X` as a class name (when it looks like an obfuscated class).
- **Current crate behavior**: Leaves the input unchanged.
- **Why it fails**: These lines are neither parsed as throwables nor stack frames, so they currently fall through to “print as-is”.
- **What needs fixing**:
  - **Extra line kinds**: add a parser/rewriter for circular-reference marker lines that extracts the referenced class name and applies `remap_class`.
  - **Robustness**: keep the upstream behavior of only rewriting valid markers and leaving invalid marker formats unchanged.

## `test_exception_message_with_class_name_in_message`

- **Upstream behavior**: Retrace can replace obfuscated class names appearing inside arbitrary log/exception message text (here it replaces `net::ERR_CONNECTION_CLOSED` → `foo.bar.baz::ERR_CONNECTION_CLOSED`).
- **Current crate behavior**: Does not rewrite inside plain text lines.
- **Why it fails**: `remap_stacktrace` currently only remaps:
  - throwable headers (`X: message`, `Caused by: ...`, etc.)
  - parsed stack frames (`at ...`)
  Everything else is emitted unchanged.
- **What needs fixing**:
  - **Text rewriting pass** (R8-like): implement optional “message rewriting” for known patterns where an obfuscated class appears in text (in this fixture: a token that looks like `<class>::<rest>`).
  - **Scoping**: upstream uses context; we likely need a conservative implementation to avoid over-replacing.

## `test_unknown_source_stacktrace`

- **Expected in test**: deterministic ordering for ambiguous alternatives: `bar` then `foo`, repeated for each frame.
- **Current crate behavior**: emits the same set of alternatives but in the opposite order (`foo` then `bar`).
- **Why it fails**: ambiguous member ordering is currently determined by internal iteration order (mapping parse order / sorting) which does not match upstream’s ordering rules for this fixture.
- **What needs fixing**:
  - **Stable ordering rule** for ambiguous alternatives (e.g., preserve mapping file order, or sort by original method name/signature in a defined way matching R8).

