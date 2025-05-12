# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/itsaphel/kuri/compare/v0.1.0...kuri-v0.2.0) - 2025-05-12

- Transport rewrite ([#1](https://github.com/itsaphel/kuri/pull/1))
- Increase flexibility in tool handler return types (adds a `IntoCallToolResponse`)
- Server `description` -> `instructions`, and make instructions optional
- Notification handlers
- Support JSON-RPC batching
- Rename JSON-RPC structs
- Error handling improvements

### Internal and documentation

- Documentation (`lib.rs`, README, etc)
- Tower layer documentation + example (tracing middleware)
- Integration testing improvements
- `is_error: Option<bool>` -> `is_error: bool`
- Various code improvements
