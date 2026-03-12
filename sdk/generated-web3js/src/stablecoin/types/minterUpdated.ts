import { PublicKey } from "@solana/web3.js";
import {
  fixCodecSize,
  getBooleanCodec,
  getBytesCodec,
  getStructCodec,
  getU64Codec,
  transformCodec,
} from "@solana/codecs";

export interface MinterUpdated {
  mint: PublicKey;
  minter: PublicKey;
  quota: bigint;
  active: boolean;
}

export const minterUpdatedCodec = getStructCodec([
  [
    "mint",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  [
    "minter",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  ["quota", getU64Codec()],
  ["active", getBooleanCodec()],
]);
