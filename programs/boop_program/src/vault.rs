use crate::{common::*, errors::BoopError, state::*};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(seeds=[b"config"],bump=config.bump,constraint=config.vaults_enabled @ BoopError::Disabled)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(init,payer=owner,space=8+Vault::INIT_SPACE,seeds=[b"vault",owner.key().as_ref()],bump)]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}
pub fn initialize(
    ctx: Context<InitializeVault>,
    recovery: Pubkey,
    cosigner: Pubkey,
    daily_limit: u64,
) -> Result<()> {
    let owner = ctx.accounts.owner.key();
    require!(
        valid_signers(owner, recovery, cosigner) && daily_limit > 0,
        BoopError::InvalidInput
    );
    let v = &mut ctx.accounts.vault;
    v.owner = owner;
    v.recovery = recovery;
    v.cosigner = cosigner;
    v.available = 0;
    v.nonce = 0;
    v.daily_limit = daily_limit;
    v.day = Clock::get()?.unix_timestamp.div_euclid(86_400);
    v.spent_today = 0;
    v.bump = ctx.bumps.vault;
    activity(
        VAULT_CREATED,
        owner,
        v.key(),
        recovery,
        USDC,
        daily_limit,
        0,
        cosigner.to_bytes(),
    );
    Ok(())
}

#[derive(Accounts)]
pub struct DepositVault<'info> {
    #[account(seeds=[b"config"],bump=config.bump,constraint=config.vaults_enabled @ BoopError::Disabled)]
    pub config: Account<'info, Config>,
    pub depositor: Signer<'info>,
    #[account(mut,seeds=[b"vault",vault.owner.as_ref()],bump=vault.bump)]
    pub vault: Account<'info, Vault>,
    #[account(address=USDC,constraint=mint.decimals==6 @ BoopError::InvalidMint)]
    pub mint: Account<'info, Mint>,
    #[account(mut,token::mint=mint,token::authority=depositor)]
    pub depositor_token: Account<'info, TokenAccount>,
    #[account(mut,token::mint=mint,token::authority=vault)]
    pub vault_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
