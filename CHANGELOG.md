# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).


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
