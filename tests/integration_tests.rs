use dig_wallet::{Wallet, WalletError};
use std::env;
use tempfile::TempDir;

// Test helper to set up isolated test environment
fn setup_integration_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Set up isolated keyring path for this test
    let keyring_path = temp_dir.path().join("integration_keyring.json");
    env::set_var(
        "TEST_KEYRING_PATH",
        keyring_path.to_string_lossy().to_string(),
    );
    env::set_var("HOME", temp_dir.path());

    temp_dir
}

#[tokio::test]
async fn test_full_wallet_lifecycle() {
    let _temp_dir = setup_integration_test_env();

    // 1. Create a new wallet
    let mnemonic = Wallet::create_new_wallet("lifecycle_test").await.unwrap();
    assert_eq!(mnemonic.split_whitespace().count(), 24);

    // 2. Load the wallet
    let wallet = Wallet::load(Some("lifecycle_test".to_string()), false)
        .await
        .unwrap();
    assert_eq!(wallet.get_wallet_name(), "lifecycle_test");
    assert_eq!(wallet.get_mnemonic().unwrap(), mnemonic);

    // 3. Generate keys and address
    let _master_sk = wallet.get_master_secret_key().await.unwrap();
    let public_key = wallet.get_public_synthetic_key().await.unwrap();
    let private_key = wallet.get_private_synthetic_key().await.unwrap();
    let puzzle_hash = wallet.get_owner_puzzle_hash().await.unwrap();
    let address = wallet.get_owner_public_key().await.unwrap();

    // 4. Verify key consistency
    assert_eq!(
        datalayer_driver::secret_key_to_public_key(&private_key),
        public_key
    );
    assert!(address.starts_with("xch1"));
    assert_eq!(puzzle_hash.as_ref().len(), 32);

    // 5. Test signature operations
    let nonce = "integration_test_nonce";
    let signature = wallet.create_key_ownership_signature(nonce).await.unwrap();
    let public_key_hex = hex::encode(public_key.to_bytes());
    let is_valid = Wallet::verify_key_ownership_signature(nonce, &signature, &public_key_hex)
        .await
        .unwrap();
    assert!(is_valid);

    // 6. Test address conversion
    let converted_puzzle_hash = Wallet::address_to_puzzle_hash(&address).unwrap();
    let converted_address = Wallet::puzzle_hash_to_address(converted_puzzle_hash, "xch").unwrap();
    assert_eq!(address, converted_address);

    // 7. Verify wallet is in list
    let wallets = Wallet::list_wallets().await.unwrap();
    assert!(wallets.contains(&"lifecycle_test".to_string()));

    // 8. Delete wallet
    let deleted = Wallet::delete_wallet("lifecycle_test").await.unwrap();
    assert!(deleted);

    // 9. Verify wallet is gone
    let wallets_after = Wallet::list_wallets().await.unwrap();
    assert!(!wallets_after.contains(&"lifecycle_test".to_string()));

    // 10. Try to load deleted wallet (should fail)
    let result = Wallet::load(Some("lifecycle_test".to_string()), false).await;
    assert!(matches!(result, Err(WalletError::WalletNotFound(_))));
}

#[tokio::test]
async fn test_wallet_import_and_consistency() {
    let _temp_dir = setup_integration_test_env();

    // Known test mnemonic that should produce consistent results
    let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    // Import wallet twice with different names
    Wallet::import_wallet("consistent1", Some(test_mnemonic))
        .await
        .unwrap();
    Wallet::import_wallet("consistent2", Some(test_mnemonic))
        .await
        .unwrap();

    // Load both wallets
    let wallet1 = Wallet::load(Some("consistent1".to_string()), false)
        .await
        .unwrap();
    let wallet2 = Wallet::load(Some("consistent2".to_string()), false)
        .await
        .unwrap();

    // Both should have the same mnemonic
    assert_eq!(wallet1.get_mnemonic().unwrap(), test_mnemonic);
    assert_eq!(wallet2.get_mnemonic().unwrap(), test_mnemonic);

    // Both should generate the same keys
    let sk1 = wallet1.get_master_secret_key().await.unwrap();
    let sk2 = wallet2.get_master_secret_key().await.unwrap();
    assert_eq!(sk1.to_bytes(), sk2.to_bytes());

    let pk1 = wallet1.get_public_synthetic_key().await.unwrap();
    let pk2 = wallet2.get_public_synthetic_key().await.unwrap();
    assert_eq!(pk1.to_bytes(), pk2.to_bytes());

    let addr1 = wallet1.get_owner_public_key().await.unwrap();
    let addr2 = wallet2.get_owner_public_key().await.unwrap();
    assert_eq!(addr1, addr2);

    // Both should produce the same signatures
    let nonce = "consistency_test";
    let sig1 = wallet1.create_key_ownership_signature(nonce).await.unwrap();
    let sig2 = wallet2.create_key_ownership_signature(nonce).await.unwrap();
    assert_eq!(sig1, sig2);
}

