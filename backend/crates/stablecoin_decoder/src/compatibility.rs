use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

pub fn next_account<'a, I>(iter: &mut I) -> Option<Pubkey>
where
    I: Iterator<Item = &'a AccountMeta>,
{
    iter.next().map(|account| account.pubkey)
}
