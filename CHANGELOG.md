# [Unreleased]

### Added
- `Table` trait and derive macro.
- `query_scalar` function.
- `Decode` and `Encode` derive macro (#1).

### Changed
- renamed `query` function to `query_as`.
- renamed `query_row` function to `query`.

### Removed
- `execute` function.

### Fixed

- fix Into impl from Config
- fix to suspend FetchStream on decode error
- fix time Decoding logic
- fix json Decoding logic
- fix handle NULL value

