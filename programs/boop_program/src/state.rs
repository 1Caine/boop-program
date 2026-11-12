use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub pending_admin: Pubkey,
    pub treasury: Pubkey,
    pub tag_fee: u64,
    pub fee_bps: u16,
    pub payments_enabled: bool,
    pub tags_enabled: bool,
    pub accounts_enabled: bool,
    pub vaults_enabled: bool,
    pub version: u8,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Tag {
    pub owner: Pubkey,
    #[max_len(20)]
    pub name: String,
    /// Zero means unlisted. Listings are part of the tag, not a separate rent-funded account.
    pub price: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct TagOwner {
    pub owner: Pubkey,
    pub default_tag: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Savings {
    pub owner: Pubkey,
    pub available: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Goal {
    pub creator: Pubkey,
    pub id: u64,
    #[max_len(32)]
    pub name: String,
    pub target: u64,
    pub available: u64,
    pub members: u32,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct GoalPosition {
    pub goal: Pubkey,
    pub member: Pubkey,
    pub available: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub owner: Pubkey,
    pub recovery: Pubkey,
    pub cosigner: Pubkey,
    pub available: u64,
    pub nonce: u64,
    pub daily_limit: u64,
    pub day: i64,
    pub spent_today: u64,
    pub bump: u8,
}

/// One stable, versioned event format for activity, money and ownership changes.
/// Amounts are always base units. Zero mint means a non-monetary action.
#[event]
pub struct Activity {
    pub version: u8,
    pub kind: u8,
    pub actor: Pubkey,
    pub subject: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub fee: u64,
    pub reference: [u8; 32],
}

pub fn activity(
    kind: u8,
    actor: Pubkey,
    subject: Pubkey,
    recipient: Pubkey,
    mint: Pubkey,
    amount: u64,
    fee: u64,
    reference: [u8; 32],
) {
    emit!(Activity {
        version: 1,
        kind,
        actor,
        subject,
        recipient,
        mint,
        amount,
        fee,
        reference
    });
}

// Payment kinds 0..=3 match direct / request / split / business.
pub const TAG_REGISTERED: u8 = 10;
pub const TAG_DEFAULT: u8 = 11;
pub const TAG_TRANSFERRED: u8 = 12;
pub const TAG_LISTED: u8 = 13;
pub const TAG_CANCELLED: u8 = 14;
pub const TAG_PURCHASED: u8 = 15;
pub const SAVINGS_DEPOSIT: u8 = 20;
pub const SAVINGS_WITHDRAW: u8 = 21;
pub const GOAL_CREATED: u8 = 30;
pub const GOAL_JOINED: u8 = 31;
pub const GOAL_DEPOSIT: u8 = 32;
pub const GOAL_WITHDRAW: u8 = 33;
pub const VAULT_CREATED: u8 = 40;
pub const VAULT_DEPOSIT: u8 = 41;
pub const VAULT_WITHDRAW: u8 = 42;
pub const CONFIGURED: u8 = 50;
pub const ADMIN_PROPOSED: u8 = 51;
pub const ADMIN_ACCEPTED: u8 = 52;
