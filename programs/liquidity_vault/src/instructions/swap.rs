use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};
use anchor_spl::token::{Token, TokenAccount};

use crate::{constants::*, error::ErrorCode, state::*};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = admin @ ErrorCode::Unauthorized,
        seeds = [VAULT_SEED, vault.token_mint.as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        address = vault.vault_token
    )]
    pub vault_token_in: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = vault
    )]
    pub vault_token_out: Account<'info, TokenAccount>,

    /// CHECK: Raydium CLMM Program ID (пул мен Raydium CLMM адресі бекітілгенде нақты тексеріледі)
    pub clmm_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_swap<'info>(
    ctx: Context<'info, Swap<'info>>,
    amount_in: u64,
    min_amount_out: u64,
    swap_ix_data: Vec<u8>,
) -> Result<()> {
    require!(amount_in > 0, ErrorCode::ZeroAmount);

    let balance_out_before = ctx.accounts.vault_token_out.amount;

    let token_mint = ctx.accounts.vault.token_mint;
    let bump = ctx.accounts.vault.bump;
    let vault_seeds: &[&[u8]] = &[VAULT_SEED, token_mint.as_ref(), &[bump]];
    let signer_seeds: &[&[&[u8]]] = &[vault_seeds];

    let mut account_metas = Vec::with_capacity(ctx.remaining_accounts.len());
    let mut account_infos = Vec::with_capacity(ctx.remaining_accounts.len() + 1);

    for acc in ctx.remaining_accounts.iter() {
        let is_signer = acc.key() == ctx.accounts.vault.key();
        account_metas.push(if acc.is_writable {
            AccountMeta::new(*acc.key, is_signer)
        } else {
            AccountMeta::new_readonly(*acc.key, is_signer)
        });
        account_infos.push(acc.to_account_info());
    }
    account_infos.push(ctx.accounts.vault.to_account_info());

    let ix = Instruction {
        program_id: ctx.accounts.clmm_program.key(),
        accounts: account_metas,
        data: swap_ix_data,
    };

    invoke_signed(&ix, &account_infos, signer_seeds)?;

    ctx.accounts.vault_token_out.reload()?;
    let balance_out_after = ctx.accounts.vault_token_out.amount;
    let amount_out = balance_out_after
        .checked_sub(balance_out_before)
        .ok_or(ErrorCode::MathOverflow)?;

    require!(amount_out >= min_amount_out, ErrorCode::SlippageExceeded);

    if ctx.accounts.vault_token_in.mint == ctx.accounts.vault.token_mint {
        let vault = &mut ctx.accounts.vault;
        vault.total_deposited = vault.total_deposited.saturating_sub(amount_in);
    }

    Ok(())
}
