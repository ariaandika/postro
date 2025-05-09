# [Unreleased]

### Added
- `Table` trait and derive macro.
- `query_scalar` function.
- `Decode` and `Encode` derive macro ([#1]).

[#1]: https://github.com/ariaandika/postro/issues/1

### Changed
- renamed `query` function to `query_as`.
- renamed `query_row` function to `query`.

### Removed
- `execute` function.

### Fixed

- `Into` impl from Config
- suspend `FetchStream` on decode error
- time `Decoding` logic
- json `Decoding` logic
- handle `NULL` value

