use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use client::util;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction, instruction::AccountMeta, message, pubkey::Pubkey, signature::Signer, transaction,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BankAccount {
    pub amount: u64,
    pub authority: Pubkey,
    pub name: String,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub enum BankInstruction {
    InitAccount { amount: u64, name: String },

    Transfer { amount: u64 },
}

fn main() {
    let client = util::new_dev_client();
    let program_id = Pubkey::from_str("Hk3sQwqGNbVzc8nbxpBWwQXLQXLEuuNcKCKDr8fs3Xfc").unwrap();

    util::check_program(&client, &program_id).unwrap();
    println!("using program {}", program_id);

    let key_pair = util::get_keypair();
    let mut buffer: Vec<u8> = Vec::new();
    BankAccount {
        amount: 100,
        name: "vic".to_string(),
        authority: key_pair.pubkey(),
    }
    .serialize(&mut buffer)
    .unwrap();
    let program_account = util::create_program_account(
        &client,
        &program_id,
        "vic_bank_test1",
        Box::new(key_pair),
        buffer.len() as u64,
    )
    .unwrap();
    let key_pair = util::get_keypair();
    init_bank_account(&client, &program_account, Box::new(key_pair), &program_id).unwrap();

    let key_pair = util::get_keypair();
    let program_account2 = util::create_program_account(
        &client,
        &program_id,
        "vic_bank_test2",
        Box::new(key_pair),
        buffer.len() as u64,
    )
    .unwrap();
    let key_pair = util::get_keypair();
    init_bank_account(&client, &program_account2, Box::new(key_pair), &program_id).unwrap();
    let key_pair = util::get_keypair();
    transfer_bank_account(
        &client,
        &program_account,
        &program_account2,
        &program_id,
        Box::new(key_pair),
        50,
    )
    .unwrap();

    println!(
        "program_account: {}",
        get_account_balance(&client, &program_account).unwrap()
    );
    println!(
        "program_account2: {}",
        get_account_balance(&client, &program_account2).unwrap()
    )
}

fn init_bank_account(
    client: &RpcClient,
    program_account: &Pubkey,
    signer: Box<dyn Signer>,
    program_id: &Pubkey,
) -> Result<(), String> {
    let accounts = vec![
        AccountMeta::new(program_account.to_owned(), false),
        AccountMeta::new(signer.pubkey(), true),
    ];

    let i = BankInstruction::InitAccount {
        amount: 1000,
        name: "vic".to_string(),
    };
    let init_account_inst =
        instruction::Instruction::new_with_borsh(program_id.to_owned(), &i, accounts);

    let msg = message::Message::new(&[init_account_inst][..], Some(&signer.pubkey()));
    let block = match client.get_recent_blockhash() {
        Ok(r) => r,
        Err(e) => {
            println!("err: {}", e);
            return Err("get block failed".to_string());
        }
    };

    let t = transaction::Transaction::new(&vec![signer], msg, block.0);
    match client.send_and_confirm_transaction(&t) {
        Err(e) => {
            println!("err: {}", e);
            return Err("send tx failed".to_string());
        }
        _ => Ok(()),
    }
}

fn transfer_bank_account(
    client: &RpcClient,
    from: &Pubkey,
    to: &Pubkey,
    program_id: &Pubkey,
    signer: Box<dyn Signer>,
    amount: u64,
) -> Result<(), String> {
    let i = BankInstruction::Transfer { amount: amount };
    let accounts = vec![
        AccountMeta::new(from.to_owned(), false),
        AccountMeta::new(to.to_owned(), false),
        AccountMeta::new(signer.pubkey(), true),
    ];

    let transfer_ints =
        instruction::Instruction::new_with_borsh(program_id.to_owned(), &i, accounts);

    let msg = message::Message::new(&[transfer_ints][..], Some(&signer.pubkey()));
    let block = match client.get_recent_blockhash() {
        Ok(r) => r,
        Err(e) => {
            println!("err: {}", e);
            return Err("get block failed".to_string());
        }
    };
    let t = transaction::Transaction::new(&vec![signer], msg, block.0);
    match client.send_and_confirm_transaction(&t) {
        Err(e) => {
            println!("err: {}", e);
            return Err("send tx failed".to_string());
        }
        _ => Ok(()),
    }
}

fn get_account_balance(client: &RpcClient, account: &Pubkey) -> Result<u64, String> {
    let account_info = client.get_account(account).unwrap();
    let data = &mut &account_info.data[..];
    let bank_account = BankAccount::try_from_slice(&data).unwrap();
    Ok(bank_account.amount)
}
