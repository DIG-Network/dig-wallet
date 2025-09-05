use dig_wallet::{Wallet, WalletError};
use datalayer_driver::NetworkType;

#[tokio::main]
async fn main() -> Result<(), WalletError> {
    println!("🚀 Dig Wallet Rust Example");
    println!("==========================\n");

    // 1. Create or load a wallet
    println!("📝 Loading/Creating wallet...");
    let wallet = Wallet::load(Some("example_wallet".to_string()), true).await?;
    println!("✅ Wallet loaded successfully!\n");

    // 2. Get wallet information
    println!("🔑 Wallet Information:");
    let mnemonic = wallet.get_mnemonic()?;
    println!("   Mnemonic: {} words", mnemonic.split_whitespace().count());
    
    let address = wallet.get_owner_public_key().await?;
    println!("   Address: {}", address);
    
    let puzzle_hash = wallet.get_owner_puzzle_hash().await?;
    println!("   Puzzle Hash: {}", hex::encode(puzzle_hash.as_ref()));
    println!();

    // 3. Demonstrate address conversion
    println!("🔄 Address Conversion:");
    let converted_puzzle_hash = Wallet::address_to_puzzle_hash(&address)?;
    let converted_address = Wallet::puzzle_hash_to_address(converted_puzzle_hash, "xch")?;
    println!("   Original Address: {}", address);
    println!("   Converted Back:   {}", converted_address);
    println!("   Conversion Match: {}", address == converted_address);
    println!();

    // 4. Create a signature
    println!("✍️  Digital Signature:");
    let nonce = "example_nonce_12345";
    let signature = wallet.create_key_ownership_signature(nonce).await?;
    println!("   Nonce: {}", nonce);
    println!("   Signature: {}...", &signature[..32]);
    
    // Verify the signature
    let public_key = wallet.get_public_synthetic_key().await?;
    let public_key_hex = hex::encode(public_key.to_bytes());
    let is_valid = Wallet::verify_key_ownership_signature(nonce, &signature, &public_key_hex).await?;
    println!("   Signature Valid: {}", is_valid);
    println!();

    // 5. List all wallets
    println!("📋 Available Wallets:");
    let wallets = Wallet::list_wallets().await?;
    for wallet_name in wallets {
        println!("   - {}", wallet_name);
    }
    println!();

    // 6. Demonstrate peer connection (commented out as it requires SSL certs)
    println!("🌐 Peer Connection:");
    println!("   Note: Peer connection requires Chia SSL certificates.");
    println!("   Example methods available:");
    println!("   - Wallet::connect_mainnet_peer()");
    println!("   - Wallet::connect_testnet_peer()");
    println!("   - Wallet::connect_random_peer(network, cert_path, key_path)");
    
    /*
    // Uncomment this section if you have Chia SSL certificates set up
    println!("   Attempting to connect to mainnet...");
    match Wallet::connect_mainnet_peer().await {
        Ok(_peer) => {
            println!("   ✅ Successfully connected to mainnet peer!");
            
            // Example of using the peer for coin operations
            // let coins = wallet.select_unspent_coins(&peer, 1000000, 1000, vec![]).await?;
            // println!("   Found {} unspent coins", coins.len());
        }
        Err(e) => {
            println!("   ⚠️  Failed to connect: {}", e);
            println!("   This is expected if Chia SSL certificates are not set up.");
        }
    }
    */
    
    println!("\n🎉 Example completed successfully!");
    println!("   The Rust wallet now has full feature parity with the TypeScript version!");

    Ok(())
}
