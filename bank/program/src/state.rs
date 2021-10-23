use std::convert::TryInto;

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_option::COption,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bank {
    pub decimals: u8,
    pub bank_owner: Pubkey,
    pub is_opened: bool,
    pub total_supply: u64,
}

impl Sealed for Bank {}
impl IsInitialized for Bank {
    fn is_initialized(&self) -> bool {
        self.is_opened
    }
}

impl Pack for Bank {
    const LEN: usize = 42;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, 42];
        let (decimals, bank_owner, is_opened, total_supply) = array_refs![src, 1, 32, 1, 8];
        let decimals = decimals[0];
        let bank_owner = Pubkey::new(bank_owner);
        let is_opened = is_opened[0] == 1;
        let total_supply = u64::from_le_bytes(*total_supply);
        Ok(Bank {
            decimals,
            bank_owner,
            is_opened,
            total_supply,
        })
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 42];
        let (decimals, bank_owner, is_opened, total_supply) = mut_array_refs![dst, 1, 32, 1, 8];
        decimals[0] = self.decimals;
        bank_owner.copy_from_slice(&self.bank_owner.as_ref());
        if self.is_opened {
            is_opened[0] = 1;
        }
        total_supply.copy_from_slice(&self.total_supply.to_le_bytes());
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Account {
    pub amount: u64,
    pub is_opened: bool,
    pub is_initialized: bool,
    pub owner: Pubkey,
    pub delegate: COption<Pubkey>,
    pub delegated_amount: u64,
    pub bank: Pubkey,
}

impl Account {
    pub fn can_trade(&self) -> bool {
        return self.is_opened && self.is_initialized;
    }
}

impl Sealed for Account {}
impl IsInitialized for Account {
    fn is_initialized(&self) -> bool {
        return self.is_initialized;
    }
}

impl Pack for Account {
    const LEN: usize = 118;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, 118];
        let (amount, is_opened, is_initialized, owner, delegate, delegated_amount, bank) =
            array_refs![src, 8, 1, 1, 32, 36, 8, 32];
        let amount = u64::from_le_bytes(*amount);
        let is_opened = is_opened[0] == 1;
        let is_initialized = is_initialized[0] == 1;
        let owner = Pubkey::new(&owner[..]);
        let delegate = unpack_coption_key(delegate)?;
        let delegated_amount = u64::from_le_bytes(*delegated_amount);
        let bank = Pubkey::new(&bank[..]);
        Ok(Account {
            amount,
            is_opened,
            is_initialized,
            owner,
            delegate,
            delegated_amount,
            bank,
        })
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 118];
        let (amount, is_opened, is_initialized, owner, delegate, delegated_amount, bank) =
            mut_array_refs![dst, 8, 1, 1, 32, 36, 8, 32];
        amount.copy_from_slice(&self.amount.to_le_bytes());
        is_opened[0] = if self.is_opened { 1 } else { 0 };
        if self.is_initialized {
            is_initialized[0] = 1;
        }
        owner.copy_from_slice(&self.owner.to_bytes());
        pack_coption_key(&self.delegate, delegate);
        delegated_amount.copy_from_slice(&self.delegated_amount.to_le_bytes());
        bank.copy_from_slice(&self.bank.to_bytes());
    }
}

fn pack_coption_key(src: &COption<Pubkey>, dst: &mut [u8; 36]) {
    let (tag, body) = mut_array_refs![dst, 4, 32];
    match src {
        COption::Some(key) => {
            *tag = [1, 0, 0, 0];
            body.copy_from_slice(key.as_ref());
        }
        COption::None => {
            *tag = [0; 4];
        }
    }
}

fn unpack_coption_key(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 32];
    match *tag {
        [0, 0, 0, 0] => Ok(COption::None),
        [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

#[cfg(test)]
mod tests {
    use super::{pack_coption_key, Account, Bank};
    use solana_program::program_option::COption;
    use solana_program::program_pack::Pack;
    use solana_program::pubkey::Pubkey;
    use std::convert::TryInto;

    #[test]
    fn test_bank_pack_unpack() {
        let bank_owner = Pubkey::default();
        let bank = Bank {
            decimals: 10,
            bank_owner,
            is_opened: true,
            total_supply: 100,
        };
        let mut buf: Vec<u8> = vec![0; 42];
        bank.pack_into_slice(&mut buf[..]);

        assert_eq!(buf[0], 10);
        assert_eq!(buf[1..33], *bank_owner.as_ref());
        assert_eq!(buf[33] == 1, true);
        assert_eq!(u64::from_le_bytes(buf[34..42].try_into().unwrap()), 100);

        if let Ok(bank) = Bank::unpack_from_slice(&buf[..]) {
            assert_eq!(bank.decimals, 10);
            assert_eq!(bank.bank_owner, bank_owner);
            assert_eq!(bank.is_opened, true);
            assert_eq!(bank.total_supply, 100);
        } else {
            panic!("unpack failed")
        }
    }

    #[test]
    fn test_account_pack_unpack() {
        let account_owner = Pubkey::default();
        let account_delegate = Pubkey::default();
        let bank = Pubkey::default();
        let account = Account {
            amount: 100,
            is_opened: true,
            is_initialized: true,
            owner: account_owner,
            delegate: COption::Some(account_delegate),
            delegated_amount: 50,
            bank,
        };
        let mut buf: Vec<u8> = vec![0; 118];
        account.pack_into_slice(&mut buf[..]);
        assert_eq!(buf[..8], u64::to_le_bytes(100));
        assert_eq!(buf[8], 1);
        assert_eq!(buf[9], 1);
        assert_eq!(buf[10..42], account_owner.to_bytes());
        let mut c_option_buf = [0; 36];
        pack_coption_key(&account.delegate, &mut c_option_buf);
        assert_eq!(buf[42..78], c_option_buf);
        assert_eq!(buf[78..86], u64::to_le_bytes(50));
        assert_eq!(buf[86..118], bank.to_bytes());

        if let Ok(account) = Account::unpack_from_slice(&buf[..]) {
            assert_eq!(account.amount, 100);
            assert_eq!(account.is_opened, true);
            assert_eq!(account.is_initialized, true);
            assert_eq!(account.owner, account_owner);
            assert_eq!(account.delegate.is_some(), true);
            assert_eq!(account.delegate, COption::Some(account_delegate));
            assert_eq!(account.delegated_amount, 50);
            assert_eq!(account.bank, bank);
        } else {
            panic!("unpack failed")
        }
    }
}
