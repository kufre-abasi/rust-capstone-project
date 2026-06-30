#![allow(unused)]
// use bitcoin::hex::DisplayHex;
// use bitcoincore_rpc::bitcoin::{Amount, Address};
use bitcoincore_rpc::{
    bitcoin::{Address, Amount, Network},
    Auth, Client, RpcApi,
};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::Write;
// use bitcoin::Address;

// Node access params
const RPC_URL: &str = "http://127.0.0.1:18443"; // Default regtest RPC port
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

// You can use calls not provided in RPC lib API using the generic `call` function.
// An example of using the `send` RPC call, which doesn't have exposed API.
// You can also use serde_json `Deserialize` derivation to capture the returned json result.
fn send(rpc: &Client, addr: &str, amount: f64) -> bitcoincore_rpc::Result<String> {
    let args = [
        json!([{addr : amount }]), // recipient address
        json!(null),               // conf target
        json!(null),               // estimate mode
        json!(null),               // fee rate in sats/vb
        json!(null),               // Empty option object
    ];

    #[derive(Deserialize)]
    struct SendResult {
        complete: bool,
        txid: String,
    }
    let send_result = rpc.call::<SendResult>("send", &args)?;
    assert!(send_result.complete);
    Ok(send_result.txid)
}

fn create_or_load_wallet(rpc: &Client, wallet_name: String) -> bitcoincore_rpc::Result<()> {
    let loaded_wallets = rpc.call::<Vec<String>>("listwallets", &[])?;
    if loaded_wallets.contains(&wallet_name) {
        println!("Wallet '{}' is already loaded.", wallet_name);
        return Ok(());
    }

    // Try loading first
    let load_res = rpc.call::<serde_json::Value>("loadwallet", &[json!(wallet_name)]);
    if load_res.is_ok() {
        println!("Loaded wallet '{}'", wallet_name);
        return Ok(());
    }

    // Create if load failed
    let create_res = rpc.call::<serde_json::Value>(
        "createwallet",
        &[
            json!(wallet_name),
            json!(false), // disable private keys = false
        ],
    );
    match create_res {
        Ok(_) => {
            println!("Wallet {} created.", wallet_name);
        }
        Err(e) => {
            println!("Failed to create/load wallet {}: {:?}", wallet_name, e);
            return Err(e);
        }
    }
    Ok(())
}

fn mine_until_spendable_balance(
    miner_rpc: &Client,
    mining_address: &Address,
) -> bitcoincore_rpc::Result<Amount> {
    let mut blocks_mined = 0;
    let mut current_balance = Amount::from_sat(0);

    while current_balance <= Amount::from_sat(0) {
        blocks_mined += 1;
        miner_rpc.generate_to_address(1, mining_address)?;

        current_balance = miner_rpc.get_balance(None, None)?;

        println!(
            "Mined {} blocks, current balance: {}",
            blocks_mined, current_balance
        );
    }
    println!("Spendable balance reached after {} blocks!", blocks_mined);
    println!("Mining completed. Final balance: {}", current_balance);
    // println!("Spendable balances: {}", current_balance.spendable);
    // println!("immature balances: {}", current_balance.immature);
    // println!("trusted balances: {}", current_balance.trusted);
    // println!("untrusted balances: {}", current_balance.untrusted);

    Ok(current_balance)
}

