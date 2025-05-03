# Solana Debugger: Save Input

To debug a Solana program, you need to specify its input. In this case the input is quite complex and it can be quite tedious to create it manually.

This repo contains a Rust module to make this easier: you include it into your project, run its main function inside a test and it will create the necessary files for you

## Example: How to include `save_input.rs`

The SPL governance program uses its own test SDK with a function that processes all transactions. We can use this as a hook to call `save_input`.

To do this, make these changes to `governance/test-sdk/src/lib.rs`:
```
mod save_input;

[...]

pub async fn process_transaction(
    &mut self,
    instructions: &[Instruction],
    signers: Option<&[&Keypair]>,
) -> Result<(), ProgramError> {
    let mut transaction = Transaction::new_with_payer(instructions, Some(&self.payer.pubkey()));

    let mut all_signers = vec![&self.payer];

    if let Some(signers) = signers {
        all_signers.extend_from_slice(signers);
    }

    let recent_blockhash = self
        .context
        .banks_client
        .get_latest_blockhash()
        .await
        .unwrap();

    // Add this line
    save_input::save_input(&self.context.banks_client, &transaction, &all_signers).await.unwrap();

    transaction.sign(&all_signers, recent_blockhash);

    self.context
        .banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| map_transaction_error(e.into()))?;

    Ok(())
}
```

Now, you can run a test to generate the debug input:
```
cd governance/program

cargo-test-sbf test_create_realm --test process_create_realm -- --exact --nocapture
```

## Required dependencies

This module needs:
* `base64`
* `solana-program-test`
* `solana-program`

It suffices to add them as `dev-dependencies` since we only need them for the tests.

For example, you can add this to your `Cargo.toml`:
```
[dev-dependencies]
base64 = "0.22.1"
solana-program-test = "=2.1.9"
solana-sdk = "=2.1.9"
```

## Examples

These are Solana programs where `save_input.rs` is already included. Running their integration tests will create the debugger inputs.

* [Governance program](https://github.com/Solana-Debugger/governance-program-example)
* [Delta counter program](https://github.com/Solana-Debugger/delta-counter-example)
