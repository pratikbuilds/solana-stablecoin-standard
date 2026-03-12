import { PublicKey } from "@solana/web3.js";
import {
  fixCodecSize,
  getBooleanCodec,
  getBytesCodec,
  getStructCodec,
  transformCodec,
} from "@solana/codecs";

export interface PauseChanged {
  mint: PublicKey;
  paused: boolean;
  authority: PublicKey;
}

export const pauseChangedCodec = getStructCodec([
  [
    "mint",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  ["paused", getBooleanCodec()],
  [
    "authority",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
]);
