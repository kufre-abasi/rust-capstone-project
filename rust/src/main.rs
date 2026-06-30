#![allow(unused)]
use bitcoin::hex::DisplayHex;
use bitcoincore_rpc::bitcoin::Amount;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::Write;

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
        json!(null),            // conf target
        json!(null),            // estimate mode
        json!(null),            // fee rate in sats/vb
        json!(null),            // Empty option object
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

fn create_or_load_wallet(
    rpc: &Client,
    wallet_name: String,
 ) -> bitcoincore_rpc::Result<()> {
    // rpc.create_wallet(&wallet_name, None, wallet_password.as_deref(), None, Some(false), Some(false), None)?;
    // rpc.load_wallet(&wallet_name, None, wallet_password.as_deref(), None)
    let result = rpc.call::<serde_json::Value>(
        "createwallet",
        &[
            json!(wallet_name),
            json!(false), // disable private
        ]
    );
    match result{
        Ok(_) => {
            println!("Wallet {} created.", wallet_name)
        } Err(e)=>{
            println!("{:?}", e)
             rpc.call::<serde_json::Value>(
            "loadwallet",
            &[
                json!(wallet_name)
            ],
            );
            println!("Loaded wallet '{}'", wallet_name);
        }
    };  
    Ok(())
}

fn mine_until_spendable_balance(
    miner_rpc: &Client,
    mining_address: &Address
) -> bitcoincore_rpc::Result<Amount>{    
    let mut blocks_mined = 0;
    let mut current_balance = Amount::from_sat(0);


    while current_balance <= Amount::from_sat(0) {
        blocks_mined += 1;
        miner_rpc.generate_to_address(1, mining_address)?;

        current_balance = miner_rpc.get_balances(None, None)?;

        println!("Mined {} blocks, current balance: {}", blocks_mined, current_balance);
    }
   println!(
        "Spendable balance reached after {} blocks!",
        blocks_mined
    );
    println!("Mining completed. Final balance: {}", current_balance);
    println!("Spendable balances: {}", current_balance.spendable);
    println!("immature balances: {}", current_balance.immature);
    println!("trusted balances: {}", current_balance.trusted);
    println!("untrusted balances: {}", current_balance.untrusted);

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


    let mining_address = miner_rpc.get_new_address(
        Some("Mining Reward"), 
        None,
    )?;
    println!("Mining Address: {}", mining_address);

    let miner_balance = mine_until_spendable_balance(&miner_rpc, &mining_address)?;
    println!("Miner Balance: {}", miner_balance);



    // Load Trader wallet and generate a new address

    let trader_rpc = Client::new(
        &format!("{}/wallet/Trader", RPC_URL),  
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;
    let trader_address = trader_rpc.get_new_address(
        Some("Received"), 
        None, 
    )?;
    println!("Trader Address: {}", trader_address);
    

    // Send 20 BTC from Miner to Trader
    let txid = send(&miner_rpc, &trader_address, 20.0)?;
    println!("Transaction ID: {}", txid);
    
 
    // Check transaction in mempool
    let mempool_entry = rpc.call::<serde_json::Value>(
        "getmempoolentry",
        &[json!(txid)]
    )?
    println!("Mempool entry: {}", mempool_entry);

    // Mine 1 block to confirm the transaction 

    // Extract all required transaction details

    // Write the data to ../out.txt in the specified format given in readme.md

    Ok(())
}
