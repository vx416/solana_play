use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    message, pubkey::Pubkey, signature::Signer, signer::keypair::Keypair, system_instruction,
    transaction,
};

pub fn create_program_account(
    client: &RpcClient,
    program_id: &Pubkey,
    seed: &str,
    signer: Box<dyn Signer>,
    space: u64,
) -> Result<Pubkey, String> {
    let pub_key = signer.pubkey();
    let program_account = match Pubkey::create_with_seed(&pub_key, seed, program_id) {
        Ok(r) => r,
        Err(e) => {
            println!("err: {}", e);
            return Err("init public key failed".to_string());
        }
    };

    match client.get_account(&program_account) {
        Ok(a) => {
            if program_id.eq(&a.owner) {
                return Ok(program_account);
            }
        }
        _ => {
            println!("create an account")
        }
    }

    let min_rent = match client.get_minimum_balance_for_rent_exemption(space as usize) {
        Ok(r) => r,
        Err(e) => {
            println!("err: {}", e);
            return Err("get rent exemption failed".to_string());
        }
    };

    let create_account_inst = system_instruction::create_account_with_seed(
        &pub_key,
        &program_account,
        &pub_key,
        seed,
        min_rent,
        space,
        program_id,
    );

    let block = match client.get_recent_blockhash() {
        Ok(r) => r,
        Err(e) => {
            println!("err: {}", e);
            return Err("get recent block failed".to_string());
        }
    };

    let ss = vec![signer];
    let msg = message::Message::new(&[create_account_inst], Some(&pub_key));
    let t = transaction::Transaction::new(&ss, msg, block.0);
    match client.send_and_confirm_transaction(&t) {
        Err(e) => {
            println!("err: {}", e);
            return Err("send and confirm transaction failed".to_string());
        }
        _ => {}
    }
    println!("create account {}", program_account);
    Ok(program_account)
}

pub fn check_program(client: &RpcClient, program_id: &Pubkey) -> Result<bool, String> {
    match client.get_account(&program_id) {
        Ok(acc) => {
            if !acc.executable {
                return Err("account is not program".to_string());
            }
        }
        Err(e) => {
            println!("err: {}", e);
            return Err("account is not found".to_string());
        }
    }
    Ok(true)
}

pub fn new_dev_client() -> RpcClient {
    let url = "https://api.devnet.solana.com".to_string();
    RpcClient::new(url)
}

pub fn get_keypair() -> Keypair {
    let private_key = [
        149, 60, 15, 69, 250, 136, 150, 132, 63, 132, 180, 80, 144, 60, 22, 44, 105, 201, 192, 41,
        82, 250, 4, 141, 202, 13, 105, 117, 101, 48, 169, 204, 12, 31, 121, 70, 7, 84, 194, 222,
        187, 140, 19, 148, 97, 215, 37, 209, 111, 77, 253, 51, 172, 67, 217, 77, 206, 125, 66, 65,
        92, 6, 40, 27,
    ];
    Keypair::from_bytes(&private_key).unwrap()
}