fn main() -> bitcoincore_rpc::Result<()> {
    // Connect to Bitcoin Core RPC
    let rpc = Client::new(
        RPC_URL,
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Get blockchain info
    let blockchain_info = rpc.get_blockchain_info()?;
    println!("Blockchain Info: {:?}", blockchain_info);

    // Create/Load the wallets, named 'Miner' and 'Trader'. Have logic to optionally create/load them if they do not exist or not loaded already.
    create_or_load_wallet(&rpc, "Miner".to_string())?;
    create_or_load_wallet(&rpc, "Trader".to_string())?;

    // Generate spendable balances in the Miner wallet. How many blocks needs to be mined?
    let miner_rpc = Client::new(
        &format!("{}/wallet/Miner", RPC_URL),
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    let mining_address = miner_rpc
        .get_new_address(Some("Mining Reward"), None)?
        .require_network(Network::Regtest)
        .unwrap();
    println!("Mining Address: {}", mining_address);

    let miner_balance = mine_until_spendable_balance(&miner_rpc, &mining_address)?;
    println!("Miner Balance: {}", miner_balance);

    // Load Trader wallet and generate a new address

    let trader_rpc = Client::new(
        &format!("{}/wallet/Trader", RPC_URL),
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;
    let trader_address = trader_rpc
        .get_new_address(Some("Received"), None)?
        .require_network(Network::Regtest)
        .unwrap();
    println!("Trader Address: {}", trader_address);

    // Send 20 BTC from Miner to Trader
    let txid = send(&miner_rpc, &trader_address.to_string(), 20.0)?;
    println!("Transaction ID: {}", txid);

    // Check transaction in mempool
    let mempool_entry = rpc.call::<serde_json::Value>("getmempoolentry", &[json!(txid)])?;
    println!("Mempool entry: {:?}", mempool_entry);

    // Mine 1 block to confirm the transaction
    let blockhash = rpc.generate_to_address(1, &mining_address)?;
    println!("Blockhash: {:?}", blockhash);

    // Extract all required transaction details
    let tx = miner_rpc.call::<serde_json::Value>("gettransaction", &[json!(txid), json!(null)])?;
    let rawtx =
        miner_rpc.call::<serde_json::Value>("getrawtransaction", &[json!(txid), json!(1)])?;
    println!("Transaction: {:#?}", tx);
    println!("Raw Transaction: {:#?}", rawtx);

    // Write the data to ../out.txt in the specified format given in readme.md
    let txid_str = txid.clone();

    // Miner input UTXO details
    let vin = &rawtx["vin"][0];
    let vin_txid = vin["txid"].as_str().unwrap();
    let vin_vout = vin["vout"].as_u64().unwrap();

    let input_rawtx = miner_rpc.call::<serde_json::Value>(
        "getrawtransaction",
        &[json!(vin_txid), json!(1)]
    )?;
    let input_out = &input_rawtx["vout"][vin_vout as usize];
    let miner_input_address = if let Some(addr_str) = input_out["scriptPubKey"]["address"].as_str() {
        addr_str.to_string()
    } else if let Some(addresses) = input_out["scriptPubKey"]["addresses"].as_array() {
        addresses[0].as_str().unwrap_or("").to_string()
    } else {
        String::new()
    };
    let miner_input_amount = input_out["value"].as_f64().unwrap();

    // Trader output and Miner change details
    let vouts = rawtx["vout"].as_array().unwrap();
    let mut trader_output_address = String::new();
    let mut trader_output_amount = 0.0;
    let mut miner_change_address = String::new();
    let mut miner_change_amount = 0.0;

    for vout in vouts {
        let addr = if let Some(addr_str) = vout["scriptPubKey"]["address"].as_str() {
            addr_str.to_string()
        } else if let Some(addresses) = vout["scriptPubKey"]["addresses"].as_array() {
            addresses[0].as_str().unwrap_or("").to_string()
        } else {
            String::new()
        };
        let val = vout["value"].as_f64().unwrap();
        if addr == trader_address.to_string() {
            trader_output_address = addr;
            trader_output_amount = val;
        } else {
            miner_change_address = addr;
            miner_change_amount = val;
        }
    }

    let fee_val = tx["fee"].as_f64().unwrap();
    let block_height = tx["blockheight"].as_i64().unwrap();
    let block_hash = tx["blockhash"].as_str().unwrap();

    let output_file = File::create("../out.txt")?;
    let mut writer = std::io::BufWriter::new(output_file);
    writeln!(writer, "{}", txid_str)?;
    writeln!(writer, "{}", miner_input_address)?;
    writeln!(writer, "{}", miner_input_amount)?;
    writeln!(writer, "{}", trader_output_address)?;
    writeln!(writer, "{}", trader_output_amount)?;
    writeln!(writer, "{}", miner_change_address)?;
    writeln!(writer, "{}", miner_change_amount)?;
    writeln!(writer, "{}", fee_val)?;
    writeln!(writer, "{}", block_height)?;
    writeln!(writer, "{}", block_hash)?;

    Ok(())
}
