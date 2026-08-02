# poe-trader-core

The domain logic for poe-trader. Path of Exile 1 and 2.

It parses item text into a structured item, then turns that item into a trade
site query.

## No I/O

This crate opens no socket, reads no file and touches no window. Everything it
needs from outside arrives through a trait in `src/adapter/`.

That makes the whole crate testable with plain `cargo test`.

## Usage

```rust
use poe_trader_core::types::GameVersion;
use poe_trader_core::controller::parse::text_to_sections;

let sections = text_to_sections(clipboard_text);
```

## Build

```sh
forge build
forge test-all
```
