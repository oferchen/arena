# arena

## Prerequisites

Install the WebAssembly toolchain:

```
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

## Building

### Client

```
cd client
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir ../web/pkg target/wasm32-unknown-unknown/debug/client.wasm
```

To enable the WebGL2 fallback, build the client with the `webgl2` feature:

```
cargo build --target wasm32-unknown-unknown --features webgl2
```

### Server

From the repository root:

```
cargo run -p xtask
cargo run -p server
```

For details on the networking model see the [Netcode guide](docs/netcode.md). To extend gameplay, follow the [Module guide](docs/modules.md) or the example [Duck Hunt module](docs/DuckHunt.md). Email configuration is covered in the [Email guide](docs/Email.md).

## Running

After building, open `http://localhost:3000` in a browser to enter the lobby.

The server must be served with the following headers:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`

These enable cross-origin isolation required by the client.

For deployment and operational details, consult the [Operations guide](docs/ops.md).

## Development

Run Prettier before committing changes:

```
npm run prettier
```

Additional resources:

- [Module development](docs/modules.md)
- [Duck Hunt module](docs/DuckHunt.md)
- [Netcode design](docs/netcode.md)
- [Deployment and operations](docs/ops.md)
- [Email configuration](docs/Email.md)

## Documentation

Documentation lives under `docs/`:

- [Netcode](docs/netcode.md)
- [Operations](docs/ops.md)
- [Modules](docs/modules.md)
- [Duck Hunt](docs/DuckHunt.md)
- [Email](docs/Email.md)
