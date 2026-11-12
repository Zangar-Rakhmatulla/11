use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::{constants::*, error::ErrorCode, state::*};

#[derive(Accounts)]
pub struct Deposit<'info> {
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

pub fn handle_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(amount > 0, ErrorCode::ZeroAmount);

    // 1. Сколько share-токенов положено
    let supply = ctx.accounts.share_mint.supply;
    let assets = ctx.accounts.vault_token.amount;
    let shares: u64 = if supply == 0 || assets == 0 {
        amount
    } else {
        let v = (amount as u128)
            .checked_mul(supply as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(assets as u128)
            .ok_or(ErrorCode::MathOverflow)?;
        u64::try_from(v).map_err(|_| error!(ErrorCode::MathOverflow))?
    };
    require!(shares > 0, ErrorCode::ZeroShares);

    // 2. Перевод токенов пользователя в хранилище (подписывает пользователь)
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.user_token.to_account_info(),
                to: ctx.accounts.vault_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
    )?;

    // 3. Выпуск share-токенов (подписывает программа от имени vault PDA)
    let token_mint = ctx.accounts.vault.token_mint;
    let bump = ctx.accounts.vault.bump;
    let seeds: &[&[u8]] = &[VAULT_SEED, token_mint.as_ref(), &[bump]];
    let signer = &[seeds];
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            MintTo {
                mint: ctx.accounts.share_mint.to_account_info(),
                to: ctx.accounts.user_share.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
            },
            signer,
        ),
        shares,
    )?;

    // 4. Обновление учёта
    let vault = &mut ctx.accounts.vault;
    vault.total_deposited = vault
        .total_deposited
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    let position = &mut ctx.accounts.position;
    position.deposited = position
        .deposited
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    Ok(())
}
