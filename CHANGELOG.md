# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.2] - 2026-11-25

### Fixed

- `move_group` no longer moves a group into a group of its own subtree: such a move cut the subtree off the root,
  and the database with it. The check is in the engine itself, so a merge cannot do it either.
- `move_group` refuses to move the root group, the recycle bin or a group in it, and to move a group into the bin or
  a group in it: a group reaches the bin by deleting, not by a move. Nothing changes on a refusal.

## [1.0.1] - 2026-11-15

### Changed

- The repository and the engine crate are renamed: `kdbx-rust-core` - `kdbx-core` (`crates/kdbx-core`, `use kdbx_core::`).
  The code is the same as in 1.0.0.

## [1.0.0] - 2026-11-14

The first version of the core under its own numbering; the engine and its command api share one version.

### Added

- Read and write KeePass KDBX 4 databases: a password, a key file, or both; AES-256 and ChaCha20, Argon2d / Argon2id
  and AES-KDF.
- Entries and groups: custom fields, attachments, history, recycle bin, search, TOTP.
- Merge of two copies of one database and import of another database.
- A password generator.
- `pass-api`: the command api the apps call - json in, json out - and `CORE_VERSION`.

[Unreleased]: https://github.com/BigToEstDev/kdbx-core/compare/v1.0.2...HEAD
[1.0.2]: https://github.com/BigToEstDev/kdbx-core/releases/tag/v1.0.2
[1.0.1]: https://github.com/BigToEstDev/kdbx-core/releases/tag/v1.0.1
[1.0.0]: https://github.com/BigToEstDev/kdbx-core/releases/tag/v1.0.0
