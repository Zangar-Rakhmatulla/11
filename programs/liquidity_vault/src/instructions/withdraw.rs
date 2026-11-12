use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::{constants::*, error::ErrorCode, state::*};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, vault.token_mint.as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(mut, address = vault.share_mint)]
    pub share_mint: Account<'info, Mint>,

    #[account(mut, address = vault.vault_token)]
    pub vault_token: Account<'info, TokenAccount>,

    #[account(mut, token::mint = vault.token_mint, token::authority = user)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut, token::mint = share_mint, token::authority = user)]
    pub user_share: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [POSITION_SEED, vault.key().as_ref(), user.key().as_ref()],
        bump = position.bump
    )]
    pub position: Account<'info, UserPosition>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_withdraw(ctx: Context<Withdraw>, shares_to_burn: u64) -> Result<()> {
    require!(shares_to_burn > 0, ErrorCode::ZeroAmount);

    let supply = ctx.accounts.share_mint.supply;
    let assets = ctx.accounts.vault_token.amount;

    // 1. Формула бойынша тиесілі токен мөлшерін есептеу
    let amount_to_return = (shares_to_burn as u128)
        .checked_mul(assets as u128)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(supply as u128)
        .ok_or(ErrorCode::MathOverflow)?;
    
    let amount_u64 = u64::try_from(amount_to_return).map_err(|_| error!(ErrorCode::MathOverflow))?;
    require!(amount_u64 > 0, ErrorCode::ZeroAmount);

    // 2. Share-токендерді жағу (Burn) - пайдаланушы қол қояды
    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Burn {
                mint: ctx.accounts.share_mint.to_account_info(),
                from: ctx.accounts.user_share.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        shares_to_burn,
    )?;

    // 3. Vault-тан токендерді пайдаланушыға аудару - программа PDA атынан қол қояды (CPI with signer)
    let token_mint = ctx.accounts.vault.token_mint;
    let bump = ctx.accounts.vault.bump;
    let seeds: &[&[u8]] = &[VAULT_SEED, token_mint.as_ref(), &[bump]];
    let signer = &[seeds];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.vault_token.to_account_info(),
                to: ctx.accounts.user_token.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
            },
            signer,
        ),
        amount_u64,
    )?;

    // 4. Есептерді жаңарту
    let vault = &mut ctx.accounts.vault;
    vault.total_deposited = vault
        .total_deposited
        .checked_sub(amount_u64)
        .ok_or(ErrorCode::MathOverflow)?;

    let position = &mut ctx.accounts.position;
    position.deposited = position
        .deposited
        .checked_sub(amount_u64)
        .ok_or(ErrorCode::MathOverflow)?;

    Ok(())
}
