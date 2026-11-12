use crate::{common::*, errors::BoopError, state::*};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct ManageSavings<'info> {
    #[account(seeds=[b"config"],bump=config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(init_if_needed,payer=owner,space=8+Savings::INIT_SPACE,seeds=[b"savings",owner.key().as_ref()],bump)]
    pub savings: Account<'info, Savings>,
    #[account(address=USDC,constraint=mint.decimals==6 @ BoopError::InvalidMint)]
    pub mint: Account<'info, Mint>,
    #[account(mut,token::mint=mint,token::authority=owner)]
    pub owner_token: Account<'info, TokenAccount>,
    #[account(mut,token::mint=mint,token::authority=savings)]
    pub savings_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
pub fn manage_savings(ctx: Context<ManageSavings>, amount: u64, deposit: bool) -> Result<()> {
    let a = ctx.accounts;
    require!(amount > 0, BoopError::InvalidInput);
    if deposit {
        require!(a.config.accounts_enabled, BoopError::Disabled);
        if a.savings.owner == Pubkey::default() {
            a.savings.owner = a.owner.key();
            a.savings.bump = ctx.bumps.savings;
        }
    }
    require!(a.savings.owner == a.owner.key(), BoopError::Unauthorized);
    let owner = a.owner.key();
    let bump = [a.savings.bump];
    let seeds: &[&[u8]] = &[b"savings", owner.as_ref(), &bump];
    if deposit {
        a.savings.available = add(a.savings.available, amount)?;
        transfer(
            a.token_program.key(),
            a.owner_token.to_account_info(),
            a.mint.to_account_info(),
            a.savings_token.to_account_info(),
            a.owner.to_account_info(),
            amount,
            &[],
        )?;
    } else {
        a.savings.available = sub(a.savings.available, amount)?;
        transfer(
            a.token_program.key(),
            a.savings_token.to_account_info(),
            a.mint.to_account_info(),
            a.owner_token.to_account_info(),
            a.savings.to_account_info(),
            amount,
            &[seeds],
        )?;
    }
    activity(
        if deposit {
            SAVINGS_DEPOSIT
        } else {
            SAVINGS_WITHDRAW
        },
        owner,
        a.savings.key(),
        owner,
        USDC,
        amount,
        0,
        [0; 32],
    );
    Ok(())
}

#[derive(Accounts)]
#[instruction(id:u64)]
pub struct CreateGoal<'info> {
    #[account(seeds=[b"config"],bump=config.bump,constraint=config.accounts_enabled @ BoopError::Disabled)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(init,payer=creator,space=8+Goal::INIT_SPACE,seeds=[b"goal",creator.key().as_ref(),&id.to_le_bytes()],bump)]
    pub goal: Account<'info, Goal>,
    pub system_program: Program<'info, System>,
}
pub fn create_goal(ctx: Context<CreateGoal>, id: u64, name: String, target: u64) -> Result<()> {
    let name = name.trim();
    require!(valid_goal_name(name) && target > 0, BoopError::InvalidInput);
    let goal = &mut ctx.accounts.goal;
    goal.creator = ctx.accounts.creator.key();
    goal.id = id;
    goal.name = name.into();
    goal.target = target;
    goal.available = 0;
    goal.members = 0;
    goal.bump = ctx.bumps.goal;
    activity(
        GOAL_CREATED,
        goal.creator,
        goal.key(),
        Pubkey::default(),
        USDC,
        target,
        0,
        [0; 32],
    );
    Ok(())
}

