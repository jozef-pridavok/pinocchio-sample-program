use pinocchio::{account_info::AccountInfo, program_entrypoint, pubkey::Pubkey, ProgramResult};

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

pub use pinocchio;

pub const ID: Pubkey = [
    159, 98, 103, 112, 32, 200, 88, 31, 85, 25, 149, 199, 83, 5, 152, 3, 10, 121, 172, 32, 109,
    201, 137, 149, 229, 206, 232, 196, 28, 97, 114, 53,
];

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    crate::processor::process_instruction(program_id, accounts, instruction_data)
}

program_entrypoint!(process_instruction);