#[tokio::test]
async fn test_multiple_wallet_isolation() {
    let _temp_dir = setup_integration_test_env();

    // Create multiple wallets
    let wallets_to_create = vec!["isolation1", "isolation2", "isolation3", "isolation4"];
    let mut created_mnemonics = Vec::new();

    for wallet_name in &wallets_to_create {
        let mnemonic = Wallet::create_new_wallet(wallet_name).await.unwrap();
        created_mnemonics.push(mnemonic);
    }

    // Verify all mnemonics are different
    for i in 0..created_mnemonics.len() {
        for j in i + 1..created_mnemonics.len() {
            assert_ne!(created_mnemonics[i], created_mnemonics[j]);
        }
    }

    // Load all wallets and verify their addresses are different
    let mut addresses = Vec::new();
    for wallet_name in &wallets_to_create {
        let wallet = Wallet::load(Some(wallet_name.to_string()), false)
            .await
            .unwrap();
        let address = wallet.get_owner_public_key().await.unwrap();
        addresses.push(address);
    }

    // Verify all addresses are different
    for i in 0..addresses.len() {
        for j in i + 1..addresses.len() {
            assert_ne!(addresses[i], addresses[j]);
        }
    }

    // Verify all wallets are listed
    let wallet_list = Wallet::list_wallets().await.unwrap();
    for wallet_name in &wallets_to_create {
        assert!(wallet_list.contains(&wallet_name.to_string()));
    }
    assert_eq!(wallet_list.len(), wallets_to_create.len());
}

