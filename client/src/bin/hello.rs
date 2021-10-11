use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use client::util;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction, instruction::AccountMeta, message, pubkey::Pubkey,
    signature::Signer, transaction,
};

/// Define the type of state stored in accounts
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct GreetingAccount {
    /// number of greetings
    pub counter: u32,
}

fn main() {
    let client = util::new_dev_client();
    let program_id = Pubkey::from_str("8obM4XyWGp8isXpS2NW4zSjYJrTMT7VV4Hkvrv2TXoaV").unwrap();

    util::check_program(&client, &program_id).unwrap();
    println!("using program {}", program_id);

    let key_pair = util::get_keypair();
    let empty_account = &GreetingAccount { counter: 0 };
    let mut buffer: Vec<u8> = Vec::new();
    empty_account.serialize(&mut buffer).unwrap();
    println!("account size {}", buffer.len());
    let signer = Box::new(key_pair);

    let program_account = util::create_program_account(
        &client,
        &program_id,
        "hello-account",
        signer,
        buffer.len() as u64,
    )
    .unwrap();

    let key_pair = util::get_keypair();
    let signer = Box::new(key_pair);
    say_hello(&client, vec![program_account], &program_id, signer).unwrap();
    println!("account say {} hello", program_account);

    let greeting_account = get_greeting_account(&client, &program_account);
    println!(
        "account {} has {} greeting",
        program_account, greeting_account.counter
    )
}

fn say_hello(
    client: &RpcClient,
    accounts: Vec<Pubkey>,
    program_id: &Pubkey,
    signer: Box<dyn Signer>,
) -> Result<bool, String> {
    let instructions: Vec<instruction::Instruction> = accounts
        .iter()
        .map(|a| AccountMeta::new(a.to_owned(), false))
        .map(|am| {
            instruction::Instruction::new_with_bincode(program_id.to_owned(), &Some(()), vec![am])
        })
        .collect();
    let msg = message::Message::new(&instructions[..], Some(&signer.pubkey()));

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
        _ => {}
    }
    Ok(true)
}

fn get_greeting_account(client: &RpcClient, account: &Pubkey) -> GreetingAccount {
    let account_info = &client.get_account(account).unwrap();
    let data = &mut &account_info.data[..];
    GreetingAccount::deserialize(data).unwrap()
}
