import { Connection, PublicKey } from "@solana/web3.js";
import {
  addCodecSizePrefix,
  fixCodecSize,
  getBooleanCodec,
  getBytesCodec,
  getI64Codec,
  getStructCodec,
  getU32Codec,
  getU64Codec,
  getU8Codec,
  getUtf8Codec,
  transformCodec,
} from "@solana/codecs";

export interface StablecoinConfigAccountData {
  mint: PublicKey;
  authority: PublicKey;
  name: string;
  symbol: string;
  uri: string;
  decimals: number;
  enablePermanentDelegate: boolean;
  enableTransferHook: boolean;
  defaultAccountFrozen: boolean;
  paused: boolean;
  totalMinted: bigint;
  totalBurned: bigint;
  createdAt: bigint;
  lastChangedBy: PublicKey;
  lastChangedAt: bigint;
  bump: number;
}

export interface StablecoinConfigAccount {
  address: PublicKey;
  data: StablecoinConfigAccountData;
}

const StablecoinConfigAccountDataCodec = getStructCodec([
  ["discriminator", fixCodecSize(getBytesCodec(), 8)],
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
  ["name", addCodecSizePrefix(getUtf8Codec(), getU32Codec())],
  ["symbol", addCodecSizePrefix(getUtf8Codec(), getU32Codec())],
  ["uri", addCodecSizePrefix(getUtf8Codec(), getU32Codec())],
  ["decimals", getU8Codec()],
  ["enablePermanentDelegate", getBooleanCodec()],
  ["enableTransferHook", getBooleanCodec()],
  ["defaultAccountFrozen", getBooleanCodec()],
  ["paused", getBooleanCodec()],
  ["totalMinted", getU64Codec()],
  ["totalBurned", getU64Codec()],
  ["createdAt", getI64Codec()],
  [
    "lastChangedBy",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  ["lastChangedAt", getI64Codec()],
  ["bump", getU8Codec()],
]);

export function deserializeStablecoinConfigAccount(
  data: Uint8Array,
): StablecoinConfigAccountData {
  const deserialized = StablecoinConfigAccountDataCodec.decode(data);
  const { discriminator: _, ...accountData } = deserialized;
  return accountData as StablecoinConfigAccountData;
}

export async function fetchStablecoinConfigAccount(
  connection: Connection,
  address: PublicKey,
): Promise<StablecoinConfigAccount> {
  const accountInfo = await connection.getAccountInfo(address);
  if (!accountInfo) {
    throw new Error(
      "StablecoinConfig account not found at address: " + address.toBase58(),
    );
  }
  return {
    address,
    data: deserializeStablecoinConfigAccount(accountInfo.data),
  };
}

export async function fetchAllMaybeStablecoinConfigAccounts(
  connection: Connection,
  addresses: PublicKey[],
): Promise<(StablecoinConfigAccount | null)[]> {
  const accountInfos = await connection.getMultipleAccountsInfo(addresses);
  return accountInfos.map((accountInfo, index) => {
    if (!accountInfo) {
      return null;
    }
    return {
      address: addresses[index],
      data: deserializeStablecoinConfigAccount(accountInfo.data),
    };
  });
}

export async function fetchAllStablecoinConfigAccounts(
  connection: Connection,
  addresses: PublicKey[],
): Promise<StablecoinConfigAccount[]> {
  const maybeAccounts = await fetchAllMaybeStablecoinConfigAccounts(
    connection,
    addresses,
  );
  const missingAddresses = maybeAccounts
    .flatMap((account, i) => (!account ? [addresses[i].toBase58()] : []))
    .join(", ");
  if (missingAddresses) {
    throw new Error(
      "StablecoinConfig account(s) not found at address(es): " +
        missingAddresses,
    );
  }
  return maybeAccounts.filter((a): a is StablecoinConfigAccount => a !== null);
}

export async function fetchProgramAccountsStablecoinConfig(
  connection: Connection,
  programId: PublicKey,
  options?: { commitment?: "processed" | "confirmed" | "finalized" },
): Promise<StablecoinConfigAccount[]> {
  const accounts = await connection.getProgramAccounts(programId, {
    commitment: options?.commitment,
    filters: [{ memcmp: { offset: 0, bytes: "NG3UhbE1ZTK" } }],
  });
  return accounts.map(({ pubkey, account }) => ({
    address: pubkey,
    data: deserializeStablecoinConfigAccount(account.data),
  }));
}
