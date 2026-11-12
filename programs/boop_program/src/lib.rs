#![allow(deprecated, unexpected_cfgs, ambiguous_glob_reexports)]
use anchor_lang::prelude::*;

// The default build is the mainnet release. `--features devnet` builds the test-network
// program (its own program ID and the test-network USDC mint).

pub mod common;
pub mod errors;
pub mod payments;
pub mod savings_goals;
pub mod state;
pub mod tags;
pub mod vault;
use errors::BoopError;
pub use payments::*;
pub use savings_goals::*;
use state::*;
pub use tags::*;
pub use vault::*;

#[cfg(not(feature = "devnet"))]
declare_id!("AzskZwMoTeXbz6pDby3vNq3nthYWcT59D61pfLEoThwX");
#[cfg(feature = "devnet")]
declare_id!("2MAFKZZyVE3J5zjjZMSTdBXPL4B7R82AiBgM6eYFiaf9");

#[program]
pub mod boop_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, treasury: Pubkey) -> Result<()> {
        require!(
            common::valid_recipient(crate::ID, treasury),
            BoopError::InvalidInput
        );
        let c = &mut ctx.accounts.config;
        c.admin = ctx.accounts.admin.key();
        c.pending_admin = Pubkey::default();
        c.treasury = treasury;
        c.tag_fee = 100_000_000;
        c.fee_bps = 75;
        c.payments_enabled = true;
        c.tags_enabled = true;
        c.accounts_enabled = true;
        c.vaults_enabled = true;
        c.version = 2;
        c.bump = ctx.bumps.config;
        activity(
            CONFIGURED,
            c.admin,
            c.key(),
            treasury,
            common::USDC,
            c.tag_fee,
            75,
            [0; 32],
        );
        Ok(())
    }
    pub fn configure(
        ctx: Context<Admin>,
        treasury: Pubkey,
        fee_bps: u16,
        tag_fee: u64,
        payments: bool,
        tags: bool,
        accounts: bool,
        vaults: bool,
    ) -> Result<()> {
        require!(
            common::valid_recipient(crate::ID, treasury) && fee_bps <= common::MAX_FEE_BPS,
            BoopError::InvalidInput
        );
        let c = &mut ctx.accounts.config;
        c.treasury = treasury;
        c.fee_bps = fee_bps;
        c.tag_fee = tag_fee;
        c.payments_enabled = payments;
        c.tags_enabled = tags;
        c.accounts_enabled = accounts;
        c.vaults_enabled = vaults;
        activity(
            CONFIGURED,
            c.admin,
            c.key(),
            treasury,
            common::USDC,
            tag_fee,
            u64::from(fee_bps),
            [0; 32],
        );
        Ok(())
    }
    pub fn propose_admin(ctx: Context<Admin>, next: Pubkey) -> Result<()> {
        require!(
            common::valid_recipient(ctx.accounts.admin.key(), next),
            BoopError::InvalidInput
        );
        ctx.accounts.config.pending_admin = next;
        activity(
            ADMIN_PROPOSED,
            ctx.accounts.admin.key(),
            ctx.accounts.config.key(),
            next,
            Pubkey::default(),
            0,
            0,
            [0; 32],
        );
        Ok(())
    }
    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        ctx.accounts.config.admin = ctx.accounts.pending_admin.key();
        ctx.accounts.config.pending_admin = Pubkey::default();
        activity(
            ADMIN_ACCEPTED,
            ctx.accounts.pending_admin.key(),
            ctx.accounts.config.key(),
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            [0; 32],
        );
        Ok(())
    }
    pub fn pay_sol(
        ctx: Context<PaySol>,
        amount: u64,
        max_fee: u64,
        kind: u8,
        reference: [u8; 32],
    ) -> Result<()> {
        payments::pay_sol(ctx, amount, max_fee, kind, reference)
    }
    pub fn pay_token(
        ctx: Context<PayToken>,
        amount: u64,
        max_fee: u64,
        kind: u8,
        reference: [u8; 32],
    ) -> Result<()> {
        payments::pay_token(ctx, amount, max_fee, kind, reference)
    }
    pub fn register_tag(ctx: Context<RegisterTag>, name: String, expected_fee: u64) -> Result<()> {
        tags::register(ctx, name, expected_fee)
    }
    pub fn set_default_tag(ctx: Context<OwnTag>) -> Result<()> {
        tags::set_default(ctx)
    }
    pub fn list_tag(ctx: Context<OwnTag>, price: u64) -> Result<()> {
        tags::list(ctx, price)
    }
    pub fn cancel_tag_listing(ctx: Context<OwnTag>) -> Result<()> {
        tags::cancel(ctx)
    }
    pub fn transfer_tag(ctx: Context<TransferTag>) -> Result<()> {
        tags::transfer_tag(ctx)
    }
    pub fn buy_tag(ctx: Context<BuyTag>, expected_price: u64) -> Result<()> {
        tags::buy(ctx, expected_price)
    }
    pub fn manage_savings(ctx: Context<ManageSavings>, amount: u64, deposit: bool) -> Result<()> {
        savings_goals::manage_savings(ctx, amount, deposit)
    }
    pub fn create_goal(ctx: Context<CreateGoal>, id: u64, name: String, target: u64) -> Result<()> {
        savings_goals::create_goal(ctx, id, name, target)
    }
    pub fn join_goal(ctx: Context<JoinGoal>) -> Result<()> {
        savings_goals::join_goal(ctx)
    }
    pub fn manage_goal(ctx: Context<ManageGoal>, amount: u64, deposit: bool) -> Result<()> {
        savings_goals::manage_goal(ctx, amount, deposit)
    }
    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        recovery: Pubkey,
        cosigner: Pubkey,
        daily_limit: u64,
    ) -> Result<()> {
        vault::initialize(ctx, recovery, cosigner, daily_limit)
    }
    pub fn deposit_vault(ctx: Context<DepositVault>, amount: u64) -> Result<()> {
        vault::deposit(ctx, amount)
    }
    pub fn withdraw_vault(
        ctx: Context<WithdrawVault>,
        amount: u64,
        nonce: u64,
        deadline: i64,
    ) -> Result<()> {
        vault::withdraw(ctx, amount, nonce, deadline)
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init,payer=admin,space=8+Config::INIT_SPACE,seeds=[b"config"],bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(constraint=program.programdata_address()? == Some(program_data.key()) @ BoopError::Unauthorized)]
    pub program: Program<'info, crate::program::BoopProgram>,
    #[account(constraint=program_data.upgrade_authority_address == Some(admin.key()) @ BoopError::Unauthorized)]
    pub program_data: Account<'info, ProgramData>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct Admin<'info> {
    #[account(mut,seeds=[b"config"],bump=config.bump,has_one=admin)]
    pub config: Account<'info, Config>,
    pub admin: Signer<'info>,
}
#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    #[account(mut,seeds=[b"config"],bump=config.bump,has_one=pending_admin)]
    pub config: Account<'info, Config>,
    pub pending_admin: Signer<'info>,
}
