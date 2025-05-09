# Unreleased

### Added
- add `Table` trait and derive macro
- add `query_scalar` function

### Changed
- rename `query` function to `query_as`
- removed `query_row` function to `query`

### Removed
- renamed `execute` function to `query`

### Fixed

- fix Into impl from Config
- fix to suspend FetchStream on decode error
- fix time Decoding logic
- fix json Decoding logic
- fix handle NULL value

