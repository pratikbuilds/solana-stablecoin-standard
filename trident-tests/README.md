# Trident Tests

Program-focused fuzz coverage belongs here.

The intended target areas are:

- preset initialization invariants
- minter quota and admin update flows
- pause gates on mint/burn/seize flows
- blacklist and seize account-shape invariants
- supply, balance, and counter conservation across stateful flows

If `cargo trident` is installed locally, use:

```bash
cd trident-tests
cargo trident fuzz run fuzz_0
```

If the command is missing, install the CLI first:

```bash
cargo install trident-cli
```

For a single deterministic debug iteration that is easier to reproduce while developing the harness:

```bash
cd trident-tests
TRIDENT_FUZZ_DEBUG=0000000000000000000000000000000000000000000000000000000000000000 cargo run --bin fuzz_0
```
