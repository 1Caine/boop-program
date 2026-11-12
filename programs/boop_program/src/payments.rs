use crate::{common::*, errors::BoopError, state::*};
use anchor_lang::{
    prelude::*,
    system_program::{self, Transfer},
};
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct PaySol<'info> {
    #[account(seeds=[b"config"],bump=config.bump,constraint=config.payments_enabled @ BoopError::Disabled)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Only credited; handler rejects self, zero and program addresses.
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
    /// CHECK: Treasury is pinned to config and only credited.
    #[account(mut,address=config.treasury)]
    pub treasury: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
pub fn pay_sol(
    ctx: Context<PaySol>,
    amount: u64,
    max_fee: u64,
    kind: u8,
    reference: [u8; 32],
) -> Result<()> {
    let a = &ctx.accounts;
    validate_payment(a.payer.key(), a.recipient.key(), kind, reference)?;
    let fee = payment_fee(amount, a.config.fee_bps)?;
    require!(fee <= max_fee, BoopError::PriceChanged);
    system_program::transfer(
        CpiContext::new(
            a.system_program.key(),
            Transfer {
                from: a.payer.to_account_info(),
                to: a.recipient.to_account_info(),
            },
        ),
        amount,
    )?;
    if fee > 0 {
        system_program::transfer(
            CpiContext::new(
                a.system_program.key(),
                Transfer {
                    from: a.payer.to_account_info(),
                    to: a.treasury.to_account_info(),
                },
            ),
            fee,
        )?;
    }
    activity(
        kind,
        a.payer.key(),
        a.config.key(),
        a.recipient.key(),
        system_program::ID,
        amount,
        fee,
        reference,
    );
    Ok(())
}

#[derive(Accounts)]
pub struct PayToken<'info> {
    #[account(seeds=[b"config"],bump=config.bump,constraint=config.payments_enabled @ BoopError::Disabled)]
    pub config: Account<'info, Config>,
    pub payer: Signer<'info>,
    /// CHECK: Bound to recipient_token's authority; handler validates destination.
    pub recipient: UncheckedAccount<'info>,
    #[account(constraint=supported(mint.key(),mint.decimals) @ BoopError::InvalidMint)]
    pub mint: Account<'info, Mint>,
    #[account(mut,token::mint=mint,token::authority=payer)]
    pub payer_token: Account<'info, TokenAccount>,
    #[account(mut,token::mint=mint,token::authority=recipient)]
    pub recipient_token: Account<'info, TokenAccount>,
    #[account(mut,token::mint=mint,constraint=treasury_token.owner==config.treasury @ BoopError::Unauthorized)]
    pub treasury_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
pub fn pay_token(
    ctx: Context<PayToken>,
    amount: u64,
    max_fee: u64,
    kind: u8,
    reference: [u8; 32],
) -> Result<()> {
    let a = &ctx.accounts;
    validate_payment(a.payer.key(), a.recipient.key(), kind, reference)?;
    let fee = payment_fee(amount, a.config.fee_bps)?;
    require!(fee <= max_fee, BoopError::PriceChanged);
    transfer(
        a.token_program.key(),
        a.payer_token.to_account_info(),
        a.mint.to_account_info(),
        a.recipient_token.to_account_info(),
        a.payer.to_account_info(),
        amount,
        &[],
    )?;
    transfer(
        a.token_program.key(),
        a.payer_token.to_account_info(),
        a.mint.to_account_info(),
        a.treasury_token.to_account_info(),
        a.payer.to_account_info(),
        fee,
        &[],
    )?;
    activity(
        kind,
        a.payer.key(),
        a.config.key(),
        a.recipient.key(),
        a.mint.key(),
        amount,
        fee,
        reference,
    );
    Ok(())
}
