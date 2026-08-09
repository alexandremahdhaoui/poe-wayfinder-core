# poe-wayfinder-core

The domain logic for poe-wayfinder. Path of Exile 1 and 2.

It parses item text into a structured item, then turns that item into a trade
site query.

## No I/O

This crate opens no socket, reads no file and touches no window. Everything it
needs from outside arrives through a trait in `src/adapter/`.

That makes the whole crate testable with plain `cargo test`.

## Usage

```rust
use poe_wayfinder_core::types::GameVersion;
use poe_wayfinder_core::controller::parse::text_to_sections;

let sections = text_to_sections(clipboard_text);
```

## Build

```sh
forge build
forge test-all
```

## Licence

Apache License 2.0, see `LICENSE`.

The parser, stat matcher and trade query builder in this crate are ports of
MIT licensed TypeScript from Awakened PoE Trade and Exiled Exchange 2. Their
notice, and what was taken and what changed, are in `NOTICE`.
