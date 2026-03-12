import { PublicKey } from "@solana/web3.js";
import {
  fixCodecSize,
  getBytesCodec,
  getStructCodec,
  getU64Codec,
  transformCodec,
} from "@solana/codecs";

export interface TokensMinted {
  mint: PublicKey;
  to: PublicKey;
  authority: PublicKey;
  amount: bigint;
}

export const tokensMintedCodec = getStructCodec([
  [
    "mint",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  [
    "to",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  [
    "authority",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  ["amount", getU64Codec()],
]);
