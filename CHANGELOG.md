# Changelog

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
