pub mod greeting_account;
pub mod bank_account;

use greeting_account::process_greeting_account;
use bank_account::process_bank_instruction;
use solana_program::entrypoint;

// Declare and export the program's entrypoint
// entrypoint!(process_greeting_account);
entrypoint!(process_greeting_account);
