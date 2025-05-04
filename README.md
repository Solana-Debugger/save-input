# Solana Debugger: Save input

To debug a Solana program, you need to specify its input. Unlike regular programs, the complete input to a Solana program is quite complex (transaction, signers, ledger state). Especially for more complex programs, this can be quite tedious to create.

This repo tries to make this easier by allowing you to create the input data directly from your integration tests.

## How it works

1. Add the [`save_input.rs`](https://github.com/Solana-Debugger/save-input/blob/main/save_input.rs) module to your test framework
2. At the point in your test framework where you send the transaction to Banks, you add a call to `save_input` (see the example below)
3. Run the integration test that contains the tx you want to debug
4. For each tx that is processed, this will generate a `debug_input/program_input_N` folder
5. To debug a tx, pass the respective folder to `solana-debugger init`

## Example: How to include `save_input.rs`

The SPL governance program has its own test SDK. This SDK has a function that processes all transactions. We can use it as a hook to call `save_input`.

To do this, make these changes to `governance/test-sdk/src/lib.rs`:
```
// Declare it
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

    // Call save_input
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
cd solana-program-library/governance/program

cargo-test-sbf test_create_realm --test process_create_realm -- --exact --nocapture

# cargo-test-sbf test_refund_proposal_deposit --test process_refund_proposal_deposit -- --exact --nocapture
```

The output will be stored in `solana-program-library/governance/program/debug_input`, containing one subfolder per transaction.

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

These are Solana programs where `save_input.rs` is already included. Running their integration tests will create inputs for the debugger.

* [Governance program](https://github.com/Solana-Debugger/governance-program-example)
* [Delta counter program](https://github.com/Solana-Debugger/delta-counter-program-example)
