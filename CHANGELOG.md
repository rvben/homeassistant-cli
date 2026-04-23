# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).















## [0.2.0](https://github.com/rvben/homeassistant-cli/compare/v0.1.14...v0.2.0) - 2026-04-23

### Added

- **registry**: add ha registry entity list and remove subcommands ([9a3ae67](https://github.com/rvben/homeassistant-cli/commit/9a3ae67808b966b88eec7ba9b404edeb8584cf59))

## [0.1.14](https://github.com/rvben/homeassistant-cli/compare/v0.1.13...v0.1.14) - 2026-04-03

### Added

- **init**: improve token URL hint, add next steps block ([0a2cb42](https://github.com/rvben/homeassistant-cli/commit/0a2cb422e1fd12b1848bf112595d2ad4075daa39))

## [0.1.13](https://github.com/rvben/homeassistant-cli/compare/v0.1.12...v0.1.13) - 2026-04-03

## [0.1.12](https://github.com/rvben/homeassistant-cli/compare/v0.1.11...v0.1.12) - 2026-04-03

## [0.1.11](https://github.com/rvben/homeassistant-cli/compare/v0.1.10...v0.1.11) - 2026-04-02

## [0.1.10](https://github.com/rvben/homeassistant-cli/compare/v0.1.9...v0.1.10) - 2026-04-02

### Fixed

- rename PyPI package to homeassistantcli ([32e531b](https://github.com/rvben/homeassistant-cli/commit/32e531bfc72935e1fb988ccc62f1cab7192331e2))

## [0.1.9](https://github.com/rvben/homeassistant-cli/compare/v0.1.8...v0.1.9) - 2026-04-02

### Fixed

- rename PyPI package to ha-cli ([675308b](https://github.com/rvben/homeassistant-cli/commit/675308bf877b0fb4e7f38e7062f9a75bc38c4f05))

## [0.1.8](https://github.com/rvben/homeassistant-cli/compare/v0.1.7...v0.1.8) - 2026-04-02

## [0.1.7](https://github.com/rvben/homeassistant-cli/compare/v0.1.6...v0.1.7) - 2026-04-02

### Added

- add PyPI distribution via maturin ([6a49bdd](https://github.com/rvben/homeassistant-cli/commit/6a49bddba90ac926ba1099d6f47c44a039343a50))

## [0.1.6](https://github.com/rvben/homeassistant-cli/compare/v0.1.5...v0.1.6) - 2026-04-02

### Added

- truncate table columns to fit terminal width ([f1506a6](https://github.com/rvben/homeassistant-cli/commit/f1506a67a890d8e7dc180950adf8dbbed9f58cc8))

## [0.1.5](https://github.com/rvben/homeassistant-cli/compare/v0.1.4...v0.1.5) - 2026-04-02

### Added

- add entity filtering, shell completions, and improved output ([536bcf9](https://github.com/rvben/homeassistant-cli/commit/536bcf9d7bdc12c965c2aa51ff762d247dbca6fd))

## [0.1.4](https://github.com/rvben/homeassistant-cli/compare/v0.1.3...v0.1.4) - 2026-04-02

### Added

- shell completions, friendly name in entity list, improved service list ([523fe36](https://github.com/rvben/homeassistant-cli/commit/523fe368f3dd2b60a93a767f249aa8ecddec61d2))

## [0.1.3](https://github.com/rvben/homeassistant-cli/compare/v0.1.2...v0.1.3) - 2026-04-02

### Added

- colorize table output — bold headers, state colors, dim domain prefix and old timestamps ([29813e1](https://github.com/rvben/homeassistant-cli/commit/29813e1fdf3d414f71e359bdedcf73aeea3fbbd0))

## [0.1.2](https://github.com/rvben/homeassistant-cli/compare/v0.1.1...v0.1.2) - 2026-04-02

### Added

- show relative timestamps in human output (entity list/watch) ([0f85059](https://github.com/rvben/homeassistant-cli/commit/0f8505900c11a0044c7ba80ebb822c6d6e023ebe))
- **init**: show token creation URL after host is entered ([f298972](https://github.com/rvben/homeassistant-cli/commit/f29897292055766703a1415497d59c04499c1293))

## [0.1.1] - 2026-04-02

### Added

- emit JSON error envelope in JSON output mode ([f54c937](https://github.com/rvben/homeassistant-cli/commit/f54c937409b91ebe4d258e705e98229aecbe7469))
- add config show and set commands ([8ff6ecb](https://github.com/rvben/homeassistant-cli/commit/8ff6ecb737ee13bd04c3227116353baccb7eac61))
- add schema command with full machine-readable CLI description ([4f2cd01](https://github.com/rvben/homeassistant-cli/commit/4f2cd013ab121446d192c27f766dbc1f0597e514))
- add event fire and watch commands ([40991ea](https://github.com/rvben/homeassistant-cli/commit/40991ea56c3d1b804abd02bfc86185fe4c127203))
- add service call and list commands ([17f0779](https://github.com/rvben/homeassistant-cli/commit/17f07790fc2605470c7ef05af0963d4ca664c923))
- add entity get, list, and watch commands ([dc0c26a](https://github.com/rvben/homeassistant-cli/commit/dc0c26a417423fbfa61929a8bd2b965144150586))
- add init command with interactive setup and JSON schema mode ([ec3a878](https://github.com/rvben/homeassistant-cli/commit/ec3a87816f8cd70f5b3debb9abcc9010b58de1bd))
- add event fire_event and SSE watch_stream ([36a50d2](https://github.com/rvben/homeassistant-cli/commit/36a50d2b59b3ed7e12802b604d50923e4c1f32c0))
- add service list_services and call_service API methods ([176a397](https://github.com/rvben/homeassistant-cli/commit/176a397bd9dc695c1d49e250e505eede4491ed2a))
- add entity get_state and list_states API methods ([209146a](https://github.com/rvben/homeassistant-cli/commit/209146a6e66f0e8087a6f44daba8fdf2c5e72b78))
- implement HaClient with Bearer auth, validate, and API types ([729ad33](https://github.com/rvben/homeassistant-cli/commit/729ad3323cbb8a54aaf44bae23db6fb79d716e0a))
- add output module with table rendering, mask_credential, and exit codes ([d36dfdf](https://github.com/rvben/homeassistant-cli/commit/d36dfdfe0a9ac510c0dfce10391b8d2826954e67))
- add config module with profile loading and env var override ([8eb014c](https://github.com/rvben/homeassistant-cli/commit/8eb014c0d6868697bf883750028549331bb0ebb3))

### Fixed

- resolve clippy warnings and document unsafe invariants ([14d83f6](https://github.com/rvben/homeassistant-cli/commit/14d83f6f8a52b5d60abecc1468c828c57faff45b))
- auto-detect output format based on TTY in OutputConfig::new ([64f04e0](https://github.com/rvben/homeassistant-cli/commit/64f04e006c425528be50031903c98d3b23723295))
- correct HaError variants, OutputFormat, and exit codes to match spec ([93ae178](https://github.com/rvben/homeassistant-cli/commit/93ae178810fb9b4fbddb09fbd35bcffe687c7b41))
