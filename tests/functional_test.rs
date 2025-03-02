use pinocchio_sample::error::RecordError;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    rent::Rent,
};
use solana_sdk::{instruction::InstructionError, transaction::TransactionError};
use {
    pinocchio_sample::{instruction::RecordInstruction, state::RecordData},
    solana_program_test::*,
    solana_sdk::{
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    },
};

static CUSTOM_PROGRAM_ID: Pubkey = Pubkey::new_from_array(pinocchio_sample::ID);

fn instruction_initialize(record_account: &Pubkey, authority: &Pubkey) -> Instruction {
    Instruction {
        program_id: CUSTOM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*record_account, false),
            AccountMeta::new_readonly(*authority, false),
        ],
        data: RecordInstruction::Initialize.pack(),
    }
}

fn instruction_write(
    record_account: &Pubkey,
    signer: &Pubkey,
    offset: u64,
    data: &[u8],
) -> Instruction {
    Instruction {
        program_id: CUSTOM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*record_account, false),
            AccountMeta::new_readonly(*signer, true),
        ],
        data: RecordInstruction::Write { offset, data }.pack(),
    }
}

fn instruction_set_authority(
    record_account: &Pubkey,
    signer: &Pubkey,
    new_authority: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: CUSTOM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*record_account, false),
            AccountMeta::new_readonly(*signer, true),
            AccountMeta::new_readonly(*new_authority, false),
        ],
        data: RecordInstruction::SetAuthority.pack(),
    }
}

fn instruction_close_account(
    record_account: &Pubkey,
    signer: &Pubkey,
    receiver: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: CUSTOM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*record_account, false),
            AccountMeta::new_readonly(*signer, true),
            AccountMeta::new(*receiver, false),
        ],
        data: RecordInstruction::CloseAccount.pack(),
    }
}

fn instruction_reallocate(
    record_account: &Pubkey,
    signer: &Pubkey,
    data_length: u64,
) -> Instruction {
    Instruction {
        program_id: CUSTOM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*record_account, false),
            AccountMeta::new_readonly(*signer, true),
        ],
        data: RecordInstruction::Reallocate { data_length }.pack(),
    }
}

async fn initialize_storage_account(
    context: &mut ProgramTestContext,
    authority: &Keypair,
    account: &Keypair,
    data: &[u8],
) {
    let account_length = std::mem::size_of::<RecordData>()
        .checked_add(data.len())
        .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                1.max(Rent::default().minimum_balance(account_length)),
                account_length as u64,
                &CUSTOM_PROGRAM_ID,
            ),
            instruction_initialize(&account.pubkey(), &authority.pubkey()),
            instruction_write(&account.pubkey(), &authority.pubkey(), 0, data),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, authority, account],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[tokio::test]
