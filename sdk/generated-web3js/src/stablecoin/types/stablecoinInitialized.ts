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

export interface StablecoinInitialized {
  mint: PublicKey;
  authority: PublicKey;
  preset: string;
}

export const stablecoinInitializedCodec = getStructCodec([
  [
    "mint",
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
  ["preset", addCodecSizePrefix(getUtf8Codec(), getU32Codec())],
]);
