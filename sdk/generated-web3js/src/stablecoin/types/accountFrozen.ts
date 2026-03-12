import { PublicKey } from "@solana/web3.js";
import {
  fixCodecSize,
  getBytesCodec,
  getStructCodec,
  transformCodec,
} from "@solana/codecs";

export interface AccountFrozen {
  mint: PublicKey;
  account: PublicKey;
  authority: PublicKey;
}

export const accountFrozenCodec = getStructCodec([
  [
    "mint",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  [
    "account",
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
]);