async fn initialize_success() {
    let program_test = ProgramTest::new("pinocchio_sample", CUSTOM_PROGRAM_ID, None);
    let mut context: ProgramTestContext = program_test.start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[111u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let account = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let account_data =
        bytemuck::try_from_bytes::<RecordData>(&account.data[..RecordData::WRITABLE_START_INDEX])
            .unwrap();
    assert_eq!(
        account_data.authority.as_slice(),
        authority.pubkey().as_array()
    );
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
    assert_eq!(&account.data[RecordData::WRITABLE_START_INDEX..], data);
}

#[tokio::test]
async fn write_success() {
    let program_test = ProgramTest::new("pinocchio_sample", CUSTOM_PROGRAM_ID, None);
    let mut context: ProgramTestContext = program_test.start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let new_data = &[200u8; 8];
    let transaction = Transaction::new_signed_with_payer(
        &[instruction_write(
            &account.pubkey(),
            &authority.pubkey(),
            0,
            new_data,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let account_data =
        bytemuck::try_from_bytes::<RecordData>(&account.data[..RecordData::WRITABLE_START_INDEX])
            .unwrap();
    assert_eq!(
        account_data.authority.as_slice(),
        authority.pubkey().as_array()
    );
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
    assert_eq!(&account.data[RecordData::WRITABLE_START_INDEX..], new_data);
}

#[tokio::test]
async fn write_fail_wrong_authority() {
    let program_test = ProgramTest::new("pinocchio_sample", CUSTOM_PROGRAM_ID, None);
    let mut context: ProgramTestContext = program_test.start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let new_data = &[200u8; 8];
    let wrong_authority = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction_write(
            &account.pubkey(),
            &wrong_authority.pubkey(),
            0,
            new_data,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_authority],
        context.last_blockhash,
    );
    assert_eq!(
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(RecordError::IncorrectAuthority as u32)
        )
    );
}

#[tokio::test]
async fn close_account_success() {
    let program_test = ProgramTest::new("pinocchio_sample", CUSTOM_PROGRAM_ID, None);
    let mut context: ProgramTestContext = program_test.start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    let account_length = std::mem::size_of::<RecordData>()
        .checked_add(data.len())
        .unwrap();
    initialize_storage_account(&mut context, &authority, &account, data).await;
    let recipient = Pubkey::new_unique();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction_close_account(
            &account.pubkey(),
            &authority.pubkey(),
            &recipient,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(recipient)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        account.lamports,
        1.max(Rent::default().minimum_balance(account_length))
    );
}

#[tokio::test]
async fn set_authority_success() {
    let program_test = ProgramTest::new("pinocchio_sample", CUSTOM_PROGRAM_ID, None);
    let mut context: ProgramTestContext = program_test.start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;
    let new_authority = Keypair::new();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction_set_authority(
            &account.pubkey(),
            &authority.pubkey(),
            &new_authority.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account_handle = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let account_data = bytemuck::try_from_bytes::<RecordData>(
        &account_handle.data[..RecordData::WRITABLE_START_INDEX],
    )
    .unwrap();
    assert_eq!(
        account_data.authority.as_slice(),
        new_authority.pubkey().as_array()
    );

    let new_data = &[200u8; 8];
    let transaction = Transaction::new_signed_with_payer(
        &[instruction_write(
            &account.pubkey(),
            &new_authority.pubkey(),
            0,
            new_data,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &new_authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account_handle = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let account_data = bytemuck::try_from_bytes::<RecordData>(
        &account_handle.data[..RecordData::WRITABLE_START_INDEX],
    )
    .unwrap();
    assert_eq!(
        account_data.authority.as_slice(),
        new_authority.pubkey().as_array()
    );
    assert_eq!(account_data.version, RecordData::CURRENT_VERSION);
    assert_eq!(
        &account_handle.data[RecordData::WRITABLE_START_INDEX..],
        new_data,
    );
}

#[tokio::test]
async fn reallocate_success() {
    let program_test = ProgramTest::new("pinocchio_sample", CUSTOM_PROGRAM_ID, None);
    let mut context: ProgramTestContext = program_test.start_with_context().await;

    let authority = Keypair::new();
    let account = Keypair::new();
    let data = &[222u8; 8];
    initialize_storage_account(&mut context, &authority, &account, data).await;

    let new_data_length = 16u64;
    let expected_account_data_length = RecordData::WRITABLE_START_INDEX
        .checked_add(new_data_length as usize)
        .unwrap();

    let delta_account_data_length = new_data_length.saturating_sub(data.len() as u64);
    let additional_lamports_needed =
        Rent::default().minimum_balance(delta_account_data_length as usize);

    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction_reallocate(&account.pubkey(), &authority.pubkey(), new_data_length),
            system_instruction::transfer(
                &context.payer.pubkey(),
                &account.pubkey(),
                additional_lamports_needed,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account_handle = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(account_handle.data.len(), expected_account_data_length);

    let old_data_length = 8u64;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction_reallocate(
            &account.pubkey(),
            &authority.pubkey(),
            old_data_length,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(account.pubkey())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(account.data.len(), expected_account_data_length);
}
