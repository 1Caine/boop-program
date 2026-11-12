# Boop program invariants

These are the invariants the program is built to enforce. They are engineering checks by the authors, not an independent audit.

- Only the program's actual upgrade authority can initialize configuration. App administration transfers require proposal and acceptance.
- Payment fees are bounded at 5%, rounded in integer units and capped by the payer's reviewed maximum. Recipient, reference, mint, decimals, token account authority and CPI program are checked.
- Only classic SPL USDC and EURC are accepted for token payments. Custody features and tag sales accept USDC only.
- Savings claims belong to their wallet. Goal claims belong to the contributing member; the goal creator and admin cannot spend another member's contribution.
- Vault withdrawals require two distinct authorized native transaction signers. Destination, amount, vault, nonce and deadline are signed. The spending limit resets by the on-chain UTC day. Recovery plus cosigner works without the owner.
- Pausing new activity does not block savings/goal/vault withdrawals or cancellation of tag listings.
- Transfers and state updates are atomic. Clients create required token accounts in the same transaction; on-chain validation does not rely on the client.
- Unsolicited deposits do not increase tracked claims. There is no general admin sweep, surplus withdrawal or account-close instruction.
- The server verifies finalized, successful transactions and attributes events to the actual executing program, including invocation nesting. SQL settles receipts and awards points idempotently under locks.
- Addresses and signatures preserve base58 case. Auth challenges bind origin, network, nonce and expiry; database nonce consumption is atomic.
- The mainnet and devnet builds have separate program IDs and USDC mints, selected at compile time.

The unit tests run in both the mainnet and devnet configurations on every push.

## Reporting a problem

Please report security issues privately through GitHub's "Report a vulnerability" on this repository rather than in a public issue.
