# Examples

A series of examples demonstrating what MCP servers built using kuri look like.

Available examples:
* A simple calculator tool server
* A stateful counter server
* A prompt server

## Running

Open up your favourite terminal and clone this git repository or download as a zip: `git clone git@github.com:itsaphel/modelcontextprotocol-rust-sdk.git`

Then you can either run examples from within this directory:

```bash
cd examples
cargo run --example 01_simple_calculator_tool_server
```

Or from the root kuri directory:

```bash
cargo run -p examples --example 01_simple_calculator_tool_server
```

The above command will start a server, which you can communicate with over your terminal's standard input (ie: paste directly into the terminal and press enter to send the message)

For example, to list tools:
```
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
```

To call the calculator tool with some arguments:
```
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"calculator","arguments":{"x":1,"y":2,"operation":"add"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"calculator","arguments":{"x":4,"y":5,"operation":"multiply"}}}
```

## Connecting to Claude Desktop (and other MCP clients)

You can also connect the server with an MCP client, such as Claude Desktop or Cursor. First compile the example: `cargo build -p examples --example 01_simple_calculator_tool_server`

Then modify the Claude Desktop config file to execute the compiled binary to run the tool:

```
  "mcpServers": {
    "calculator": {
      "command": "/bin/bash",
      "args": [
        "-c",
        "./path/to/kuri/target/debug/examples/01_simple_calculator_tool_server"
      ],
      "env": {
        "RUST_LOG": "debug"
      }
    }
  }
```

You can now interact with the server by using a prompt like:
> Add 2 and 3 together using the calculator tool, then multiply by 15. Return the final result. Use separate tool invocations for each sub calculation.


### A note on logging

When using the stdin transport, the MCP client communicates with the server over stdin/stdout. So we can't use these channels for logging. See [the logging doc](docs/LOGGING.md) for more information.

## Inspecting traffic with `npx @modelcontextprotocol/inspector`

You can also use the [modelcontextprotocol inspector](https://github.com/modelcontextprotocol/inspector) to inspect and debug the MCP server.

Compile the server as above, then run using: `npx @modelcontextprotocol/inspector ./target/debug/examples/01_simple_calculator_tool_server`
