use crate::{common::*, errors::BoopError, state::*};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
#[instruction(name:String)]
pub struct RegisterTag<'info> {
    #[account(seeds=[b"config"],bump=config.bump,constraint=config.tags_enabled @ BoopError::Disabled)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(init_if_needed,payer=owner,space=8+Tag::INIT_SPACE,seeds=[b"tag",name.as_bytes()],bump)]
    pub tag: Account<'info, Tag>,
    #[account(init_if_needed,payer=owner,space=8+TagOwner::INIT_SPACE,seeds=[b"tag_owner",owner.key().as_ref()],bump)]
    pub profile: Account<'info, TagOwner>,
    #[account(address=USDC,constraint=mint.decimals==6 @ BoopError::InvalidMint)]
    pub mint: Account<'info, Mint>,
    #[account(mut,token::mint=mint,token::authority=owner)]
    pub payer_token: Option<Account<'info, TokenAccount>>,
    #[account(mut,token::mint=mint,constraint=treasury_token.owner==config.treasury @ BoopError::Unauthorized)]
    pub treasury_token: Option<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
pub fn register(ctx: Context<RegisterTag>, name: String, expected_fee: u64) -> Result<()> {
    require!(valid_tag(&name), BoopError::InvalidTag);
    let a = ctx.accounts;
    let existing = a.tag.owner != Pubkey::default();
    require!(
        !existing || (a.tag.owner == a.owner.key() && a.tag.name == name),
        BoopError::Unauthorized
    );
    let first = a.profile.owner == Pubkey::default();
    require!(
        first || a.profile.owner == a.owner.key(),
        BoopError::Unauthorized
    );
    let fee = if existing || first {
        0
    } else {
        a.config.tag_fee
    };
    require!(fee == expected_fee, BoopError::PriceChanged);
    if fee > 0 {
        let from = a.payer_token.as_ref().ok_or(BoopError::InvalidInput)?;
        let to = a.treasury_token.as_ref().ok_or(BoopError::InvalidInput)?;
        transfer(
            a.token_program.key(),
            from.to_account_info(),
            a.mint.to_account_info(),
            to.to_account_info(),
            a.owner.to_account_info(),
            fee,
            &[],
        )?;
    }
    if !existing {
        a.tag.owner = a.owner.key();
        a.tag.name = name;
        a.tag.price = 0;
        a.tag.bump = ctx.bumps.tag;
    }
    if first {
        a.profile.owner = a.owner.key();
        a.profile.default_tag = a.tag.key();
        a.profile.bump = ctx.bumps.profile;
    }
    activity(
        TAG_REGISTERED,
        a.owner.key(),
        a.tag.key(),
        a.config.treasury,
        USDC,
        0,
        fee,
        [0; 32],
    );
    Ok(())
}

#[derive(Accounts)]
pub struct OwnTag<'info> {
    #[account(seeds=[b"config"],bump=config.bump)]
    pub config: Account<'info, Config>,
    pub owner: Signer<'info>,
    #[account(mut,seeds=[b"tag",tag.name.as_bytes()],bump=tag.bump,has_one=owner)]
    pub tag: Account<'info, Tag>,
    #[account(mut,seeds=[b"tag_owner",owner.key().as_ref()],bump=profile.bump,has_one=owner)]
    pub profile: Account<'info, TagOwner>,
}
pub fn set_default(ctx: Context<OwnTag>) -> Result<()> {
    require!(ctx.accounts.config.tags_enabled, BoopError::Disabled);
    ctx.accounts.profile.default_tag = ctx.accounts.tag.key();
    activity(
        TAG_DEFAULT,
        ctx.accounts.owner.key(),
        ctx.accounts.tag.key(),
        Pubkey::default(),
        Pubkey::default(),
        0,
        0,
        [0; 32],
    );
    Ok(())
}
pub fn list(ctx: Context<OwnTag>, price: u64) -> Result<()> {
    require!(ctx.accounts.config.tags_enabled, BoopError::Disabled);
    require!(price > 0, BoopError::InvalidInput);
    ctx.accounts.tag.price = price;
    activity(
        TAG_LISTED,
        ctx.accounts.owner.key(),
        ctx.accounts.tag.key(),
        Pubkey::default(),
        USDC,
        price,
        0,
        [0; 32],
    );
    Ok(())
}
pub fn cancel(ctx: Context<OwnTag>) -> Result<()> {
    ctx.accounts.tag.price = 0;
    activity(
        TAG_CANCELLED,
        ctx.accounts.owner.key(),
        ctx.accounts.tag.key(),
        Pubkey::default(),
        Pubkey::default(),
        0,
        0,
        [0; 32],
    );
    Ok(())
}

