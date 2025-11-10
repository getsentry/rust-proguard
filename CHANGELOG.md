# Changelog

## 5.7.0

### Various fixes & improvements

- feat(r8): Support outline and outlineCallsite annotations in ProguardCache (#62) by @romtsn
- feat(r8): Support outline and outlineCallsite annotations in ProguardMapping (#60) by @romtsn
- build: Fix new compiler warnings (#61) by @romtsn
- ref: Forbid `unwrap` except in tests and benches (#58) by @loewenheim

## 5.6.2

### Various fixes & improvements

- fix(r8): Handle invalid headers gracefully (#57) by @Dav1dde

## 5.6.1

### Various fixes & improvements

- feat(cache): Expose current cache version constant (#56) by @loewenheim

## 5.6.0

### Various fixes & improvements

- feat: Handle "synthesized" class/member annotations (#52) by @loewenheim
- ref: Parse ProGuard files smarter (#55) by @loewenheim
- chore: Fix 1.88.0 clippy lints (#53) by @loewenheim
- ref: Robustly parse R8 headers (#50) by @loewenheim
- ref: Remove lazy_static dependency (#51) by @loewenheim
- chore: Clippy (#49) by @loewenheim
- chore: Simplify check (#47) by @loewenheim
- chore(lint): Elided lifetimes (#46) by @jjbayer
- cache: Add creation benchmark (#44) by @loewenheim

## 5.5.0

### Various fixes & improvements

- Commit Cargo.lock (#43) by @loewenheim
- Update edition to 2021 (#43) by @loewenheim
- Don't rename uuid dep (#43) by @loewenheim
- Remove ClassIndex (#43) by @loewenheim
- Use chars (#43) by @loewenheim
- Add module docs (#42) by @loewenheim
- Write vectors at once (#42) by @loewenheim
- Feedback (#42) by @loewenheim
- Abstract out the binary search and use it in remap_method (#42) by @loewenheim
- cache.rs -> cache/mod.rs (#42) by @loewenheim
- Add parsing + mapping benchmarks (#42) by @loewenheim
- Missed something (#42) by @loewenheim
- Use correct name of cache everywhere (#42) by @loewenheim
- Rename error types (#42) by @loewenheim
- debug: use references (#42) by @loewenheim
- Update src/cache/raw.rs (#42) by @loewenheim
- Add cache to parsing benchmark (#42) by @loewenheim
- Remove unwrap in binary search (#42) by @loewenheim
- Remove test I committed by mistake (#42) by @loewenheim
- Typo (#42) by @loewenheim
- Cleanup (#42) by @loewenheim
- Expand test coverage (#42) by @loewenheim
- Add remapping benchmark (#42) by @loewenheim
- Use binary search more aggressively (#42) by @loewenheim

_Plus 10 more_

## 5.4.1

### Various fixes & improvements

- Update CI definitions (#37) by @Swatinem
- feat: add method signature parsing (#35) by @viglia
- Support symbolicated file names (#36) by @wzieba

## 5.4.0

### Various fixes & improvements

- enhance: make mapping by params initialization optional (#34) by @viglia
- Clear `unique_methods` per `class` (#33) by @Swatinem

## 5.3.0

### Various fixes & improvements

- Add getter method for private parameters struct field (#32) by @viglia
- release: 5.2.0 (8032053d) by @getsentry-bot

## 5.2.0

- No documented changes.

## 5.1.0

### Various fixes & improvements

- Allow remapping just a method without line numbers (#30) by @Swatinem

## 5.0.2

### Various fixes & improvements

- Fix line number mismatch for callbacks (#27) by @adinauer

## 5.0.1

### Various fixes & improvements

- perf(proguard): Try to optimize proguard mapping parsing (#26) by @Zylphrex

## 5.0.0

**Breaking Changes**:

- Update `uuid` dependency to version `1.0.0`. ([#22](https://github.com/getsentry/rust-proguard/pull/22))

**Thank you**:

Features, fixes and improvements in this release have been contributed by:

- [@jhpratt](https://github.com/jhpratt)

## 4.1.1

**Fixes**:

- Removed overly conservative limit in `has_line_info`.

**Thank you**:

Features, fixes and improvements in this release have been contributed by:

- [@JaCzekanski](https://github.com/JaCzekanski)

## 4.1.0

**Features**:

- A new `MappingSummary` was added providing some information about the mapping file.
- Added support for remapping a complete Java `StackTrace`, including the `Throwable` and cause chain.

**Thank you**:

Features, fixes and improvements in this release have been contributed by:

- [@dnaka91](https://github.com/dnaka91)

## 4.0.1

**Fixes**

- Fix `has_line_info` to not short-circuit when it found lines _without_ line-info.

## 4.0.0

This is a complete rewrite of the crate.
It focuses on two types, `ProguardMapping` and `ProguardMapper`.

**ProguardMapping**

Provides high level metadata about a proguard mapping file, and allows iterating
over the contained `ProguardRecord`s.

This is a replacement for the previous `Parser`. For example,
`Parser::has_line_info()` becomes `ProguardMapping::has_line_info()`.

**ProguardMapper**

Allows re-mapping class names and entire frames, with support for inlined frames.

This is a replacement for the previous `MappingView`, and allows easier
re-mapping of both class-names and complete frames.

`MappingView::find_class("obfuscated").map(Class::class_name)` becomes
`ProguardMapper::remap_class("obfuscated")`, and the
`ProguardMapper::remap_frame` function replaces manually collecting and
processing the results of `Class::get_methods`.

## 3.0.0

- Update `uuid` to `0.8.1`.

## 2.0.0

- Implement support for R8.
- Update `uuid` to `0.7.2`.

## 1.1.0

- Update `uuid` to `0.6.0`.

## 1.0.0

- Initial stable release.
