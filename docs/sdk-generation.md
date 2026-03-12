# SDK Generation

How to regenerate the Codama SDKs for Kit and web3.js from Anchor IDLs.

## Prerequisites

- `anchor build` must have run at least once (IDLs live in `target/idl/`)
- Dependencies installed (`yarn install`)

## Quick Regenerate

```bash
anchor build
yarn generate
```

Or use the full build (which runs generate automatically):

```bash
yarn build
```

## What Gets Generated

| Package           | Renderer                    | Output                          |
|------------------|-----------------------------|---------------------------------|
| `sdk/generated-kit`    | `@codama/renderers-js`      | `@solana/kit` clients           |
| `sdk/generated-web3js` | `@pratikbuilds/web3js-legacy` | `@solana/web3.js` clients    |

Both packages include generated code for:

- `stablecoin` program
- `transfer_hook` program

## Pipeline

1. **`scripts/generate-sdks.mjs`** — Runs Codama for each IDL:
   - Reads `target/idl/stablecoin.json` and `target/idl/transfer_hook.json`
   - Converts Anchor IDL → Codama tree via `@codama/nodes-from-anchor`
   - Renders Kit clients → `sdk/generated-kit/src/{stablecoin,transfer-hook}/`
   - Renders web3.js clients → `sdk/generated-web3js/src/{stablecoin,transfer-hook}/`

2. **`scripts/patch-generated.mjs`** — Fixes known renderer issues:
   - generated-kit: transfer-hook naming conflicts (ParsedTransferHookInstruction, parseTransferHookInstruction)
   - generated-web3js: MinterQuotaPdaSeeds uses `minter` (update_minter) vs `authority` (mint)

## When to Regenerate

- After changing Anchor program instructions, accounts, or types
- After `anchor build` updates the IDLs

## Build After Regenerate

```bash
yarn build:ts
```

Or run the full pipeline:

```bash
yarn build
yarn verify
```
