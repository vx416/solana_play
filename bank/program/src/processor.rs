use std::{alloc::GlobalAlloc, borrow::Borrow};

use crate::instruction::BankInstruction;
use crate::state::{Account, Bank};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program_error::{PrintProgramError, ProgramError},
    program_option::COption,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

pub struct Processor {}
impl Processor {
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = BankInstruction::unpack(input)?;

        match instruction {
            BankInstruction::InitializeBank { decimals } => {
                msg!("Instruction: InitializeBank");
                Self::process_initialize_bank(program_id, accounts, decimals)
            }
            BankInstruction::InitializeAccount => {
                msg!("Instruction: InitializeAccount");
                Self::process_initialize_account(program_id, accounts)
            }
            BankInstruction::Transfer { amount } => {
                msg!("Instruction: Transfer");
                Self::process_transfer(program_id, accounts, amount)
            }
            BankInstruction::Approve { amount } => {
                msg!("Instruction: Approve");
                Self::process_approve(program_id, accounts, amount)
            }
            BankInstruction::MintTo { amount } => {
                msg!("Instruction: MintTo");
                Self::process_mint_to(program_id, accounts, amount)
            }
            BankInstruction::Burn { amount } => {
                msg!("Instruction: Burn");
                Self::process_burn(program_id, accounts, amount)
            }
            BankInstruction::CloseAccount => {
                msg!("Instruction: CloseAccount");
                Self::process_close_account(program_id, accounts)
            }
        }
    }

    pub fn process_initialize_bank(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        decimals: u8,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let bank_account_info = next_account_info(account_info_iter)?;
        let bank_owner_info = next_account_info(account_info_iter)?;
        if bank_account_info.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }
        if !bank_owner_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let mut bank = Bank::unpack_unchecked(&mut bank_account_info.data.borrow_mut())?;
        if bank.is_opened {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        bank.decimals = decimals;
        bank.bank_owner = *bank_owner_info.key;
        bank.is_opened = true;

        Bank::pack(bank, &mut bank_account_info.data.borrow_mut())?;
        Ok(())
    }

    pub fn process_initialize_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let bank_account_info = next_account_info(account_info_iter)?;
        let account_info = next_account_info(account_info_iter)?;
        let account_owner_info = next_account_info(account_info_iter)?;
        if bank_account_info.owner != program_id || account_info.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }
        if !account_owner_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        let mut bank_account = Account::unpack_unchecked(&mut account_info.data.borrow_mut())?;
        if bank_account.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        bank_account.amount = 0;
        bank_account.bank = *bank_account_info.key;
        bank_account.owner = *account_owner_info.key;
        bank_account.is_initialized = true;
        bank_account.is_opened = true;
        bank_account.delegate = COption::None;
        bank_account.delegated_amount = 0;

        Account::pack(bank_account, &mut account_info.data.borrow_mut())?;
        Ok(())
    }

    pub fn process_transfer(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        transfer_amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let from_account_info = next_account_info(account_info_iter)?;
        let to_account_info = next_account_info(account_info_iter)?;
        let from_account_owner_info = next_account_info(account_info_iter)?;

        if from_account_info.owner != program_id || to_account_info.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }

        let mut from_account = Account::unpack(&from_account_info.data.borrow_mut())?;
        let mut to_account = Account::unpack(&to_account_info.data.borrow_mut())?;
        if !from_account.can_trade() || !to_account.can_trade() {
            return Err(ProgramError::InvalidAccountData);
        }
        if from_account.bank != to_account.bank {
            return Err(ProgramError::InvalidAccountData);
        }

        let use_deletegate = Self::validate_owner(&from_account, &from_account_owner_info)?;
        if use_deletegate {
            if from_account.delegated_amount < transfer_amount {
                return Err(ProgramError::InvalidAccountData);
            }

            from_account.delegated_amount = from_account
                .delegated_amount
                .checked_sub(transfer_amount)
                .ok_or(ProgramError::InvalidArgument)?
        } else {
            if from_account.amount < transfer_amount {
                return Err(ProgramError::InvalidAccountData);
            }

            from_account.amount = from_account
                .amount
                .checked_sub(transfer_amount)
                .ok_or(ProgramError::InvalidArgument)?
        }
        to_account.amount = to_account
            .amount
            .checked_add(transfer_amount)
            .ok_or(ProgramError::InvalidArgument)?;

        Account::pack(from_account, &mut from_account_info.data.borrow_mut())?;
        Account::pack(to_account, &mut to_account_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_approve(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        delegate_amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let account_info = next_account_info(account_info_iter)?;
        let account_delegate_info = next_account_info(account_info_iter)?;
        let account_owner_info = next_account_info(account_info_iter)?;
        if account_info.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }
        let mut bank_account = Account::unpack_unchecked(&mut account_info.data.borrow_mut())?;
        if !bank_account.can_trade() {
            return Err(ProgramError::InvalidAccountData);
        }
        if bank_account.amount < delegate_amount {
            return Err(ProgramError::InvalidArgument);
        }
        if bank_account.delegate.is_some() {
            if bank_account.delegate.unwrap() != *account_delegate_info.key {
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            bank_account.delegate = COption::Some(*account_delegate_info.key);
        }

        Self::validate_owner(&bank_account, &account_owner_info)?;
        bank_account.amount = bank_account
            .amount
            .checked_sub(delegate_amount)
            .ok_or(ProgramError::InvalidArgument)?;
        bank_account.delegated_amount = bank_account
            .delegated_amount
            .checked_add(delegate_amount)
            .ok_or(ProgramError::InvalidArgument)?;

        Account::pack(bank_account, &mut account_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_mint_to(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        mint_amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let bank_account_info = next_account_info(account_info_iter)?;
        let to_account_info = next_account_info(account_info_iter)?;
        let bank_owner_info = next_account_info(account_info_iter)?;

        if bank_account_info.owner != program_id || to_account_info.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }
        if !bank_owner_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        let mut bank = Bank::unpack(&mut bank_account_info.data.borrow_mut())?;
        let mut to_account = Account::unpack(&mut to_account_info.data.borrow_mut())?;
        if to_account.bank != *bank_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }
        if !to_account.can_trade() {
            return Err(ProgramError::InvalidAccountData);
        }
        if bank.bank_owner != *bank_owner_info.key {
            return Err(ProgramError::IllegalOwner);
        }
        bank.total_supply = bank
            .total_supply
            .checked_add(mint_amount)
            .ok_or(ProgramError::InvalidArgument)?;
        to_account.amount = to_account
            .amount
            .checked_add(mint_amount)
            .ok_or(ProgramError::InvalidArgument)?;

        Bank::pack(bank, &mut bank_account_info.data.borrow_mut())?;
        Account::pack(to_account, &mut to_account_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_burn(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        burn_amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let bank_info = next_account_info(account_info_iter)?;
        let burn_account_info = next_account_info(account_info_iter)?;
        let bank_owner_info = next_account_info(account_info_iter)?;
        let burn_account_owner_info = next_account_info(account_info_iter)?;

        if bank_info.owner != program_id || burn_account_info.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }
        if !bank_owner_info.is_signer || !burn_account_owner_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let mut bank = Bank::unpack(&mut bank_info.data.borrow_mut())?;
        let mut burn_bank_account = Account::unpack(&mut burn_account_info.data.borrow_mut())?;
        if burn_bank_account.bank != *bank_info.key {
            return Err(ProgramError::IllegalOwner);
        }
        if bank.bank_owner != *bank_owner_info.key
            || burn_bank_account.owner != *burn_account_owner_info.key
        {
            return Err(ProgramError::IllegalOwner);
        }
        if burn_bank_account.amount < burn_amount {
            return Err(ProgramError::InvalidArgument);
        }

        bank.total_supply = bank
            .total_supply
            .checked_sub(burn_amount)
            .ok_or(ProgramError::InvalidArgument)?;
        burn_bank_account.amount = burn_bank_account
            .amount
            .checked_sub(burn_amount)
            .ok_or(ProgramError::InvalidArgument)?;

        Bank::pack(bank, &mut bank_info.data.borrow_mut())?;
        Account::pack(burn_bank_account, &mut burn_account_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_close_account(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let closed_account_info = next_account_info(account_info_iter)?;
        let closed_account_owner_info = next_account_info(account_info_iter)?;
        if closed_account_info.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }
        if !closed_account_owner_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        let mut closed_account = Account::unpack(&mut closed_account_info.data.borrow_mut())?;
        if closed_account.owner != *closed_account_owner_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        closed_account.is_opened = false;
        Account::pack(closed_account, &mut closed_account_info.data.borrow_mut())?;
        Ok(())
    }

    pub fn validate_owner(
        from_account: &Account,
        owner_account_info: &AccountInfo,
    ) -> Result<bool, ProgramError> {
        if !owner_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if from_account.delegate.is_some() {
            if from_account.delegate.unwrap() == *owner_account_info.key {
                return Ok(true);
            }
        }
        if from_account.owner != *owner_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::instruction::{
        self, approve, burn, close_account, initialize_account, initialize_bank, mint_to, transfer,
    };
    use solana_program::{
        account_info::IntoAccountInfo, bpf_loader_upgradeable::close, clock::Epoch,
        instruction::Instruction, native_token::Sol, program_error, system_program, sysvar::rent,
    };
    use solana_sdk::account::{
        create_account_for_test, create_is_signer_account_infos, Account as SolanaAccount,
    };

    struct TestSuite {
        program_id: Pubkey,
        bank_info: (Pubkey, SolanaAccount),
        bank_owner_info: (Pubkey, SolanaAccount),
        bank_accounts_info: Vec<(Pubkey, SolanaAccount)>,
        bank_accounts_owner_info: Vec<(Pubkey, SolanaAccount)>,
        lamports: u64,
    }

    impl TestSuite {
        fn new_key_account(lamports: u64) -> (Pubkey, SolanaAccount) {
            (
                Pubkey::new_unique(),
                SolanaAccount::new(lamports, Account::get_packed_len(), &system_program::ID),
            )
        }

        fn default(lamports: u64) -> TestSuite {
            let program_id = Pubkey::new_unique();
            TestSuite {
                program_id,
                bank_info: (
                    Pubkey::new_unique(),
                    SolanaAccount::new(lamports, Bank::get_packed_len(), &program_id),
                ),
                bank_owner_info: (
                    Pubkey::new_unique(),
                    SolanaAccount::new(lamports, Bank::get_packed_len(), &system_program::ID),
                ),
                bank_accounts_info: Vec::with_capacity(2),
                bank_accounts_owner_info: Vec::with_capacity(2),
                lamports,
            }
        }

        fn add_default_bank_accounts<'a>(&'a mut self, num: u64) -> &'a mut Self {
            for _ in 0..num {
                self.bank_accounts_info.push((
                    Pubkey::new_unique(),
                    SolanaAccount::new(self.lamports, Account::get_packed_len(), &self.program_id),
                ));
                self.bank_accounts_owner_info.push((
                    Pubkey::new_unique(),
                    SolanaAccount::new(
                        self.lamports,
                        Account::get_packed_len(),
                        &system_program::ID,
                    ),
                ));
            }
            self
        }

        fn init_bank_instruction(&self, decimal: u8) -> Result<Instruction, ProgramError> {
            initialize_bank(
                &self.program_id,
                &self.bank_info.0,
                &self.bank_owner_info.0,
                decimal,
            )
        }

        fn bank_eq(&self, expect_bank: &Bank) -> Result<bool, ProgramError> {
            let bank = Bank::unpack_unchecked(&self.bank_info.1.data)?;
            Ok(expect_bank.eq(&bank))
        }

        fn account_eq(&self, i: usize, expect_account: &Account) -> Result<bool, ProgramError> {
            if i >= self.bank_accounts_info.len() {
                return Err(ProgramError::Custom(000));
            }

            let account = Account::unpack_unchecked(&self.bank_accounts_info[i].1.data)?;
            Ok(expect_account.eq(&account))
        }

        fn process_init_bank_instruction(&mut self, decimal: u8) -> ProgramResult {
            let instruction = self.init_bank_instruction(decimal).unwrap();
            do_process_instruction(
                instruction,
                vec![&mut self.bank_info.1, &mut self.bank_owner_info.1],
            )
        }

        fn process_init_bank_account_instruction(&mut self, i: usize) -> ProgramResult {
            self.check_index(i)?;
            let instruction = initialize_account(
                &self.program_id,
                &self.bank_info.0,
                &self.bank_accounts_info[i].0,
                &self.bank_accounts_owner_info[i].0,
            )?;
            do_process_instruction(
                instruction,
                vec![
                    &mut self.bank_info.1,
                    &mut self.bank_accounts_info[i].1,
                    &mut self.bank_accounts_owner_info[i].1,
                ],
            )
        }

        fn process_init_all_accounts(&mut self) -> ProgramResult {
            for i in 0..self.bank_accounts_info.len() {
                self.process_init_bank_account_instruction(i)?;
            }
            Ok(())
        }

        fn process_mint_to(&mut self, i: usize, amount: u64) -> ProgramResult {
            self.check_index(i)?;

            let instruction = mint_to(
                &self.program_id,
                &self.bank_info.0,
                &self.bank_accounts_info[i].0,
                &self.bank_owner_info.0,
                amount,
            )?;
            do_process_instruction(
                instruction,
                vec![
                    &mut self.bank_info.1,
                    &mut self.bank_accounts_info[i].1,
                    &mut self.bank_owner_info.1,
                ],
            )
        }

        fn process_transfer(&mut self, from: usize, to: usize, amount: u64) -> ProgramResult {
            self.check_index(from)?;
            self.check_index(to)?;

            let instruction = transfer(
                &self.program_id,
                &self.bank_accounts_info[from].0,
                &self.bank_accounts_info[to].0,
                &self.bank_accounts_owner_info[from].0,
                amount,
            )?;
            let mut from_acc = self.bank_accounts_info[from].1.clone();
            let mut to_acc = self.bank_accounts_info[to].1.clone();
            do_process_instruction(
                instruction,
                vec![&mut from_acc, &mut to_acc, &mut self.bank_owner_info.1],
            )?;
            self.bank_accounts_info[from].1 = from_acc;
            self.bank_accounts_info[to].1 = to_acc;
            Ok(())
        }

        fn process_transfer_delegate(
            &mut self,
            from: usize,
            delegate: (&Pubkey, &mut SolanaAccount),
            to: usize,
            amount: u64,
        ) -> ProgramResult {
            self.check_index(from)?;
            self.check_index(to)?;

            let instruction = transfer(
                &self.program_id,
                &self.bank_accounts_info[from].0,
                &self.bank_accounts_info[to].0,
                delegate.0,
                amount,
            )?;
            let mut from_acc = self.bank_accounts_info[from].1.clone();
            let mut to_acc = self.bank_accounts_info[to].1.clone();
            do_process_instruction(instruction, vec![&mut from_acc, &mut to_acc, delegate.1])?;
            self.bank_accounts_info[from].1 = from_acc;
            self.bank_accounts_info[to].1 = to_acc;
            Ok(())
        }

        fn process_approve(
            &mut self,
            i: usize,
            delegate_key: (&Pubkey, &mut SolanaAccount),
            delegate_amount: u64,
        ) -> ProgramResult {
            self.check_index(i)?;

            let instruction = approve(
                &self.program_id,
                &self.bank_accounts_info[i].0,
                delegate_key.0,
                &self.bank_accounts_owner_info[i].0,
                delegate_amount,
            )?;

            do_process_instruction(
                instruction,
                vec![
                    &mut self.bank_accounts_info[i].1,
                    delegate_key.1,
                    &mut self.bank_accounts_owner_info[i].1,
                ],
            )
        }

        fn process_burn(&mut self, i: usize, burn_amount: u64) -> ProgramResult {
            self.check_index(i)?;
            let instruction = burn(
                &self.program_id,
                &self.bank_info.0,
                &self.bank_accounts_info[i].0,
                &self.bank_owner_info.0,
                &self.bank_accounts_owner_info[i].0,
                burn_amount,
            )?;

            do_process_instruction(
                instruction,
                vec![
                    &mut self.bank_info.1,
                    &mut self.bank_accounts_info[i].1,
                    &mut self.bank_owner_info.1,
                    &mut self.bank_accounts_owner_info[i].1,
                ],
            )
        }

        fn process_close(&mut self, i: usize) -> ProgramResult {
            self.check_index(i)?;
            let instruction = close_account(
                &self.program_id,
                &self.bank_accounts_info[i].0,
                &self.bank_accounts_owner_info[i].0,
            )?;

            do_process_instruction(
                instruction,
                vec![
                    &mut self.bank_accounts_info[i].1,
                    &mut self.bank_accounts_owner_info[i].1,
                ],
            )
        }

        fn check_index(&self, i: usize) -> ProgramResult {
            if i >= self.bank_accounts_info.len() {
                return Err(ProgramError::Custom(000));
            }
            Ok(())
        }
    }

    fn do_process_instruction(
        instruction: Instruction,
        accounts: Vec<&mut SolanaAccount>,
    ) -> ProgramResult {
        let mut meta = instruction
            .accounts
            .iter()
            .zip(accounts)
            .map(|(account_meta, account)| (&account_meta.pubkey, account_meta.is_signer, account))
            .collect::<Vec<_>>();

        let account_infos = create_is_signer_account_infos(&mut meta);
        Processor::process(&instruction.program_id, &account_infos, &instruction.data)
    }

    #[test]
    fn test_initialize_bank() {
        let mut test_suite = TestSuite::default(60);
        test_suite.process_init_bank_instruction(8).unwrap();

        assert_eq!(
            Ok(true),
            test_suite.bank_eq(&Bank {
                decimals: 8,
                bank_owner: test_suite.bank_owner_info.0,
                is_opened: true,
                total_supply: 0,
            })
        );

        assert_eq!(
            Err(ProgramError::AccountAlreadyInitialized),
            test_suite.process_init_bank_instruction(8)
        );
    }

    #[test]
    fn test_initialize_bank_account() {
        let mut test_suite = TestSuite::default(60);
        test_suite.add_default_bank_accounts(1);
        test_suite.process_init_bank_instruction(8).unwrap();
        test_suite.process_init_bank_account_instruction(0).unwrap();

        assert_eq!(
            Ok(true),
            test_suite.account_eq(
                0,
                &Account {
                    amount: 0,
                    is_initialized: true,
                    is_opened: true,
                    owner: test_suite.bank_accounts_owner_info[0].0,
                    delegate: COption::None,
                    delegated_amount: 0,
                    bank: test_suite.bank_info.0,
                }
            )
        );

        assert_eq!(
            Err(ProgramError::AccountAlreadyInitialized),
            test_suite.process_init_bank_account_instruction(0)
        );
    }

    #[test]
    fn test_mint_to() {
        let mut test_suite = TestSuite::default(60);
        test_suite.add_default_bank_accounts(1);
        test_suite.process_init_bank_instruction(8).unwrap();
        test_suite.process_init_bank_account_instruction(0).unwrap();
        test_suite.process_mint_to(0, 100).unwrap();

        assert_eq!(
            Ok(true),
            test_suite.bank_eq(&Bank {
                decimals: 8,
                bank_owner: test_suite.bank_owner_info.0,
                is_opened: true,
                total_supply: 100,
            })
        );

        assert_eq!(
            Ok(true),
            test_suite.account_eq(
                0,
                &Account {
                    amount: 100,
                    is_initialized: true,
                    is_opened: true,
                    owner: test_suite.bank_accounts_owner_info[0].0,
                    delegate: COption::None,
                    delegated_amount: 0,
                    bank: test_suite.bank_info.0,
                }
            )
        );

        let failed_instruction = mint_to(
            &test_suite.program_id,
            &test_suite.bank_info.0,
            &test_suite.bank_accounts_info[0].0,
            &Pubkey::new_unique(),
            100,
        )
        .unwrap();

        assert_eq!(
            Err(ProgramError::IllegalOwner),
            do_process_instruction(
                failed_instruction,
                vec![
                    &mut test_suite.bank_info.1,
                    &mut test_suite.bank_accounts_info[0].1,
                    &mut test_suite.bank_owner_info.1
                ]
            )
        );
    }

    #[test]
    fn test_transfer() {
        let mut test_suite = TestSuite::default(60);
        test_suite.add_default_bank_accounts(2);
        test_suite.process_init_bank_instruction(8).unwrap();
        test_suite.process_init_all_accounts().unwrap();
        test_suite.process_mint_to(0, 100).unwrap();
        test_suite.process_transfer(0, 1, 60).unwrap();

        assert_eq!(
            Ok(true),
            test_suite.account_eq(
                0,
                &Account {
                    amount: 40,
                    is_initialized: true,
                    is_opened: true,
                    owner: test_suite.bank_accounts_owner_info[0].0,
                    delegate: COption::None,
                    delegated_amount: 0,
                    bank: test_suite.bank_info.0,
                },
            )
        );

        assert_eq!(
            Ok(true),
            test_suite.account_eq(
                1,
                &Account {
                    amount: 60,
                    is_initialized: true,
                    is_opened: true,
                    owner: test_suite.bank_accounts_owner_info[1].0,
                    delegate: COption::None,
                    delegated_amount: 0,
                    bank: test_suite.bank_info.0,
                },
            )
        );

        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            test_suite.process_transfer(0, 1, 60)
        );
    }

    #[test]
    fn test_approve() {
        let mut test_suite = TestSuite::default(64);
        test_suite.add_default_bank_accounts(1);
        let (key, mut account) = TestSuite::new_key_account(64);
        test_suite.process_init_bank_instruction(8).unwrap();
        test_suite.process_init_all_accounts().unwrap();
        test_suite.process_mint_to(0, 100).unwrap();
        test_suite
            .process_approve(0, (&key, &mut account), 50)
            .unwrap();

        assert_eq!(
            Ok(true),
            test_suite.account_eq(
                0,
                &Account {
                    amount: 50,
                    is_initialized: true,
                    is_opened: true,
                    owner: test_suite.bank_accounts_owner_info[0].0,
                    delegate: COption::Some(key),
                    delegated_amount: 50,
                    bank: test_suite.bank_info.0,
                }
            )
        );
    }

    #[test]
    fn test_approve_transfer() {
        let mut test_suite = TestSuite::default(64);
        test_suite.add_default_bank_accounts(2);
        let (key, mut account) = TestSuite::new_key_account(64);
        test_suite.process_init_bank_instruction(8).unwrap();
        test_suite.process_init_all_accounts().unwrap();
        test_suite.process_mint_to(0, 100).unwrap();
        test_suite
            .process_approve(0, (&key, &mut account), 50)
            .unwrap();
        test_suite
            .process_transfer_delegate(0, (&key, &mut account), 1, 30)
            .unwrap();

        assert_eq!(
            Ok(true),
            test_suite.account_eq(
                0,
                &Account {
                    amount: 50,
                    is_initialized: true,
                    is_opened: true,
                    owner: test_suite.bank_accounts_owner_info[0].0,
                    delegate: COption::Some(key),
                    delegated_amount: 20,
                    bank: test_suite.bank_info.0,
                },
            )
        );

        assert_eq!(
            Ok(true),
            test_suite.account_eq(
                1,
                &Account {
                    amount: 30,
                    is_initialized: true,
                    is_opened: true,
                    owner: test_suite.bank_accounts_owner_info[1].0,
                    delegate: COption::None,
                    delegated_amount: 0,
                    bank: test_suite.bank_info.0,
                },
            )
        );

        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            test_suite.process_transfer_delegate(0, (&key, &mut account), 1, 30)
        );
    }

    #[test]
    fn test_burn() {
        let mut test_suite = TestSuite::default(64);
        test_suite.add_default_bank_accounts(1);
        test_suite.process_init_bank_instruction(8).unwrap();
        test_suite.process_init_all_accounts().unwrap();
        test_suite.process_mint_to(0, 100).unwrap();
        test_suite.process_burn(0, 50).unwrap();

        assert_eq!(
            Ok(true),
            test_suite.bank_eq(&Bank {
                decimals: 8,
                bank_owner: test_suite.bank_owner_info.0,
                is_opened: true,
                total_supply: 50,
            })
        );

        assert_eq!(
            Ok(true),
            test_suite.account_eq(
                0,
                &Account {
                    amount: 50,
                    is_initialized: true,
                    is_opened: true,
                    owner: test_suite.bank_accounts_owner_info[0].0,
                    delegate: COption::None,
                    delegated_amount: 0,
                    bank: test_suite.bank_info.0,
                }
            )
        );
    }

    #[test]
    fn test_close() {
        let mut test_suite = TestSuite::default(64);
        test_suite.add_default_bank_accounts(1);
        test_suite.process_init_bank_instruction(8).unwrap();
        test_suite.process_init_bank_account_instruction(0).unwrap();
        test_suite.process_close(0).unwrap();

        assert_eq!(
            Ok(true),
            test_suite.account_eq(
                0,
                &Account {
                    amount: 0,
                    is_initialized: true,
                    is_opened: false,
                    owner: test_suite.bank_accounts_owner_info[0].0,
                    delegate: COption::None,
                    delegated_amount: 0,
                    bank: test_suite.bank_info.0,
                }
            )
        );

        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            test_suite.process_mint_to(0, 50)
        );
    }
}
