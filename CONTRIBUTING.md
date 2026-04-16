# Contributing to soroban-gmp-hub

## Prerequisites

| Tool | Version |
|------|---------|
| Rust | stable (≥ 1.78) |
| `wasm32-unknown-unknown` target | `rustup target add wasm32-unknown-unknown` |
| Stellar CLI | ≥ 0.9 (`cargo install --locked stellar-cli`) |

## Development setup

```sh
git clone https://github.com/<your-org>/soroban-gmp-hub.git
cd soroban-gmp-hub
cargo build
```

## Running tests

```sh
cargo test -p soroban-gmp-hub
```

To see emitted events and diagnostic output:

```sh
cargo test -p soroban-gmp-hub -- --nocapture
```

## Building the WASM artifact

```sh
stellar contract build
# Output: target/wasm32-unknown-unknown/release/soroban_gmp_hub.wasm
```

## Git workflow

- Branch from `main`: `git checkout -b feat/<short-description>`
- Keep commits atomic and descriptive.
- All PRs must pass `cargo test` and `cargo clippy -- -D warnings`.
- Squash-merge into `main` after review.

## Testing requirements

- Every new public contract function must have at least one positive and one negative test.
- Replay-protection paths must be explicitly tested.
- Tests must not use `unwrap()` on external data — propagate errors with `?` or assert with `is_err()`.

## Code style

```sh
cargo fmt --all         # format
cargo clippy -- -D warnings  # lint
```
