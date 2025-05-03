# Solana Debugger: Save Input

To debug a Solana program, you need to specify its input. In this case the input is quite complex and it can be quite tedious to create it manually.

This repo contains a Rust module to make this easier: you include it into your project, run its main function inside a test and it will create the necessary files for you

## Required dependencies

## Examples

These are Solana programs where `save_input.rs` is already included. Running their integration tests will create the debugger inputs.

* [[solana-debugger-governance-program-example]]
* [[solana-debugger-delta-counter-program-example]]
