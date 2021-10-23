use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::next_account_info, account_info::AccountInfo, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub enum BankInstruction {
    InitAccount { amount: u64, name: String },

    Transfer { amount: u64 },
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BankAccount {
    pub amount: u64,
    pub authority: Pubkey,
    pub name: String,
}

impl BankAccount {
    pub fn new(amount: u64, authority: Pubkey, name: String) -> BankAccount {
        BankAccount {
            amount,
            authority,
            name,
        }
    }

    pub fn sub_with<'a>(&'a mut self, sub: u64) -> Result<&'a BankAccount, String> {
        if self.amount < sub {
            return Err("amount is insufficient".to_string());
        }
        self.amount -= sub;
        return Ok(self);
    }
    pub fn add_with<'a>(&'a mut self, add: u64) -> Result<&'a BankAccount, String> {
        self.amount += add;
        return Ok(self);
    }
}

pub fn process_bank_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    Processor {}.process_instruction(program_id, accounts, instruction_data)
}

pub struct Processor {}

impl Processor {
    pub fn process_instruction(
        &self,
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let i = match BankInstruction::try_from_slice(instruction_data) {
            Ok(i) => i,
            Err(_) => {
                return Err(ProgramError::InvalidInstructionData);
            }
        };

        match i {
            BankInstruction::InitAccount { amount, name } => {
                return self.process_init_account(program_id, accounts, amount, name);
            }
            BankInstruction::Transfer { amount } => {
                return self.process_transfer(program_id, accounts, amount);
            }
        }
    }

    fn process_init_account(
        &self,
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
        name: String,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let account = match next_account_info(accounts_iter) {
            Ok(a) => a,
            Err(_) => return Err(ProgramError::UninitializedAccount),
        };
        let authority = match next_account_info(accounts_iter) {
            Ok(a) => a,
            Err(_) => return Err(ProgramError::UninitializedAccount),
        };
        if account.owner != program_id {
            msg!("Post account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        if !authority.is_signer {
            msg!("Authority is not signer");
            return Err(ProgramError::InvalidArgument);
        };

        let bank_account = BankAccount::new(amount, authority.key.to_owned(), name);
        match bank_account.serialize(&mut &mut account.data.borrow_mut()[..]) {
            Err(_) => {
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(_) => {
                msg!("Init bank account");
            }
        };

        Ok(())
    }

    fn process_transfer(
        &self,
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let from_account = next_account_info(accounts_iter).unwrap();
        let to_account = next_account_info(accounts_iter).unwrap();
        let from_authority = next_account_info(accounts_iter).unwrap();
        if from_account.owner != program_id {
            msg!("Post account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        if to_account.owner != program_id {
            msg!("Post account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        if !from_authority.is_signer {
            msg!("Authority is not signer");
            return Err(ProgramError::InvalidArgument);
        }

        let mut from_bank_account = match BankAccount::try_from_slice(&from_account.data.borrow()) {
            Ok(a) => a,
            Err(_) => {
                return Err(ProgramError::InvalidAccountData);
            }
        };
        // if from_bank_account.authority != from_authority.key{

        // }

        let mut to_bank_account = match BankAccount::try_from_slice(&to_account.data.borrow()) {
            Ok(a) => a,
            Err(_) => {
                return Err(ProgramError::InvalidAccountData);
            }
        };

        if from_bank_account.amount < amount {
            msg!("Amount is insufficient");
            return Err(ProgramError::InvalidArgument);
        }

        let from_bank_account = from_bank_account.sub_with(amount).unwrap();
        let to_bank_account = to_bank_account.add_with(amount).unwrap();
        from_bank_account.serialize(&mut &mut from_account.data.borrow_mut()[..])?;
        to_bank_account.serialize(&mut &mut to_account.data.borrow_mut()[..])?;
        msg!("Transfer success");
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use borsh::BorshDeserialize;
    use borsh::BorshSerialize;
    use solana_program::clock::Epoch;
    use solana_program::pubkey::Pubkey;
    use std::mem;

    #[test]
    fn test_init_account() {
        let program_id = Pubkey::default();
        let key = Pubkey::default();
        let mut data = get_account_data_size("hello".to_string(), 0);
        let mut lamports: u64 = 0;
        let mut data2 = vec![0; mem::size_of::<u32>()];
        let mut lamports2: u64 = 0;
        let accounts = get_accounts(
            &program_id,
            &key,
            &mut lamports,
            &mut data[..],
            &mut lamports2,
            &mut data2[..],
        );

        let i = BankInstruction::InitAccount {
            amount: 100,
            name: "hello".to_string(),
        };
        let mut buffer: Vec<u8> = Vec::new();
        i.serialize(&mut buffer).unwrap();

        let ok = Processor {}
            .process_instruction(&program_id, &accounts, &buffer)
            .is_ok();
        assert!(ok);

        let bank_account = BankAccount::try_from_slice(&accounts[0].data.borrow()).unwrap();
        assert_eq!(bank_account.amount, 100);
        assert_eq!(bank_account.name, "hello".to_string());
    }

    #[test]
    fn test_transfer() {
        let program_id = Pubkey::default();
        let key = Pubkey::default();
        let mut data = get_account_data_size("vic1".to_string(), 100);
        let mut data2 = vec![0; mem::size_of::<u32>()];
        let mut data3 = get_account_data_size("vic2".to_string(), 50);

        let mut lamports: u64 = 0;
        let account = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut data[..],
            &program_id,
            false,
            Epoch::default(),
        );

        let mut lamports: u64 = 0;
        let verifier = AccountInfo::new(
            &key,
            true,
            true,
            &mut lamports,
            &mut data2[..],
            &key,
            false,
            Epoch::default(),
        );

        let mut lamports: u64 = 0;
        let account2 = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut data3[..],
            &key,
            false,
            Epoch::default(),
        );

        let transfer_accounts = vec![account, account2, verifier];

        let i = BankInstruction::Transfer { amount: 50 };
        let mut buffer: Vec<u8> = Vec::new();
        i.serialize(&mut buffer).unwrap();

        let ok = Processor {}
            .process_instruction(&program_id, &transfer_accounts, &buffer)
            .is_ok();
        assert!(ok);

        let from_account =
            BankAccount::try_from_slice(&transfer_accounts[0].data.borrow()).unwrap();
        let to_account = BankAccount::try_from_slice(&transfer_accounts[1].data.borrow()).unwrap();
        assert_eq!(from_account.amount, 50);
        assert_eq!(to_account.amount, 100);
    }

    fn get_account_data_size(name: String, amount: u64) -> Vec<u8> {
        let key = Pubkey::default();
        let mut data: Vec<u8> = Vec::new();
        BankAccount::new(amount, key, name)
            .serialize(&mut data)
            .unwrap();
        data
    }

    fn get_accounts<'a>(
        program_id: &'a Pubkey,
        key: &'a Pubkey,
        lamports: &'a mut u64,
        data: &'a mut [u8],
        lamports2: &'a mut u64,
        data2: &'a mut [u8],
    ) -> Vec<AccountInfo<'a>> {
        let account = AccountInfo::new(
            &key,
            false,
            true,
            lamports,
            &mut data[..],
            program_id,
            false,
            Epoch::default(),
        );

        let verifier = AccountInfo::new(
            &key,
            true,
            true,
            lamports2,
            &mut data2[..],
            &key,
            false,
            Epoch::default(),
        );
        vec![account, verifier]
    }
}
