import { Connection, PublicKey } from "@solana/web3.js";
import {
  fixCodecSize,
  getBytesCodec,
  getStructCodec,
  getU8Codec,
  transformCodec,
} from "@solana/codecs";

export interface RoleConfigAccountData {
  mint: PublicKey;
  masterAuthority: PublicKey;
  pauser: PublicKey;
  burner: PublicKey;
  blacklister: PublicKey;
  seizer: PublicKey;
  bump: number;
}

export interface RoleConfigAccount {
  address: PublicKey;
  data: RoleConfigAccountData;
}

const RoleConfigAccountDataCodec = getStructCodec([
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
    "masterAuthority",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  [
    "pauser",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  [
    "burner",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  [
    "blacklister",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  [
    "seizer",
    transformCodec(
      fixCodecSize(getBytesCodec(), 32),
      (value: PublicKey) => value.toBytes(),
      (value) => new PublicKey(value),
    ),
  ],
  ["bump", getU8Codec()],
]);

export function deserializeRoleConfigAccount(
  data: Uint8Array,
): RoleConfigAccountData {
  const deserialized = RoleConfigAccountDataCodec.decode(data);
  const { discriminator: _, ...accountData } = deserialized;
  return accountData as RoleConfigAccountData;
}

export async function fetchRoleConfigAccount(
  connection: Connection,
  address: PublicKey,
): Promise<RoleConfigAccount> {
  const accountInfo = await connection.getAccountInfo(address);
  if (!accountInfo) {
    throw new Error(
      "RoleConfig account not found at address: " + address.toBase58(),
    );
  }
  return {
    address,
    data: deserializeRoleConfigAccount(accountInfo.data),
  };
}

export async function fetchAllMaybeRoleConfigAccounts(
  connection: Connection,
  addresses: PublicKey[],
): Promise<(RoleConfigAccount | null)[]> {
  const accountInfos = await connection.getMultipleAccountsInfo(addresses);
  return accountInfos.map((accountInfo, index) => {
    if (!accountInfo) {
      return null;
    }
    return {
      address: addresses[index],
      data: deserializeRoleConfigAccount(accountInfo.data),
    };
  });
}

export async function fetchAllRoleConfigAccounts(
  connection: Connection,
  addresses: PublicKey[],
): Promise<RoleConfigAccount[]> {
  const maybeAccounts = await fetchAllMaybeRoleConfigAccounts(
    connection,
    addresses,
  );
  const missingAddresses = maybeAccounts
    .flatMap((account, i) => (!account ? [addresses[i].toBase58()] : []))
    .join(", ");
  if (missingAddresses) {
    throw new Error(
      "RoleConfig account(s) not found at address(es): " + missingAddresses,
    );
  }
  return maybeAccounts.filter((a): a is RoleConfigAccount => a !== null);
}

export async function fetchProgramAccountsRoleConfig(
  connection: Connection,
  programId: PublicKey,
  options?: { commitment?: "processed" | "confirmed" | "finalized" },
): Promise<RoleConfigAccount[]> {
  const accounts = await connection.getProgramAccounts(programId, {
    commitment: options?.commitment,
    filters: [
      { memcmp: { offset: 0, bytes: "B8KLDRzjr8p" } },
      { dataSize: 201 },
    ],
  });
  return accounts.map(({ pubkey, account }) => ({
    address: pubkey,
    data: deserializeRoleConfigAccount(account.data),
  }));
}
