kuri uses tokio's tracing throughout for log messages. Typically, applications might consume these messages to stdout:

```
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .with_target(false)
    .with_thread_ids(true)
    .with_file(true)
    .with_line_number(true)
    .init();
```

However, when using the stdin transport to communicate with the client, we are unable to log messages to stdout, as discussed in [the MCP docs](https://modelcontextprotocol.io/docs/tools/debugging#server-side-logging). There are some recommended alternatives:

1. Log to file

Change the writer to write to a file, or some other source, as follows. We take this approach in the [examples](/examples/src/).
```
let file_appender = tracing_appender::rolling::daily(tempfile::tempdir()?, "server.log");
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .with_writer(file_appender)
    .with_target(false)
    .with_thread_ids(true)
    .with_file(true)
    .with_line_number(true)
    .init();
```

2. Log messages to the client, by sending notifications over the MCP protocol

This is currently unsupported in kuri.
