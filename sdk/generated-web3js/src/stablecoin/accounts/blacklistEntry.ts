import { Connection, PublicKey } from "@solana/web3.js";
import {
  addCodecSizePrefix,
  fixCodecSize,
  getBytesCodec,
  getI64Codec,
  getStructCodec,
  getU32Codec,
  getU8Codec,
  getUtf8Codec,
  transformCodec,
} from "@solana/codecs";

export interface BlacklistEntryAccountData {
  mint: PublicKey;
  wallet: PublicKey;
  reason: string;
  blacklistedBy: PublicKey;
  blacklistedAt: bigint;
  bump: number;
}

export interface BlacklistEntryAccount {
  address: PublicKey;
  data: BlacklistEntryAccountData;
}

const BlacklistEntryAccountDataCodec = getStructCodec([
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
    "wallet",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  ["reason", addCodecSizePrefix(getUtf8Codec(), getU32Codec())],
  [
    "blacklistedBy",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  ["blacklistedAt", getI64Codec()],
  ["bump", getU8Codec()],
]);

export function deserializeBlacklistEntryAccount(
  data: Uint8Array,
): BlacklistEntryAccountData {
  const deserialized = BlacklistEntryAccountDataCodec.decode(data);
  const { discriminator: _, ...accountData } = deserialized;
  return accountData as BlacklistEntryAccountData;
}

export async function fetchBlacklistEntryAccount(
  connection: Connection,
  address: PublicKey,
): Promise<BlacklistEntryAccount> {
  const accountInfo = await connection.getAccountInfo(address);
  if (!accountInfo) {
    throw new Error(
      "BlacklistEntry account not found at address: " + address.toBase58(),
    );
  }
  return {
    address,
    data: deserializeBlacklistEntryAccount(accountInfo.data),
  };
}

export async function fetchAllMaybeBlacklistEntryAccounts(
  connection: Connection,
  addresses: PublicKey[],
): Promise<(BlacklistEntryAccount | null)[]> {
  const accountInfos = await connection.getMultipleAccountsInfo(addresses);
  return accountInfos.map((accountInfo, index) => {
    if (!accountInfo) {
      return null;
    }
    return {
      address: addresses[index],
      data: deserializeBlacklistEntryAccount(accountInfo.data),
    };
  });
}

export async function fetchAllBlacklistEntryAccounts(
  connection: Connection,
  addresses: PublicKey[],
): Promise<BlacklistEntryAccount[]> {
  const maybeAccounts = await fetchAllMaybeBlacklistEntryAccounts(
    connection,
    addresses,
  );
  const missingAddresses = maybeAccounts
    .flatMap((account, i) => (!account ? [addresses[i].toBase58()] : []))
    .join(", ");
  if (missingAddresses) {
    throw new Error(
      "BlacklistEntry account(s) not found at address(es): " + missingAddresses,
    );
  }
  return maybeAccounts.filter((a): a is BlacklistEntryAccount => a !== null);
}

export async function fetchProgramAccountsBlacklistEntry(
  connection: Connection,
  programId: PublicKey,
  options?: { commitment?: "processed" | "confirmed" | "finalized" },
): Promise<BlacklistEntryAccount[]> {
  const accounts = await connection.getProgramAccounts(programId, {
    commitment: options?.commitment,
    filters: [{ memcmp: { offset: 0, bytes: "dah4A9skJ4L" } }],
  });
  return accounts.map(({ pubkey, account }) => ({
    address: pubkey,
    data: deserializeBlacklistEntryAccount(account.data),
  }));
}