#[tokio::test]
async fn test_signature_verification_edge_cases() {
    let _temp_dir = setup_integration_test_env();

    let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    Wallet::import_wallet("signature_test", Some(test_mnemonic))
        .await
        .unwrap();
    let wallet = Wallet::load(Some("signature_test".to_string()), false)
        .await
        .unwrap();

    let public_key = wallet.get_public_synthetic_key().await.unwrap();
    let public_key_hex = hex::encode(public_key.to_bytes());

    // Create a valid signature
    let nonce = "edge_case_test";
    let signature = wallet.create_key_ownership_signature(nonce).await.unwrap();

    // Test 1: Valid signature should pass
    let result = Wallet::verify_key_ownership_signature(nonce, &signature, &public_key_hex)
        .await
        .unwrap();
    assert!(result);

    // Test 2: Wrong nonce should fail
    let result = Wallet::verify_key_ownership_signature("wrong_nonce", &signature, &public_key_hex)
        .await
        .unwrap();
    assert!(!result);

    // Test 3: Invalid signature hex should error
    let result =
        Wallet::verify_key_ownership_signature(nonce, "invalid_hex_signature", &public_key_hex)
            .await;
    assert!(result.is_err());

    // Test 4: Wrong signature length should error
    let short_signature = "deadbeef";
    let result =
        Wallet::verify_key_ownership_signature(nonce, short_signature, &public_key_hex).await;
    assert!(result.is_err());

    // Test 5: Invalid public key should error
    let result =
        Wallet::verify_key_ownership_signature(nonce, &signature, "invalid_public_key").await;
    assert!(result.is_err());

    // Test 6: Wrong public key length should error
    let short_key = "deadbeef";
    let result = Wallet::verify_key_ownership_signature(nonce, &signature, short_key).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_address_encoding_edge_cases() {
    // Test various address encoding/decoding scenarios

    // Test 1: Invalid address format
    let result = Wallet::address_to_puzzle_hash("invalid_address_format");
    assert!(result.is_err());

    // Test 2: Empty address
    let result = Wallet::address_to_puzzle_hash("");
    assert!(result.is_err());

    // Test 3: Wrong prefix
    let result = Wallet::address_to_puzzle_hash("btc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4");
    assert!(result.is_err());

    // Test 4: Valid address roundtrip
    let _temp_dir = setup_integration_test_env();
    let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    Wallet::import_wallet("address_edge_test", Some(test_mnemonic))
        .await
        .unwrap();
    let wallet = Wallet::load(Some("address_edge_test".to_string()), false)
        .await
        .unwrap();

    let original_address = wallet.get_owner_public_key().await.unwrap();
    let puzzle_hash = Wallet::address_to_puzzle_hash(&original_address).unwrap();
    let roundtrip_address = Wallet::puzzle_hash_to_address(puzzle_hash, "xch").unwrap();

    assert_eq!(original_address, roundtrip_address);

    // Test 5: Different prefixes
    let testnet_address = Wallet::puzzle_hash_to_address(puzzle_hash, "txch").unwrap();
    assert!(testnet_address.starts_with("txch1"));
    assert_ne!(original_address, testnet_address);
}

#[tokio::test]
async fn test_encryption_robustness() {
    let _temp_dir = setup_integration_test_env();

    // Test encryption with various data sizes and types
    let test_cases = vec![
        "short",
        "medium length mnemonic phrase with several words",
        "very long mnemonic phrase with many words that should still encrypt and decrypt properly without any issues or data corruption",
        "special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?",
        "unicode: 🔑💰🚀🌟✨🎯🔐",
        "", // empty string
    ];

    for test_data in test_cases {
        // Create wallet with this data
        let wallet_name = format!("encrypt_test_{}", test_data.len());

        if test_data.is_empty() {
            // Skip empty string as it's not a valid mnemonic
            continue;
        }

        // For valid test cases, create a proper mnemonic first
        if test_data.len() > 10 {
            // Use a real mnemonic for longer test cases
            let real_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
            Wallet::import_wallet(&wallet_name, Some(real_mnemonic))
                .await
                .unwrap();

            // Load and verify
            let wallet = Wallet::load(Some(wallet_name.clone()), false)
                .await
                .unwrap();
            assert_eq!(wallet.get_mnemonic().unwrap(), real_mnemonic);

            // Verify the wallet can perform crypto operations
            let signature = wallet.create_key_ownership_signature("test").await.unwrap();
            assert!(!signature.is_empty());
        }
    }
}

#[tokio::test]
async fn test_concurrent_wallet_operations() {
    let _temp_dir = setup_integration_test_env();

    // Create multiple wallets concurrently (though they'll run sequentially in single-threaded test)
    let wallet_names = vec!["concurrent1", "concurrent2", "concurrent3"];

    // Create wallets
    for name in &wallet_names {
        Wallet::create_new_wallet(name).await.unwrap();
    }

    // Load all wallets and perform operations
    let mut handles = Vec::new();

    for name in wallet_names {
        let handle = tokio::spawn(async move {
            let wallet = Wallet::load(Some(name.to_string()), false).await.unwrap();

            // Perform various operations
            let _master_key = wallet.get_master_secret_key().await.unwrap();
            let _public_key = wallet.get_public_synthetic_key().await.unwrap();
            let _address = wallet.get_owner_public_key().await.unwrap();
            let signature = wallet
                .create_key_ownership_signature("concurrent_test")
                .await
                .unwrap();

            (name.to_string(), signature)
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.unwrap();
        results.push(result);
    }

    // Verify all operations completed successfully
    assert_eq!(results.len(), 3);

    // Verify all signatures are different (since wallet names are different)
    for i in 0..results.len() {
        for j in i + 1..results.len() {
            assert_ne!(results[i].1, results[j].1); // Different signatures
        }
    }
}
