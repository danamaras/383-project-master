This project contains a Rust implementation of a RabbitMQ-based remote procedure
call using the lapin crate.

The RPC arguments are sent as a `types::Args` struct, JSON-encoded with serde-json.
The response comes as a `types::Result`, also JSON encoded.

Both the client and server are included here, to minimize code duplication. The
`main()` function switches depending on the command line. You can run either tool
with:
```
cargo run server
cargo run client
```

The code does reasonable things to handle errors from the client/server, but not
problems with the AMQP connection itself. i.e. if the rabbitmq server disappears, the
code will panic.
