import { PublicKey } from "@solana/web3.js";
import {
  fixCodecSize,
  getBytesCodec,
  getStructCodec,
  transformCodec,
} from "@solana/codecs";

export interface AccountThawed {
  mint: PublicKey;
  account: PublicKey;
  authority: PublicKey;
}

export const accountThawedCodec = getStructCodec([
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
