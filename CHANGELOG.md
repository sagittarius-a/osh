# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3] - 2021-11-28

### Added

- Basic support for wildcard (No recursive glob support yet)
- Add recursive alias support
- `export` and `unset` commands to manipulate environment
- Warning and Error prefixes in cli are now colorized with yellow and red respectively
- Support `;` to chain commands
- CHANGELOG.md file

### Changed

- Configuration file is automatically reloaded when edited

## [0.2] - 2021-11-27

### Added

- Alias support
- Logging support

### Changed

- Renamed the project to osh
- Listing aliases return error code 0

## [0.1] - 2021-07-15

### Added

- `reload` command to reload configuration file without opening a new osh instance
- `config` command to quickly edit configuration file with `$EDITOR`
- Colored prompt mimicking Gentoo default bash theme
- `unalias` command to delete a registered alias
- Support for global aliases
- Support for alias
- Access last visited directory with `cd -`
- Environment are expanded