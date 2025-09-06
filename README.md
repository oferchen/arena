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

### Server

From the repository root:

```
cargo run -p xtask
cargo run -p server
```

## Running

After building, open `http://localhost:3000` in a browser to enter the lobby.

The server must be served with the following headers:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`

These enable cross-origin isolation required by the client.

## Development

Run Prettier before committing changes:

```
npm run prettier
```

## Documentation

Future documentation will live under `docs/`:

- [Netcode](docs/netcode.md)
- [Operations](docs/ops.md)
- [Modules](docs/modules.md)
