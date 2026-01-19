# R8 Special Formats fixtures: failures & required behavior changes

Ported from upstream R8 retrace fixtures:
- `src/test/java/com/android/tools/r8/retrace/stacktraces/NamedModuleStackTrace.java`
- `src/test/java/com/android/tools/r8/retrace/stacktraces/AutoStackTrace.java`
- `src/test/java/com/android/tools/r8/retrace/stacktraces/PGStackTrace.java`
- `src/test/java/com/android/tools/r8/retrace/stacktraces/LongLineStackTrace.java`

This doc lists **only the failing tests** and explains, one-by-one, what would need to change in `rust-proguard` to match upstream R8 retrace behavior.

## `test_named_module_stacktrace`

- **Upstream behavior**: Recognizes and preserves Java 9+ “named module / classloader” prefixes in stack frames (e.g. `classloader.../named_module@9.0/...`) while still retracing the class/method inside.
- **Current crate behavior**: Only the last frame (`at a.e(...)`) is retraced; frames with module/classloader prefixes remain unmapped and are emitted unchanged (still tab-indented).
- **Why it fails**:
  - `parse_frame` treats everything before the last `.` as the “class”, so `classloader.../named_module@9.0/a` becomes the class name (contains `/.../`), which won’t match any mapping key (`classloader.a.b.a` / `a`).
- **What needs fixing**:
  - **Frame parsing**: support the `StackTraceElement#toString()` module/classloader prefix forms:
    - `<loader>/<module>@<ver>/<class>.<method>(...)`
    - `<loader>//<class>.<method>(...)`
    - `<module>@<ver>/<class>.<method>(...)`
    - `<module>/<class>.<method>(...)`
  - Preserve the prefix when formatting, but apply mapping to the `<class>.<method>` portion.

## `test_auto_stacktrace`

- **Upstream behavior**: Parses and retraces “auto” frame locations of the form:
  - `at qtr.a(:com.google.android.gms@...:46)`
  where the file section contains multiple `:` characters and begins with `:`.
- **Current crate behavior**: These frames are not parsed, so they are emitted unchanged.
- **Why it fails**:
  - `parse_frame` currently splits the location on the **first** `:`; with a leading `:` the “file” becomes empty and line parsing fails.
  - More generally, the location payload requires splitting on the **last** `:` to isolate the line number.
- **What needs fixing**:
  - **Location parsing**: parse `(...:<line>)` by splitting on the **last** `:` and allowing additional colons in the file name/payload, including a leading `:`.

## `test_pg_stacktrace`

- **Upstream behavior**: Recognizes and retraces logcat-prefixed frames (the frame appears after `AndroidRuntime:`), and also expands `(...(PG:<line>))` into a synthesized Java file name (`SectionHeaderListController.java:<line>`).
- **Current crate behavior**: Treats these as plain text lines and emits them unchanged.
- **Why it fails**:
  - `parse_frame` only matches lines that (after trimming) start with `at ` and end with `)`. In logcat format, the trimmed line begins with the timestamp and tag, not `at`.
  - The `PG:` file name needs special handling to synthesize a proper Java file name from the class.
- **What needs fixing**:
  - **Format support**: detect and parse stack frames embedded in logcat lines, preserving the logcat prefix while remapping the embedded `at ...(...)` portion.
  - **PG source file synthesis**: treat `PG` as “unknown source file placeholder” and replace it with `<ClassSimpleName>.java` when formatting.

