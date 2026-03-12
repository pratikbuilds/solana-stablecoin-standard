import { PublicKey } from "@solana/web3.js";
import {
  fixCodecSize,
  getBytesCodec,
  getStructCodec,
  getU64Codec,
  transformCodec,
} from "@solana/codecs";

export interface TokensBurned {
  mint: PublicKey;
  from: PublicKey;
  authority: PublicKey;
  amount: bigint;
}

export const tokensBurnedCodec = getStructCodec([
  [
    "mint",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  [
    "from",
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
