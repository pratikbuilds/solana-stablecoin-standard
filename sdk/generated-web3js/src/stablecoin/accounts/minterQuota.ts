import { Connection, PublicKey } from "@solana/web3.js";
import {
  fixCodecSize,
  getBooleanCodec,
  getBytesCodec,
  getI64Codec,
  getStructCodec,
  getU64Codec,
  getU8Codec,
  transformCodec,
} from "@solana/codecs";

export interface MinterQuotaAccountData {
  mint: PublicKey;
  minter: PublicKey;
  quota: bigint;
  minted: bigint;
  active: boolean;
  createdAt: bigint;
  bump: number;
}

export interface MinterQuotaAccount {
  address: PublicKey;
  data: MinterQuotaAccountData;
}

const MinterQuotaAccountDataCodec = getStructCodec([
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
    "minter",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  ["quota", getU64Codec()],
  ["minted", getU64Codec()],
  ["active", getBooleanCodec()],
  ["createdAt", getI64Codec()],
  ["bump", getU8Codec()],
]);

export function deserializeMinterQuotaAccount(
  data: Uint8Array,
): MinterQuotaAccountData {
  const deserialized = MinterQuotaAccountDataCodec.decode(data);
  const { discriminator: _, ...accountData } = deserialized;
  return accountData as MinterQuotaAccountData;
}

export async function fetchMinterQuotaAccount(
  connection: Connection,
  address: PublicKey,
): Promise<MinterQuotaAccount> {
  const accountInfo = await connection.getAccountInfo(address);
  if (!accountInfo) {
    throw new Error(
      "MinterQuota account not found at address: " + address.toBase58(),
    );
  }
  return {
    address,
    data: deserializeMinterQuotaAccount(accountInfo.data),
  };
}

export async function fetchAllMaybeMinterQuotaAccounts(
  connection: Connection,
  addresses: PublicKey[],
): Promise<(MinterQuotaAccount | null)[]> {
  const accountInfos = await connection.getMultipleAccountsInfo(addresses);
  return accountInfos.map((accountInfo, index) => {
    if (!accountInfo) {
      return null;
    }
    return {
      address: addresses[index],
      data: deserializeMinterQuotaAccount(accountInfo.data),
    };
  });
}

export async function fetchAllMinterQuotaAccounts(
  connection: Connection,
  addresses: PublicKey[],
): Promise<MinterQuotaAccount[]> {
  const maybeAccounts = await fetchAllMaybeMinterQuotaAccounts(
    connection,
    addresses,
  );
  const missingAddresses = maybeAccounts
    .flatMap((account, i) => (!account ? [addresses[i].toBase58()] : []))
    .join(", ");
  if (missingAddresses) {
    throw new Error(
      "MinterQuota account(s) not found at address(es): " + missingAddresses,
    );
  }
  return maybeAccounts.filter((a): a is MinterQuotaAccount => a !== null);
}

export async function fetchProgramAccountsMinterQuota(
  connection: Connection,
  programId: PublicKey,
  options?: { commitment?: "processed" | "confirmed" | "finalized" },
): Promise<MinterQuotaAccount[]> {
  const accounts = await connection.getProgramAccounts(programId, {
    commitment: options?.commitment,
    filters: [
      { memcmp: { offset: 0, bytes: "83FeYWPdRW8" } },
      { dataSize: 98 },
    ],
  });
  return accounts.map(({ pubkey, account }) => ({
    address: pubkey,
    data: deserializeMinterQuotaAccount(account.data),
  }));
}
