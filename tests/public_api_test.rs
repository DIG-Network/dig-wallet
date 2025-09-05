// Test that verifies all public API exports are accessible
// This simulates importing the crate from another Rust project

use dig_wallet::{
    Wallet, 
    WalletError, 
    FileCache, 
    Peer, 
    NetworkType, 
    Coin, 
    CoinSpend, 
    Bytes32, 
    PublicKey, 
    SecretKey,
    Signature,
    VERSION,
};
use std::env;
use tempfile::TempDir;

// Test helper to set up isolated test environment
fn setup_api_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let keyring_path = temp_dir.path().join("api_test_keyring.json");
    env::set_var("TEST_KEYRING_PATH", keyring_path.to_string_lossy().to_string());
    env::set_var("HOME", temp_dir.path());
    temp_dir
}

#[tokio::test]
async fn test_public_api_wallet_exports() {
    let _temp_dir = setup_api_test_env();
    
    // Test that all Wallet methods are accessible
    
    // 1. Wallet creation and management
    let mnemonic = Wallet::create_new_wallet("api_test_wallet").await.unwrap();
    assert_eq!(mnemonic.split_whitespace().count(), 24);
    
    let wallet = Wallet::load(Some("api_test_wallet".to_string()), false).await.unwrap();
    assert_eq!(wallet.get_wallet_name(), "api_test_wallet");
    assert_eq!(wallet.get_mnemonic().unwrap(), mnemonic);
    
    // 2. Key operations
    let _master_sk = wallet.get_master_secret_key().await.unwrap();
    let public_key = wallet.get_public_synthetic_key().await.unwrap();
    let _private_key = wallet.get_private_synthetic_key().await.unwrap();
    let puzzle_hash = wallet.get_owner_puzzle_hash().await.unwrap();
    let address = wallet.get_owner_public_key().await.unwrap();
    
    // 3. Address operations
    let converted_puzzle_hash = Wallet::address_to_puzzle_hash(&address).unwrap();
    assert_eq!(puzzle_hash, converted_puzzle_hash);
    
    let converted_address = Wallet::puzzle_hash_to_address(puzzle_hash, "xch").unwrap();
    assert_eq!(address, converted_address);
    
    // 4. Signature operations
    let signature = wallet.create_key_ownership_signature("api_test").await.unwrap();
    let public_key_hex = hex::encode(public_key.to_bytes());
    let is_valid = Wallet::verify_key_ownership_signature("api_test", &signature, &public_key_hex).await.unwrap();
    assert!(is_valid);
    
    // 5. Wallet management
    let wallets = Wallet::list_wallets().await.unwrap();
    assert!(wallets.contains(&"api_test_wallet".to_string()));
    
    let deleted = Wallet::delete_wallet("api_test_wallet").await.unwrap();
    assert!(deleted);
}

#[tokio::test]
async fn test_public_api_file_cache_exports() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test FileCache public API
    let cache: FileCache<String> = FileCache::new("test_cache", Some(temp_dir.path())).unwrap();
    
    // Test cache operations
    cache.set("test_key", &"test_value".to_string()).unwrap();
    let value = cache.get("test_key").unwrap().unwrap();
    assert_eq!(value, "test_value");
    
    let keys = cache.get_cached_keys().unwrap();
    assert!(keys.contains(&"test_key".to_string()));
    
    cache.delete("test_key").unwrap();
    let deleted_value = cache.get("test_key").unwrap();
    assert!(deleted_value.is_none());
}

#[test]
fn test_public_api_type_exports() {
    // Test that all re-exported types are accessible
    
    // This test verifies that the types can be used in function signatures
    fn _test_function_signatures(
        _peer: Peer,
        _network: NetworkType,
        _coin: Coin,
        _coin_spend: CoinSpend,
        _bytes32: Bytes32,
        _public_key: PublicKey,
        _secret_key: SecretKey,
        _signature: Signature,
    ) {
        // This function doesn't need to do anything, just compile
    }
    
    // Test error type
    let _error: WalletError = WalletError::MnemonicRequired;
    
    // Test version constant
    assert!(!VERSION.is_empty());
    assert!(VERSION.chars().any(|c| c.is_ascii_digit()));
}