#[derive(Accounts)]
pub struct TransferTag<'info> {
    #[account(seeds=[b"config"],bump=config.bump,constraint=config.tags_enabled @ BoopError::Disabled)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: Handler rejects invalid destination; only becomes tag owner.
    pub recipient: UncheckedAccount<'info>,
    #[account(mut,seeds=[b"tag",tag.name.as_bytes()],bump=tag.bump,has_one=owner)]
    pub tag: Account<'info, Tag>,
    #[account(mut,seeds=[b"tag_owner",owner.key().as_ref()],bump=profile.bump,has_one=owner)]
    pub profile: Account<'info, TagOwner>,
    #[account(init_if_needed,payer=owner,space=8+TagOwner::INIT_SPACE,seeds=[b"tag_owner",recipient.key().as_ref()],bump)]
    pub recipient_profile: Account<'info, TagOwner>,
    pub system_program: Program<'info, System>,
}
pub fn transfer_tag(ctx: Context<TransferTag>) -> Result<()> {
    let a = ctx.accounts;
    require!(a.tag.price == 0, BoopError::Listed);
    let from = a.owner.key();
    let to = a.recipient.key();
    let tag = a.tag.key();
    move_tag(
        &mut a.tag,
        &mut a.profile,
        &mut a.recipient_profile,
        tag,
        to,
        ctx.bumps.recipient_profile,
    )?;
    activity(
        TAG_TRANSFERRED,
        from,
        tag,
        to,
        Pubkey::default(),
        0,
        0,
        [0; 32],
    );
    Ok(())
}

#[derive(Accounts)]
pub struct BuyTag<'info> {
    #[account(seeds=[b"config"],bump=config.bump,constraint=config.tags_enabled @ BoopError::Disabled)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    /// CHECK: Bound to tag owner and seller token authority.
    #[account(address=tag.owner)]
    pub seller: UncheckedAccount<'info>,
    #[account(mut,seeds=[b"tag",tag.name.as_bytes()],bump=tag.bump)]
    pub tag: Account<'info, Tag>,
    #[account(mut,seeds=[b"tag_owner",seller.key().as_ref()],bump=seller_profile.bump,constraint=seller_profile.owner==seller.key() @ BoopError::Unauthorized)]
    pub seller_profile: Account<'info, TagOwner>,
    #[account(init_if_needed,payer=buyer,space=8+TagOwner::INIT_SPACE,seeds=[b"tag_owner",buyer.key().as_ref()],bump)]
    pub buyer_profile: Account<'info, TagOwner>,
    #[account(address=USDC,constraint=mint.decimals==6 @ BoopError::InvalidMint)]
    pub mint: Account<'info, Mint>,
    #[account(mut,token::mint=mint,token::authority=buyer)]
    pub buyer_token: Account<'info, TokenAccount>,
    #[account(mut,token::mint=mint,token::authority=seller)]
    pub seller_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
pub fn buy(ctx: Context<BuyTag>, expected_price: u64) -> Result<()> {
    let a = ctx.accounts;
    require!(
        a.tag.price > 0 && a.tag.price == expected_price,
        BoopError::PriceChanged
    );
    let from = a.seller.key();
    let to = a.buyer.key();
    let tag = a.tag.key();
    move_tag(
        &mut a.tag,
        &mut a.seller_profile,
        &mut a.buyer_profile,
        tag,
        to,
        ctx.bumps.buyer_profile,
    )?;
    transfer(
        a.token_program.key(),
        a.buyer_token.to_account_info(),
        a.mint.to_account_info(),
        a.seller_token.to_account_info(),
        a.buyer.to_account_info(),
        expected_price,
        &[],
    )?;
    activity(
        TAG_PURCHASED,
        to,
        tag,
        from,
        USDC,
        expected_price,
        0,
        [0; 32],
    );
    Ok(())
}
fn move_tag(
    tag: &mut Tag,
    old: &mut TagOwner,
    new: &mut TagOwner,
    key: Pubkey,
    to: Pubkey,
    bump: u8,
) -> Result<()> {
    require!(valid_recipient(tag.owner, to), BoopError::InvalidInput);
    if old.default_tag == key {
        old.default_tag = Pubkey::default();
    }
    if new.owner == Pubkey::default() {
        new.owner = to;
        new.default_tag = key;
        new.bump = bump;
    }
    require!(new.owner == to, BoopError::Unauthorized);
    tag.owner = to;
    tag.price = 0;
    Ok(())
}
pub fn valid_tag(name: &str) -> bool {
    let bytes = name.as_bytes();
    (1..=20).contains(&bytes.len())
        && bytes.iter().any(u8::is_ascii_lowercase)
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && !matches!(
            name,
            "admin"
                | "api"
                | "boop"
                | "help"
                | "moderator"
                | "official"
                | "security"
                | "settings"
                | "support"
                | "system"
        )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_names() {
        for name in ["a", "cat123", "1cat"] {
            assert!(valid_tag(name));
        }
        for name in [
            "",
            "123",
            "Admin",
            "admin",
            "cat-one",
            "abcdefghijklmnopqrstu",
        ] {
            assert!(!valid_tag(name));
        }
    }
    #[test]
    fn ownership_moves_and_clears_sale_and_default() {
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        let key = Pubkey::new_unique();
        let mut tag = Tag {
            owner: a,
            name: "cat".into(),
            price: 10,
            bump: 1,
        };
        let mut old = TagOwner {
            owner: a,
            default_tag: key,
            bump: 1,
        };
        let mut new = TagOwner {
            owner: Pubkey::default(),
            default_tag: Pubkey::default(),
            bump: 0,
        };
        assert!(move_tag(&mut tag, &mut old, &mut new, key, a, 2).is_err());
        move_tag(&mut tag, &mut old, &mut new, key, b, 2).unwrap();
        assert_eq!(tag.owner, b);
        assert_eq!(tag.price, 0);
        assert_eq!(old.default_tag, Pubkey::default());
        assert_eq!(new.default_tag, key);
    }
}
