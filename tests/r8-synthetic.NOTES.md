# r8-synthetic.rs failures

This doc summarizes the current failures from running `cargo test --test r8-synthetic`.

## `test_synthetic_lambda_method_stacktrace`

- **Failure**: An extra synthetic frame is emitted:
  - `example.Foo$$ExternalSyntheticLambda0.run(Foo.java:0)`
- **Expected**: Only the “real” deobfuscated frames (`lambda$main$0`, `runIt`, `main`, `Main.main`).
- **Why**:
  - The mapper currently includes synthesized lambda bridge frames (marked via `com.android.tools.r8.synthesized`) instead of filtering them out when a better “real” frame exists.
  - Also, `Unknown Source` maps to `:0` for line numbers, so the synthetic frame shows `Foo.java:0`.

## `test_synthetic_lambda_method_with_inlining_stacktrace`

- **Failure**: Same as above — extra synthetic frame:
  - `example.Foo$$ExternalSyntheticLambda0.run(Foo.java:0)`
- **Expected**: No synthetic lambda `run(...)` frame in the output.
- **Why**:
  - Same root cause: missing synthesized-frame suppression when a non-synthesized alternative exists.

## `test_moved_synthetized_info_stacktrace`

- **Failure**: An extra synthesized frame is emitted:
  - `com.android.tools.r8.BaseCommand$Builder.inlinee$synthetic(BaseCommand.java:0)`
- **Expected**: Only:
  - `com.android.tools.r8.BaseCommand$Builder.inlinee(BaseCommand.java:206)`
- **Why**:
  - The mapping has both “real” and `$synthetic` variants, with synthesized metadata attached to the `$synthetic` variant.
  - The mapper currently emits both candidates rather than filtering out the synthesized one when the non-synthesized target exists.

