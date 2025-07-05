# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
Here's a template for each release section. This file should only include changes that are
noticeable to end-users since the last release. For developers, this project follows
[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) to track changes.

## [1.0.0] - YYYY-MM-DD

### Added

- (**BREAKING**) Always place breaking changes at the top.
- Append other changes in chronological order under the relevant subsections.

### Changed

### Deprecated

### Removed

### Fixed

### Security

[1.0.0]: https://github.com/user/repo/compare/v0.0.0..v1.0.0
-->

## [Unreleased]

### Added

- Implement `Emplace` for `&mut MaybeUninit<[u8; N]>` ([#2]).

### Changed

- Add `#[must_use]` for `Slot`, `from_fn!()` and `from_closure()` (#PRNUM).

### Removed

- (**BREAKING**) Remove `Emplace` implementations for `&mut [u8; N]`, `&mut [u8]` and `&mut Vec<u8>`
  ([#2]).

- Add `#[must_use]` for `Slot`, `from_fn!()` and `from_closure()` (#PRNUM).

[#PRNUM]: https://github.com/loichyan/dynify/pull/PRNUM
[#2]: https://github.com/loichyan/dynify/pull/2

## [0.0.1] - 2025-07-05

🎉 Initial release. Check out [README](https://github.com/loichyan/dynify/blob/v0.0.1/README.md) for
more details.

[Unreleased]: https://github.com/loichyan/dynify/compare/v0.0.1..HEAD
[0.0.1]: https://github.com/loichyan/dynify/releases/tag/v0.0.1