#[tokio::test]
async fn test_public_api_error_handling() {
    let _temp_dir = setup_api_test_env();
    
    // Test that all error types are accessible and can be matched
    
    // Test WalletNotFound error
    let result = Wallet::load(Some("nonexistent_wallet".to_string()), false).await;
    match result {
        Err(WalletError::WalletNotFound(name)) => {
            assert_eq!(name, "nonexistent_wallet");
        }
        _ => panic!("Expected WalletNotFound error"),
    }
    
    // Test InvalidMnemonic error
    let result = Wallet::import_wallet("invalid_test", Some("invalid mnemonic")).await;
    match result {
        Err(WalletError::InvalidMnemonic) => {
            // Expected
        }
        _ => panic!("Expected InvalidMnemonic error"),
    }
    
    // Test MnemonicRequired error
    let result = Wallet::import_wallet("empty_test", None).await;
    match result {
        Err(WalletError::MnemonicRequired) => {
            // Expected
        }
        _ => panic!("Expected MnemonicRequired error"),
    }
}

#[test]
fn test_public_api_constants() {
    // Test that constants are accessible
    use dig_wallet::wallet::DEFAULT_FEE_COIN_COST;
    
    assert_eq!(DEFAULT_FEE_COIN_COST, 64_000_000);
    assert!(VERSION.len() > 0);
}

#[tokio::test]
async fn test_external_crate_usage_simulation() {
    let _temp_dir = setup_api_test_env();
    
    // This test simulates how an external crate would use dig-wallet
    // It only uses the public API as it would be available to external users
    
    // Step 1: Create a wallet (as external crate would)
    let wallet_result = Wallet::load(Some("external_test".to_string()), true).await;
    assert!(wallet_result.is_ok());
    let wallet = wallet_result.unwrap();
    
    // Step 2: Get address (as external crate would)
    let address_result = wallet.get_owner_public_key().await;
    assert!(address_result.is_ok());
    let address = address_result.unwrap();
    assert!(address.starts_with("xch1"));
    
    // Step 3: Create signature (as external crate would)
    let signature_result = wallet.create_key_ownership_signature("external_nonce").await;
    assert!(signature_result.is_ok());
    let signature = signature_result.unwrap();
    assert!(!signature.is_empty());
    
    // Step 4: Address conversion (as external crate would)
    let puzzle_hash_result = Wallet::address_to_puzzle_hash(&address);
    assert!(puzzle_hash_result.is_ok());
    
    // Step 5: List wallets (as external crate would)
    let wallets_result = Wallet::list_wallets().await;
    assert!(wallets_result.is_ok());
    let wallets = wallets_result.unwrap();
    assert!(wallets.contains(&"external_test".to_string()));
    
    // Step 6: Clean up (as external crate would)
    let delete_result = Wallet::delete_wallet("external_test").await;
    assert!(delete_result.is_ok());
    assert!(delete_result.unwrap());
}

#[test]
fn test_crate_metadata() {
    // Test that crate metadata is properly accessible
    
    // Version should be accessible
    assert!(!VERSION.is_empty());
    
    // Test that we can construct error types
    let errors = vec![
        WalletError::MnemonicRequired,
        WalletError::InvalidMnemonic,
        WalletError::MnemonicNotLoaded,
        WalletError::WalletNotFound("test".to_string()),
        WalletError::CryptoError("test".to_string()),
        WalletError::NetworkError("test".to_string()),
        WalletError::FileSystemError("test".to_string()),
        WalletError::SerializationError("test".to_string()),
        WalletError::DataLayerError("test".to_string()),
    ];
    
    // Verify all error types can be created and have Display implementation
    for error in errors {
        let error_string = format!("{}", error);
        assert!(!error_string.is_empty());
    }
}
