use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("3JSsMyjnTCAofXRZjcZsk8fxT8R4uufWRGFkCdtYmFDb");

#[program]
pub mod lulo_dex {
    use super::*;
    /* Initialize the program, the signer is set as admin */
    pub fn initialize(ctx: Context<Initialize>, fee: u64, fee_scalar: u64) -> Result<()> {
        // Set dex state
        let state = &mut ctx.accounts.state;
        state.admin = ctx.accounts.signer.key();
        state.fee_scalar = fee_scalar;
        state.fee = fee;
        Ok(())
    }
    /* Create a vault for the dex, which enables support for that SPL */
    pub fn create_vault(ctx: Context<CreateVault>) -> Result<()> {
        Ok(())
    }
    /* Create a new listing */
    pub fn list(ctx: Context<List>, ask: u64) -> Result<()> {
        // Create listing
        let listing = &mut ctx.accounts.listing;
        listing.seller = ctx.accounts.signer.key();
        listing.mint = ctx.accounts.nft_mint.key();
        listing.ask_mint = ctx.accounts.ask_mint.key();
        listing.ask = ask;
        listing.active = true;
        listing.contract = ctx.accounts.contract.key();

        // Transfer NFT to vault
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.seller_nft.to_account_info(),
                    to: ctx.accounts.nft_vault.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info(),
                },
                &[],
            ),
            1,
        )?;
        Ok(())
    }
    /* Buy a listing */
    pub fn buy(ctx: Context<Buy>) -> Result<()> {
        // Vault bump
        let nft_vault_bump = *ctx.bumps.get("nft_vault").unwrap();

        // Transfer funds to seller
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.source.to_account_info(),
                    to: ctx.accounts.seller_escrow.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info(),
                },
                &[],
            ),
            ctx.accounts.listing.ask,
        )?;
        // Transfer NFT to buyer
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.nft_vault.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    authority: ctx.accounts.nft_vault.to_account_info(),
                },
                &[&[
                    b"vault",
                    &ctx.accounts.nft_mint.key().as_ref(),
                    &[nft_vault_bump],
                ]],
            ),
            1,
        )?;
        // Mark listing as inactive
        ctx.accounts.listing.active = false;
        Ok(())
    }
    /* Insta-sell a Contract */
    pub fn sell(ctx: Context<Sell>) -> Result<()> {
        Ok(())
    }
}

/*
- signer: Any
- state: State PDA
- system_program: System
 */
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 200,
        payer = signer,
        seeds = [b"state"],
        bump
    )]
    pub state: Box<Account<'info, State>>,
    pub system_program: Program<'info, System>,
}

/*
- signer: Admin of State
- vault: TokenAccount PDA
- mint: Mint to support, used in the Vault's PDA
- state: State
- system_program: System
 */
#[derive(Accounts)]
pub struct CreateVault<'info> {
    #[account(
        mut,
        constraint = signer.key() == state.admin)]
    pub signer: Signer<'info>,
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = mint,
        token::authority = vault,
        seeds = [b"vault", mint.key().as_ref()],
        bump
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    #[account()]
    pub mint: Box<Account<'info, Mint>>,
    #[account()]
    pub state: Box<Account<'info, State>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/*
signer: any
listing: PDA of nft mint + signer, assumes 1:1 nfts
seller_nft: token account holding the nft to sell
nft_vault: Program nft vault pda
nft_mint: Mint of the nft to sell
ask_mint: Mint of the ask
system_program: System
token_program: Token
rent: Rent
*/
#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = 250,
        seeds = [b"listing", nft_mint.key().as_ref(), signer.key().as_ref()],
        bump
    )]
    pub listing: Box<Account<'info, Listing>>,
    #[account(
        mut,
        constraint = seller_nft.mint == nft_mint.key())]
    pub seller_nft: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = nft_mint,
        token::authority = nft_vault,
        seeds = [b"vault", nft_mint.key().as_ref()],
        bump
    )]
    pub nft_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        constraint = nft_mint.supply == 1
    )]
    pub nft_mint: Box<Account<'info, Mint>>,
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = ask_mint,
        token::authority = seller_escrow,
        seeds = [b"escrow", signer.key().as_ref(), ask_mint.key().as_ref()],
        bump
    )]
    pub seller_escrow: Box<Account<'info, TokenAccount>>,
    #[account()]
    /// CHECK: Only informational. TODO: CPI
    pub contract: UncheckedAccount<'info>,
    #[account()]
    pub ask_mint: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

/*
- signer: Any
- source: Source of buyer funds
- listing: Listing to buy
- seller_escrow: Destination of buyer funds
- destination: Destination of nft sold
- nft_mint: Mint of NFT sold
- nft_vault: Protocol vault holding NFT to buy
- system_program: System
- token_program: Token
- rent: rent
*/
#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK: Constraint checks this is the seller authority
    #[account(
        mut,
        constraint = seller.key() == listing.seller,
    )]
    pub seller: AccountInfo<'info>,
    #[account(
        mut,
        constraint = source.mint == listing.ask_mint)]
    pub source: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        close = seller)]
    pub listing: Box<Account<'info, Listing>>,
    #[account(
        mut,
        seeds = [b"escrow", listing.seller.as_ref(), listing.ask_mint.as_ref()],
        bump)]
    pub seller_escrow: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = nft_mint,
        associated_token::authority = signer,
    )]
    pub destination: Box<Account<'info, TokenAccount>>,
    #[account(
        constraint = nft_mint.key() == listing.mint
    )]
    pub nft_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        seeds = [b"vault", nft_mint.key().as_ref()],
        bump
    )]
    pub nft_vault: Box<Account<'info, TokenAccount>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Sell<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        constraint = seller_nft.mint == nft_mint.key())]
    pub seller_nft: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = nft_mint,
        token::authority = nft_vault,
        seeds = [b"vault", nft_mint.key().as_ref()],
        bump
    )]
    pub nft_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        constraint = nft_mint.supply == 1
    )]
    pub nft_mint: Box<Account<'info, Mint>>,
    #[account(
        seeds = [b"vault", mint.key().as_ref()],
        bump
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    #[account()]
    /// CHECK: Only informational. TODO: CPI
    pub contract: UncheckedAccount<'info>,
    #[account()]
    pub ask_mint: Box<Account<'info, Mint>>,
    #[account()]
    pub mint: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct Listing {
    // Active
    active: bool,
    // Creator of listing
    seller: Pubkey,
    // Mint of the token for sale
    mint: Pubkey,
    // Contract associated with Mint
    contract: Pubkey,
    // SPL for ask
    ask_mint: Pubkey,
    // Price TODO: Orderbook?
    ask: u64,
}

#[account]
pub struct State {
    admin: Pubkey,
    fee: u64,
    fee_scalar: u64,
}
