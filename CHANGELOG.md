# Changelog

## 4.0.0

This is a complete rewrite of the crate.
It focuses on two types, `Mapping` and `Mapper`.

**Mapping**

Provides high level metadata about a proguard mapping file, and allows iterating
over the contained `Record`s.

This is a replacement for the previous `Parser`. For example,
`Parser::has_line_info()` becomes `Mapping::has_line_info()`.

**Mapper**

Allows re-mapping class names and entire frames, with support for inlined frames.

This is a replacement for the previous `MappingView`, and allows easier
re-mapping of both class-names and complete frames.

`MappingView::find_class("obfuscated").map(Class::class_name)` becomes
`Mapper::remap_class("obfuscated")`, and the `Mapper::remap_frame` function
replaces manually collecting and processing the results of `Class::get_methods`.

## 3.0.0

- Update `uuid` to `0.8.1`.

## 2.0.0

- Implement support for R8.
- Update `uuid` to `0.7.2`.

## 1.1.0

- Update `uuid` to `0.6.0`.

## 1.0.0

- Initial stable release.
