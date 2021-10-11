pub mod greeting_account;

use greeting_account::process_greeting_account;
use solana_program::entrypoint;

// Declare and export the program's entrypoint
entrypoint!(process_greeting_account);
