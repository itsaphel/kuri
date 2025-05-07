# kuri_mcp_protocol

[![Crates.io](https://img.shields.io/crates/v/kuri_mcp_protocol)](https://crates.io/crates/kuri_mcp_protocol)
[![Documentation](https://docs.rs/kuri_mcp_protocol/badge.svg)](https://docs.rs/kuri_mcp_protocol)

This crate contains types for the MCP protocol, and handlers to enable serde
serialisation/deserialisation from these types.

This crate is intended to be independent of kuri's implementation and usage of these types, so you
may use `kuri_mcp_protocol` by itself in your project, if you only want the types and not the `kuri`
server framework.
