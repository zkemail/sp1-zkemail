//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --release -- --execute
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run --release -- --prove
//! ```

use actix_web::web;
use clap::Parser;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};
use std::fs;
use std::process::Command;
use tokio::task;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct EmailInputs {
    public_key: String,
    signature: String,
    headers: String,
    body: String,
    body_hash: String,
}

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const ZKEMAIL_ELF: &[u8] = include_elf!("zkemail-program");

/// The arguments for the command.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    execute: bool,

    #[clap(long)]
    prove: bool,

    #[clap(long, default_value = "20")]
    n: u32,
}

async fn generate_email_inputs(email: String) -> Result<EmailInputs, String> {
    // Save email as email.eml in ../node-scripts/
    let write_email = web::block(move || {
        fs::write("email-input/email.eml", email).expect("failed to write email.eml");
    });
    write_email.await.expect("failed to write email.eml");

    // Delete email-inputs.json if it already exists
    let delete_script = task::spawn_blocking(|| {
        let script_path = "email-input/email-inputs.json";
        if fs::metadata(script_path).is_ok() {
            fs::remove_file(script_path).expect("failed to delete email-inputs.json");
        }
    });
    delete_script
        .await
        .expect("failed to delete email-inputs.json");

    // Change the working directory to ../node-scripts and run generate-email-inputs.js
    let run_script = task::spawn_blocking(|| {
        Command::new("sh")
            .arg("-c")
            .arg("cd email-input && node generate-email-inputs.js")
            .spawn()
            .expect("failed to run generate-email-inputs.js")
            .wait()
            .expect("failed to wait for generate-email-inputs.js");
    });
    run_script
        .await
        .expect("failed to run generate-email-inputs.js");

    // Read email inputs & convert to rust object
    let email_inputs_path = "email-input/email-inputs.json";
    let email_inputs_json = match fs::read_to_string(email_inputs_path) {
        Ok(json) => json,
        Err(err) => return Err(format!("failed to read email-inputs.json: {}", err)),
    };
    let email_inputs: EmailInputs = match serde_json::from_str(&email_inputs_json) {
        Ok(email_inputs) => email_inputs,
        Err(err) => return Err(format!("failed to parse email-inputs.json: {}", err)),
    };

    // Return email inputs object
    Ok(email_inputs)
}

#[tokio::main]
async fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    // Parse the command line arguments.
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    // Setup the prover client.
    let client = ProverClient::new();

    // Generate email inputs
    let email = fs::read_to_string("test-emails/test-email.eml")
        .expect("Failed to read test email file - ensure test-emails/test-email.eml exists");
    let email_inputs = generate_email_inputs(email)
        .await
        .expect("Failed to generate email inputs");

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    stdin.write(&email_inputs);

    if args.execute {
        // Execute the program
        let (output, report) = client.execute(ZKEMAIL_ELF, stdin).run().unwrap();
        println!("Program executed successfully.");

        // Read the output.
        println!("Output: {:?}", output);

        // Record the number of cycles executed.
        println!("Number of cycles: {}", report.total_instruction_count());
    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(ZKEMAIL_ELF);

        // Generate the proof
        let proof = client
            .prove(&pk, stdin)
            .run()
            .expect("failed to generate proof");

        println!("Successfully generated proof!");

        // Verify the proof.
        client.verify(&proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");
    }
}
