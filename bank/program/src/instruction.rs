use solana_program::instruction::{AccountMeta, Instruction};
// use crate::error::{self};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use std::convert::TryInto;
use std::iter::Inspect;
use std::mem::size_of;

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum BankInstruction {
    InitializeBank { decimals: u8 },

    InitializeAccount,

    Transfer { amount: u64 },

    Approve { amount: u64 },

    MintTo { amount: u64 },

    Burn { amount: u64 },

    CloseAccount,
}

impl BankInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        use ProgramError::InvalidInstructionData;

        let (&tag, rest) = input.split_first().ok_or(InvalidInstructionData)?;

        Ok(match tag {
            0 => {
                let (&decimal, _rest) = rest.split_first().ok_or(InvalidInstructionData)?;
                Self::InitializeBank { decimals: decimal }
            }
            1 => Self::InitializeAccount,
            2 | 3 | 4 | 5 => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;
                match tag {
                    2 => Self::Transfer { amount },
                    3 => Self::Approve { amount },
                    4 => Self::MintTo { amount },
                    5 => Self::Burn { amount },
                    _ => unreachable!(),
                }
            }
            6 => Self::CloseAccount,
            _ => {
                return Err(InvalidInstructionData);
            }
        })
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            &Self::InitializeBank { decimals } => {
                buf.push(0);
                buf.push(decimals);
            }
            &Self::InitializeAccount => {
                buf.push(1);
            }
            &Self::Transfer { amount } => {
                buf.push(2);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            &Self::Approve { amount } => {
                buf.push(3);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            &Self::MintTo { amount } => {
                buf.push(4);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            &Self::Burn { amount } => {
                buf.push(5);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            &Self::CloseAccount => {
                buf.push(6);
            }
        };
        buf
    }

    fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        if input.len() >= 32 {
            let (key, rest) = input.split_at(32);
            let pk = Pubkey::new(key);
            return Ok((pk, rest));
        }
        Err(ProgramError::InvalidInstructionData)
    }
}

pub fn initialize_bank(
    bank_program_id: &Pubkey,
    bank: &Pubkey,
    bank_owner: &Pubkey,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let data = BankInstruction::InitializeBank { decimals }.pack();
    let accounts = vec![
        AccountMeta::new(*bank, false),
        AccountMeta::new(*bank_owner, true),
    ];
    Ok(Instruction {
        program_id: *bank_program_id,
        accounts,
        data,
    })
}

pub fn initialize_account(
    bank_program_id: &Pubkey,
    bank: &Pubkey,
    bank_account: &Pubkey,
    bank_account_owner: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = BankInstruction::InitializeAccount.pack();
    let accounts = vec![
        AccountMeta::new(*bank, false),
        AccountMeta::new(*bank_account, false),
        AccountMeta::new(*bank_account_owner, true),
    ];
    Ok(Instruction {
        program_id: *bank_program_id,
        accounts,
        data,
    })
}

pub fn transfer(
    bank_program_id: &Pubkey,
    from_account: &Pubkey,
    to_account: &Pubkey,
    from_account_owner: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = BankInstruction::Transfer { amount }.pack();
    let accounts = vec![
        AccountMeta::new(*from_account, false),
        AccountMeta::new(*to_account, false),
        AccountMeta::new(*from_account_owner, true),
    ];
    Ok(Instruction {
        program_id: *bank_program_id,
        accounts,
        data,
    })
}

pub fn approve(
    bank_program_id: &Pubkey,
    account: &Pubkey,
    delegated_account: &Pubkey,
    account_owner: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = BankInstruction::Approve { amount }.pack();
    let accounts = vec![
        AccountMeta::new(*account, false),
        AccountMeta::new(*delegated_account, false),
        AccountMeta::new(*account_owner, true),
    ];
    Ok(Instruction {
        program_id: *bank_program_id,
        accounts,
        data,
    })
}

pub fn mint_to(
    bank_program_id: &Pubkey,
    bank: &Pubkey,
    mint_account: &Pubkey,
    bank_owner: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = BankInstruction::MintTo { amount }.pack();
    let accounts = vec![
        AccountMeta::new(*bank, false),
        AccountMeta::new(*mint_account, false),
        AccountMeta::new(*bank_owner, true),
    ];
    Ok(Instruction {
        program_id: *bank_program_id,
        accounts,
        data,
    })
}

pub fn burn(
    bank_program_id: &Pubkey,
    bank: &Pubkey,
    burn_account: &Pubkey,
    bank_owner: &Pubkey,
    burn_account_owner: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = BankInstruction::Burn { amount }.pack();
    let accounts = vec![
        AccountMeta::new(*bank, false),
        AccountMeta::new(*burn_account, false),
        AccountMeta::new(*bank_owner, true),
        AccountMeta::new(*burn_account_owner, true),
    ];
    Ok(Instruction {
        program_id: *bank_program_id,
        accounts,
        data,
    })
}

pub fn close_account(
    bank_program_id: &Pubkey,
    closed_account: &Pubkey,
    account_owner: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = BankInstruction::CloseAccount.pack();
    let accounts = vec![
        AccountMeta::new(*closed_account, false),
        AccountMeta::new(*account_owner, true),
    ];
    Ok(Instruction {
        program_id: *bank_program_id,
        accounts,
        data,
    })
}