pub fn deposit(ctx: Context<DepositVault>, amount: u64) -> Result<()> {
    let a = ctx.accounts;
    require!(amount > 0, BoopError::InvalidInput);
    a.vault.available = add(a.vault.available, amount)?;
    transfer(
        a.token_program.key(),
        a.depositor_token.to_account_info(),
        a.mint.to_account_info(),
        a.vault_token.to_account_info(),
        a.depositor.to_account_info(),
        amount,
        &[],
    )?;
    activity(
        VAULT_DEPOSIT,
        a.depositor.key(),
        a.vault.key(),
        a.vault.owner,
        USDC,
        amount,
        0,
        [0; 32],
    );
    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawVault<'info> {
    pub signer_a: Signer<'info>,
    pub signer_b: Signer<'info>,
    #[account(mut,seeds=[b"vault",vault.owner.as_ref()],bump=vault.bump)]
    pub vault: Account<'info, Vault>,
    /// CHECK: Explicit destination approved by both transaction signers, bound to destination_token.
    pub destination: UncheckedAccount<'info>,
    #[account(address=USDC,constraint=mint.decimals==6 @ BoopError::InvalidMint)]
    pub mint: Account<'info, Mint>,
    #[account(mut,token::mint=mint,token::authority=vault)]
    pub vault_token: Account<'info, TokenAccount>,
    #[account(mut,token::mint=mint,token::authority=destination)]
    pub destination_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
pub fn withdraw(ctx: Context<WithdrawVault>, amount: u64, nonce: u64, deadline: i64) -> Result<()> {
    let a = ctx.accounts;
    let now = Clock::get()?.unix_timestamp;
    require!(
        valid_recipient(a.vault.key(), a.destination.key()),
        BoopError::InvalidInput
    );
    authorize(
        &mut a.vault,
        a.signer_a.key(),
        a.signer_b.key(),
        amount,
        nonce,
        deadline,
        now,
    )?;
    let owner = a.vault.owner;
    let bump = [a.vault.bump];
    let seeds: &[&[u8]] = &[b"vault", owner.as_ref(), &bump];
    transfer(
        a.token_program.key(),
        a.vault_token.to_account_info(),
        a.mint.to_account_info(),
        a.destination_token.to_account_info(),
        a.vault.to_account_info(),
        amount,
        &[seeds],
    )?;
    activity(
        VAULT_WITHDRAW,
        a.signer_a.key(),
        a.vault.key(),
        a.destination.key(),
        USDC,
        amount,
        0,
        nonce_reference(nonce),
    );
    Ok(())
}
fn nonce_reference(nonce: u64) -> [u8; 32] {
    let mut value = [0; 32];
    value[..8].copy_from_slice(&nonce.to_le_bytes());
    value
}
pub fn valid_signers(owner: Pubkey, recovery: Pubkey, cosigner: Pubkey) -> bool {
    valid_recipient(owner, recovery) && valid_recipient(owner, cosigner) && recovery != cosigner
}
fn authorize(
    v: &mut Vault,
    a: Pubkey,
    b: Pubkey,
    amount: u64,
    nonce: u64,
    deadline: i64,
    now: i64,
) -> Result<()> {
    let member = |s| s == v.owner || s == v.recovery || s == v.cosigner;
    require!(a != b && member(a) && member(b), BoopError::Unauthorized);
    require!(
        nonce == v.nonce && deadline >= now && deadline <= now.saturating_add(3_600),
        BoopError::InvalidApproval
    );
    require!(amount > 0, BoopError::InvalidInput);
    let day = now.div_euclid(86_400);
    let spent = if day == v.day { v.spent_today } else { 0 };
    let next = add(spent, amount)?;
    require!(next <= v.daily_limit, BoopError::DailyLimit);
    let available = sub(v.available, amount)?;
    let next_nonce = add(v.nonce, 1)?;
    v.day = day;
    v.spent_today = next;
    v.available = available;
    v.nonce = next_nonce;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vault {
        Vault {
            owner: Pubkey::new_unique(),
            recovery: Pubkey::new_unique(),
            cosigner: Pubkey::new_unique(),
            available: 1_000,
            nonce: 0,
            daily_limit: 100,
            day: 1,
            spent_today: 0,
            bump: 1,
        }
    }
    #[test]
    fn every_pair_and_owner_unavailable_recovery() {
        for pair in [0, 1, 2] {
            let mut v = fixture();
            let signers = [v.owner, v.recovery, v.cosigner];
            authorize(
                &mut v,
                signers[pair],
                signers[(pair + 1) % 3],
                100,
                0,
                86_500,
                86_400,
            )
            .unwrap();
            assert_eq!(v.available, 900);
            assert_eq!(v.nonce, 1);
        }
    }
    #[test]
    fn replay_expiry_duplicates_unknown_signers_and_balance() {
        let mut v = fixture();
        let a = v.owner;
        let b = v.recovery;
        assert!(authorize(&mut v, a, a, 1, 0, 86_500, 86_400).is_err());
        assert!(authorize(&mut v, a, Pubkey::new_unique(), 1, 0, 86_500, 86_400).is_err());
        assert!(authorize(&mut v, a, b, 1, 0, 86_399, 86_400).is_err());
        assert!(authorize(&mut v, a, b, 1, 1, 86_500, 86_400).is_err());
        assert!(authorize(&mut v, a, b, 101, 0, 86_500, 86_400).is_err());
        authorize(&mut v, a, b, 100, 0, 86_500, 86_400).unwrap();
        assert!(authorize(&mut v, a, b, 1, 0, 86_500, 86_400).is_err());
        assert!(authorize(&mut v, a, b, 1, 1, 86_500, 86_400).is_err());
        authorize(&mut v, a, b, 50, 1, 172_900, 172_800).unwrap();
        assert_eq!(v.spent_today, 50);
        assert!(!valid_signers(a, b, b));
        assert!(!valid_signers(a, a, b));
    }
}
