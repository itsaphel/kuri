# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/itsaphel/kuri/compare/kuri-v0.1.0...kuri-v0.2.0) - 2025-05-07

### Other

- Example with arbitrary enum in tool handler
- cleanup integration tests, add test for enums in tool handlers
- Server description -> instructions, and make instructions optional
- Update README example, add ability to return String errors from tool handlers
- Notification handlers
- rename JSON-RPC structs
- Support JSON-RPC batching and improve error handling
- split integration tests into separate files
- `Tool`/`Prompt`/`Resource` -> `ToolMeta`/`PromptMeta`/`ResourceMeta` use aliases
- `context.rs` and `handler.rs`
- `is_error: Option<bool>` -> `is_error: bool`
- Error handling improvements/cleanups
- extract repeated code
- Add and improve integration tests
- Tower layer documentation + example (tracing middleware)
- `lib.rs` documentation
- Increase flexibility in tool handler return types (adds a `IntoCallToolResponse`)
- README and CI updates
- Transport rewrite ([#1](https://github.com/itsaphel/kuri/pull/1))
