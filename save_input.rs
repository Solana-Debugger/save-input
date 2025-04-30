use std::error::Error;
use std::fs;
use std::fs::{create_dir, File};
use std::path::{Path, PathBuf};
use solana_program_test::{BanksClient, ProgramTest};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{write_keypair_file, Keypair};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use base64::{engine::general_purpose::STANDARD as base64, Engine};
use solana_program::instruction::AccountMeta;

pub async fn save_input(client: &BanksClient, transaction: &Transaction, signers: &[&Keypair]) -> Result<(), Box<dyn Error>> {

    // Create output directory

    let base_dir = Path::new("debug_input");

    if !base_dir.exists() {
        create_dir(&base_dir)?;
    }

    let mut dir_suffix = 1;
    let mut output_dir: PathBuf;
    loop {
        output_dir = base_dir.join(format!("program_input_{}", dir_suffix));
        if !output_dir.exists() {
            break;
        }
        dir_suffix += 1;
    }

    eprintln!("Create output directory: {}", output_dir.display());

    create_dir(&output_dir)?;

    // Save keypairs

    let keypairs_dir = output_dir.join(Path::new("keypairs"));
    create_dir(&keypairs_dir)?;

    save_keypairs(signers, &keypairs_dir)?;

    // Save accounts

    let accounts_dir = output_dir.join(Path::new("accounts"));
    create_dir(&accounts_dir)?;

    save_accounts(&client, &transaction, &accounts_dir).await?;

    // Save transaction

    let transaction_file = output_dir.join(Path::new("transaction.json"));

    save_transaction(&transaction, &transaction_file)?;

    Ok(())
}

async fn get_debugee_id(empty_banks_client: &BanksClient, transaction: &Transaction) -> Option<Pubkey> {
    for ix in transaction.message.instructions.iter() {
        let program_id = transaction.message.account_keys[ix.program_id_index as usize];

        let get_acc = empty_banks_client.get_account(program_id.clone()).await.unwrap();

        if get_acc.is_none() {
            return Some(program_id);
        }
    }
    None
}

pub fn save_keypairs(signers: &[&Keypair], keypairs_dir: &Path) -> Result<(), Box<dyn Error>> {
    for (i, keypair) in signers.iter().enumerate() {
        eprintln!("Keypair: {}", &keypair.pubkey());

        let keypair_file = keypairs_dir.join(format!("keypair_{}.json", i + 1));
        eprintln!("Save keypair to: {}", &keypair_file.display());
        write_keypair_file(&keypair, &keypair_file)?;
    }

    Ok(())
}

pub async fn save_accounts(client: &BanksClient, transaction: &Transaction, accounts_dir: &Path) -> Result<(), Box<dyn Error>> {

    // Create empty BanksClient

    let empty_program_test = ProgramTest::default();
    let (empty_banks_client, _, _) = empty_program_test.start().await;

    // Find program id of debugee program

    let debugee_program_id = get_debugee_id(&empty_banks_client, &transaction).await;

    // Sometimes, the user tries to save a tx that doesn't include the debugee program
    // This input not useful for the debugger, but to avoid having to force the user to filter out these tx, we allow it
    if debugee_program_id.is_none() {
        eprintln!("Failed to find debugee's program id. Still save the tx");
    }

    let mut acc_suffix = 1;

    for acc_key in &transaction.message.account_keys {
        eprintln!("Account: {}", acc_key);

        if debugee_program_id.is_some() && acc_key.eq(&debugee_program_id.unwrap()) {
            eprintln!("Debugee's program id. Skip.");
            continue;
        }

        let acc = client.get_account(acc_key.clone()).await.unwrap();
        match acc {
            None => { eprintln!("Empty account. Skip."); }
            Some(acc) => {
                let check_acc = empty_banks_client.get_account(acc_key.clone()).await.unwrap();

                if check_acc.is_some() && check_acc.unwrap().eq(&acc) {
                    eprintln!("Same account exists in empty BanksClient. Skip.");
                    continue;
                }

                // This is a simplified version of `encode_ui_account` from agave/account-decoder/src/lib.rs

                let acc_serialized = format!(
                    r#"{{
  "pubkey": "{}",
  "account": {{
    "lamports": {},
    "data": [
      "{}",
      "base64"
    ],
    "owner": "{}",
    "executable": {},
    "rentEpoch": {},
    "space": {}
  }}
}}"#,
                    acc_key.to_string(),
                    acc.lamports,
                    base64.encode(&acc.data),
                    acc.owner.to_string(),
                    acc.executable,
                    acc.rent_epoch,
                    acc.data.len()
                );

                let account_path = accounts_dir.join(format!("account_{}.json", acc_suffix));

                eprintln!("Save account to: {}", &account_path.display());

                fs::write(account_path, acc_serialized)?;

                acc_suffix += 1;
            }
        }
    }
    Ok(())
}

fn get_payer(transaction: &Transaction) -> Option<Pubkey> {
    if !transaction.message.account_keys.is_empty() &&
        transaction.message.is_signer(0) &&
        transaction.message.is_writable(0) {
        Some(transaction.message.account_keys[0])
    } else {
        None
    }
}

pub fn save_transaction(transaction: &Transaction, output_file: &Path) -> Result<(), Box<dyn Error>> {
    let payer = get_payer(&transaction).unwrap();

    eprintln!("Payer: {}", payer);

    let mut ix_str = Vec::<String>::new();

    for instruction in &transaction.message.instructions {
        let mut account_metas = Vec::new();

        for account_idx in &instruction.accounts {
            let account_idx = *account_idx as usize;

            let pubkey = transaction.message.account_keys[account_idx];

            let is_signer = transaction.message.is_signer(account_idx);
            let is_writable = transaction.message.is_maybe_writable(account_idx, None);

            account_metas.push(AccountMeta {
                pubkey,
                is_signer,
                is_writable,
            });
        }

        let ix_program_id = transaction.message.account_keys[instruction.program_id_index as usize];

        let ix_data = &instruction.data;

        let account_metas_str = account_metas.iter()
            .map(|meta| format!(r#"{{"pubkey": "{}", "is_signer": {}, "is_writable": {}}}"#,
                                meta.pubkey.to_string(),
                                meta.is_signer,
                                meta.is_writable))
            .collect::<Vec<String>>()
            .join(",\n                ");

        let ix_data_str = format!("[{}]",
                                  ix_data.iter()
                                      .map(|&byte| byte.to_string())
                                      .collect::<Vec<String>>()
                                      .join(", ")
        );

        ix_str.push(format!(r#"        {{
            "program_id": "{}",
            "accounts": [
                {}
            ],
            "data": {}
        }}"#,
                            ix_program_id.to_string(),
                            account_metas_str,
                            ix_data_str
        ));
    }

    let tx_serialized = format!(r#"{{
    "payer": "{}",
    "instructions": [
{}
    ]
}}"#, payer.to_string(), ix_str.join(", \n"));

    eprintln!("Save transaction to: {}", &output_file.display());

    fs::write(output_file, tx_serialized)?;

    Ok(())
}