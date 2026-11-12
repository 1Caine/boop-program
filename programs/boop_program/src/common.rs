use crate::errors::BoopError;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TransferChecked};

/// Circle USDC. Mainnet by default; the devnet build uses Circle's test-network mint.
#[cfg(not(feature = "devnet"))]
pub const USDC: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
#[cfg(feature = "devnet")]
pub const USDC: Pubkey = pubkey!("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
/// Circle EURC. The mint address is the same on mainnet and devnet.
pub const EURC: Pubkey = pubkey!("HzwqbKZw8HxMN6bF2yFZNrht3c2iXXzpKcFu7uBEDKtr");
pub const MAX_FEE_BPS: u16 = 500;

pub fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(BoopError::Math.into())
}
pub fn sub(a: u64, b: u64) -> Result<u64> {
    a.checked_sub(b)
        .ok_or(BoopError::InsufficientBalance.into())
}
pub fn supported(mint: Pubkey, decimals: u8) -> bool {
    decimals == 6 && (mint == USDC || mint == EURC)
}
pub fn valid_recipient(payer: Pubkey, recipient: Pubkey) -> bool {
    recipient != Pubkey::default() && recipient != payer && recipient != crate::ID
}
pub fn payment_fee(amount: u64, bps: u16) -> Result<u64> {
    require!(amount > 0 && bps <= MAX_FEE_BPS, BoopError::InvalidInput);
    Ok(((u128::from(amount) * u128::from(bps) + 9_999) / 10_000) as u64)
}
pub fn validate_payment(
    payer: Pubkey,
    recipient: Pubkey,
    kind: u8,
    reference: [u8; 32],
) -> Result<()> {
    require!(
        valid_recipient(payer, recipient) && kind <= 3,
        BoopError::InvalidInput
    );
    require!(kind == 0 || reference != [0; 32], BoopError::InvalidInput);
    Ok(())
}

/// Account creation is composed by the client before the program instruction, in the same
/// transaction. The program validates token account owners/mints and never accepts arbitrary CPI.
pub fn transfer<'info>(
    program: Pubkey,
    from: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    amount: u64,
    seeds: &[&[&[u8]]],
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    token::transfer_checked(
        CpiContext::new_with_signer(
            program,
            TransferChecked {
                from,
                mint,
                to,
                authority,
            },
            seeds,
        ),
        amount,
        6,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fee_bounds_and_rounding() {
        assert_eq!(payment_fee(1, 75).unwrap(), 1);
        assert_eq!(payment_fee(1_000_000, 75).unwrap(), 7_500);
        assert_eq!(payment_fee(u64::MAX, 500).unwrap(), 922_337_203_685_477_581);
        assert_eq!(payment_fee(1, 0).unwrap(), 0);
        assert!(payment_fee(0, 75).is_err());
        assert!(payment_fee(1, 501).is_err());
        assert!(add(u64::MAX, 1).is_err());
        assert!(sub(0, 1).is_err());
    }
    #[test]
    fn payment_identity_and_reference() {
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        assert!(validate_payment(a, b, 0, [0; 32]).is_ok());
        assert!(validate_payment(a, b, 1, [0; 32]).is_err());
        assert!(validate_payment(a, b, 3, [1; 32]).is_ok());
        assert!(validate_payment(a, b, 4, [1; 32]).is_err());
        assert!(validate_payment(a, a, 0, [0; 32]).is_err());
        assert!(supported(USDC, 6));
        assert!(supported(EURC, 6));
        assert!(!supported(USDC, 9));
        assert!(!supported(Pubkey::new_unique(), 6));
    }
}