#[derive(Accounts)]
pub struct JoinGoal<'info> {
    #[account(seeds=[b"config"],bump=config.bump,constraint=config.accounts_enabled @ BoopError::Disabled)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub member: Signer<'info>,
    #[account(mut,seeds=[b"goal",goal.creator.as_ref(),&goal.id.to_le_bytes()],bump=goal.bump)]
    pub goal: Account<'info, Goal>,
    #[account(init_if_needed,payer=member,space=8+GoalPosition::INIT_SPACE,seeds=[b"position",goal.key().as_ref(),member.key().as_ref()],bump)]
    pub position: Account<'info, GoalPosition>,
    pub system_program: Program<'info, System>,
}
pub fn join_goal(ctx: Context<JoinGoal>) -> Result<()> {
    let a = ctx.accounts;
    let key = a.goal.key();
    join(
        &mut a.goal,
        &mut a.position,
        key,
        a.member.key(),
        ctx.bumps.position,
    )
}
fn join(
    goal: &mut Goal,
    position: &mut GoalPosition,
    key: Pubkey,
    member: Pubkey,
    bump: u8,
) -> Result<()> {
    if position.member == Pubkey::default() {
        position.goal = key;
        position.member = member;
        position.available = 0;
        position.bump = bump;
        goal.members = goal.members.checked_add(1).ok_or(BoopError::Math)?;
        activity(
            GOAL_JOINED,
            member,
            key,
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            [0; 32],
        );
    }
    require!(
        position.member == member && position.goal == key,
        BoopError::Unauthorized
    );
    Ok(())
}

#[derive(Accounts)]
pub struct ManageGoal<'info> {
    #[account(seeds=[b"config"],bump=config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub member: Signer<'info>,
    #[account(mut,seeds=[b"goal",goal.creator.as_ref(),&goal.id.to_le_bytes()],bump=goal.bump)]
    pub goal: Account<'info, Goal>,
    #[account(init_if_needed,payer=member,space=8+GoalPosition::INIT_SPACE,seeds=[b"position",goal.key().as_ref(),member.key().as_ref()],bump)]
    pub position: Account<'info, GoalPosition>,
    #[account(address=USDC,constraint=mint.decimals==6 @ BoopError::InvalidMint)]
    pub mint: Account<'info, Mint>,
    #[account(mut,token::mint=mint,token::authority=member)]
    pub member_token: Account<'info, TokenAccount>,
    #[account(mut,token::mint=mint,token::authority=goal)]
    pub goal_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
pub fn manage_goal(ctx: Context<ManageGoal>, amount: u64, deposit: bool) -> Result<()> {
    let a = ctx.accounts;
    require!(amount > 0, BoopError::InvalidInput);
    let key = a.goal.key();
    let member = a.member.key();
    if deposit {
        require!(a.config.accounts_enabled, BoopError::Disabled);
        join(
            &mut a.goal,
            &mut a.position,
            key,
            member,
            ctx.bumps.position,
        )?;
    }
    require!(
        a.position.member == member && a.position.goal == key,
        BoopError::Unauthorized
    );
    if deposit {
        a.position.available = add(a.position.available, amount)?;
        a.goal.available = add(a.goal.available, amount)?;
        transfer(
            a.token_program.key(),
            a.member_token.to_account_info(),
            a.mint.to_account_info(),
            a.goal_token.to_account_info(),
            a.member.to_account_info(),
            amount,
            &[],
        )?;
    } else {
        a.position.available = sub(a.position.available, amount)?;
        a.goal.available = sub(a.goal.available, amount)?;
        let creator = a.goal.creator;
        let id = a.goal.id.to_le_bytes();
        let bump = [a.goal.bump];
        let seeds: &[&[u8]] = &[b"goal", creator.as_ref(), &id, &bump];
        transfer(
            a.token_program.key(),
            a.goal_token.to_account_info(),
            a.mint.to_account_info(),
            a.member_token.to_account_info(),
            a.goal.to_account_info(),
            amount,
            &[seeds],
        )?;
    }
    activity(
        if deposit { GOAL_DEPOSIT } else { GOAL_WITHDRAW },
        member,
        key,
        member,
        USDC,
        amount,
        0,
        [0; 32],
    );
    Ok(())
}
pub fn valid_goal_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 32 && !name.chars().any(char::is_control)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_bounded_utf8_not_char_counts() {
        assert!(valid_goal_name("Holiday"));
        assert!(!valid_goal_name(""));
        assert!(!valid_goal_name("bad\nname"));
        assert!(!valid_goal_name(&"🌍".repeat(9)));
    }
}
