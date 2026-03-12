import { PublicKey } from "@solana/web3.js";
import {
  addCodecSizePrefix,
  fixCodecSize,
  getBytesCodec,
  getStructCodec,
  getU32Codec,
  getUtf8Codec,
  transformCodec,
} from "@solana/codecs";

export interface AddressBlacklisted {
  mint: PublicKey;
  wallet: PublicKey;
  authority: PublicKey;
  reason: string;
}

export const addressBlacklistedCodec = getStructCodec([
  [
    "mint",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  [
    "wallet",
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
  ["reason", addCodecSizePrefix(getUtf8Codec(), getU32Codec())],
]);
