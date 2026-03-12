# Tests

Devnet integration coverage lives under `tests/devnet/`.

## Prerequisites

1. Fund `~/.config/solana/id.json` on Devnet.
2. Deploy the current programs and refresh `tests/devnet/fixtures/program-ids.json`:

```bash
yarn test:devnet:deploy
```

## Commands

Run the full Devnet suite:

```bash
yarn test:devnet
```

Run only preset config coverage:

```bash
yarn test:devnet --grep "devnet preset config"
```

Run only scenario flows:

```bash
yarn test:devnet --grep "SSS-1 devnet flow"
yarn test:devnet --grep "SSS-2 devnet flow"
```

Run the bounded stress suite:

```bash
DEVNET_STRESS_ITERATIONS=2 yarn test:devnet:stress
```
