use crate::error::WalletError;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use bip39::{Language, Mnemonic};
use chia_wallet_sdk::driver::{Cat, Puzzle};
use chia_wallet_sdk::prelude::{Allocator, ToClvm, TreeHash};
use chia::puzzles::cat::CatArgs;
use chia_wallet_sdk::types::MAINNET_CONSTANTS;
use datalayer_driver::{address_to_puzzle_hash, connect_random, get_coin_id, master_public_key_to_first_puzzle_hash, master_public_key_to_wallet_synthetic_key, master_secret_key_to_wallet_synthetic_secret_key, puzzle_hash_to_address, secret_key_to_public_key, sign_message, verify_signature, Bytes, Bytes32, Coin, CoinSpend, NetworkType, Peer, PublicKey, SecretKey, Signature, UnspentCoinStates};
use hex_literal::hex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use chia::protocol::CoinState;

pub static DIG_MIN_HEIGHT: u32 = 5777842;
pub static DIG_COIN_ASSET_ID: Lazy<Bytes32> = Lazy::new(|| {
    Bytes32::new(hex!(
        "a406d3a9de984d03c9591c10d917593b434d5263cabe2b42f6b367df16832f81"
    ))
});
const KEYRING_FILE: &str = "keyring.json";
// Cache duration constant - keeping for potential future use
#[allow(dead_code)]
const CACHE_DURATION_MS: u64 = 5 * 60 * 1000; // 5 minutes
pub const DEFAULT_FEE_COIN_COST: u64 = 64_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedData {
    data: String,
    nonce: String,
    salt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyringData {
    wallets: HashMap<String, EncryptedData>,
}

pub struct Wallet {
    mnemonic: Option<String>,
    wallet_name: String,
}

impl Wallet {
    /// Create a new Wallet instance
    fn new(mnemonic: Option<String>, wallet_name: String) -> Self {
        Self {
            mnemonic,
            wallet_name,
        }
    }

    /// Load a wallet by name, optionally creating one if it doesn't exist
    pub async fn load(
        wallet_name: Option<String>,
        create_on_undefined: bool,
    ) -> Result<Self, WalletError> {
        let name = wallet_name.unwrap_or_else(|| "default".to_string());

        if let Some(mnemonic) = Self::get_wallet_from_keyring(&name).await? {
            return Ok(Self::new(Some(mnemonic), name));
        }

        if create_on_undefined {
            // In a real implementation, you'd prompt the user for input
            // For now, we'll generate a new wallet
            let new_mnemonic = Self::create_new_wallet(&name).await?;
            return Ok(Self::new(Some(new_mnemonic), name));
        }

        Err(WalletError::WalletNotFound(name))
    }

    /// Get the mnemonic seed phrase
    pub fn get_mnemonic(&self) -> Result<&str, WalletError> {
        self.mnemonic
            .as_deref()
            .ok_or(WalletError::MnemonicNotLoaded)
    }

    /// Get the wallet name
    pub fn get_wallet_name(&self) -> &str {
        &self.wallet_name
    }

    /// Create a new wallet with a generated mnemonic
    pub async fn create_new_wallet(wallet_name: &str) -> Result<String, WalletError> {
        let entropy = rand::random::<[u8; 32]>(); // 32 bytes = 256 bits for 24 words
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
            .map_err(|_| WalletError::CryptoError("Failed to generate mnemonic".to_string()))?;
        let mnemonic_str = mnemonic.to_string();
        Self::save_wallet_to_keyring(wallet_name, &mnemonic_str).await?;
        Ok(mnemonic_str)
    }

    /// Import a wallet from a provided mnemonic
    pub async fn import_wallet(
        wallet_name: &str,
        seed: Option<&str>,
    ) -> Result<String, WalletError> {
        let mnemonic_str = match seed {
            Some(s) => s.to_string(),
            None => {
                // In a real implementation, you'd prompt for input
                return Err(WalletError::MnemonicRequired);
            }
        };

        // Validate the mnemonic
        Mnemonic::parse_in_normalized(Language::English, &mnemonic_str)
            .map_err(|_| WalletError::InvalidMnemonic)?;

        Self::save_wallet_to_keyring(wallet_name, &mnemonic_str).await?;
        Ok(mnemonic_str)
    }

    /// Get the master secret key from the mnemonic
    pub async fn get_master_secret_key(&self) -> Result<SecretKey, WalletError> {
        let mnemonic_str = self.get_mnemonic()?;
        let mnemonic = Mnemonic::parse_in_normalized(Language::English, mnemonic_str)
            .map_err(|_| WalletError::InvalidMnemonic)?;

        let seed = mnemonic.to_seed("");
        let sk = SecretKey::from_seed(&seed);
        Ok(sk)
    }

    /// Get the public synthetic key
    pub async fn get_public_synthetic_key(&self) -> Result<PublicKey, WalletError> {
        let master_sk = self.get_master_secret_key().await?;
        let master_pk = secret_key_to_public_key(&master_sk);
        Ok(master_public_key_to_wallet_synthetic_key(&master_pk))
    }

    /// Get the private synthetic key
    pub async fn get_private_synthetic_key(&self) -> Result<SecretKey, WalletError> {
        let master_sk = self.get_master_secret_key().await?;
        Ok(master_secret_key_to_wallet_synthetic_secret_key(&master_sk))
    }

    /// Get the owner puzzle hash
    pub async fn get_owner_puzzle_hash(&self) -> Result<Bytes32, WalletError> {
        let master_sk = self.get_master_secret_key().await?;
        let master_pk = secret_key_to_public_key(&master_sk);
        Ok(master_public_key_to_first_puzzle_hash(&master_pk))
    }

    /// Get the owner public key as an address
    pub async fn get_owner_public_key(&self) -> Result<String, WalletError> {
        let owner_puzzle_hash = self.get_owner_puzzle_hash().await?;
        // Convert puzzle hash to address (xch format) using DataLayer-Driver
        puzzle_hash_to_address(owner_puzzle_hash, "xch")
            .map_err(|e| WalletError::CryptoError(format!("Failed to encode address: {}", e)))
    }

    /// Delete a wallet from the keyring
    pub async fn delete_wallet(wallet_name: &str) -> Result<bool, WalletError> {
        let keyring_path = Self::get_keyring_path()?;

        if !keyring_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&keyring_path)
            .map_err(|e| WalletError::FileSystemError(e.to_string()))?;

        let mut keyring: KeyringData = serde_json::from_str(&content)
            .map_err(|e| WalletError::SerializationError(e.to_string()))?;

        if keyring.wallets.remove(wallet_name).is_some() {
            let updated_content = serde_json::to_string_pretty(&keyring)
                .map_err(|e| WalletError::SerializationError(e.to_string()))?;

            fs::write(&keyring_path, updated_content)
                .map_err(|e| WalletError::FileSystemError(e.to_string()))?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all wallets in the keyring
    pub async fn list_wallets() -> Result<Vec<String>, WalletError> {
        let keyring_path = Self::get_keyring_path()?;

        if !keyring_path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&keyring_path)
            .map_err(|e| WalletError::FileSystemError(e.to_string()))?;

        let keyring: KeyringData = serde_json::from_str(&content)
            .map_err(|e| WalletError::SerializationError(e.to_string()))?;

        Ok(keyring.wallets.keys().cloned().collect())
    }

    /// Create a key ownership signature
    pub async fn create_key_ownership_signature(&self, nonce: &str) -> Result<String, WalletError> {
        let message = format!(
            "Signing this message to prove ownership of key.\n\nNonce: {}",
            nonce
        );
        let private_synthetic_key = self.get_private_synthetic_key().await?;

        let signature = sign_message(
            &Bytes::from(message.as_bytes().to_vec()),
            &private_synthetic_key,
        )
        .map_err(|e| WalletError::CryptoError(e.to_string()))?;

        Ok(hex::encode(signature.to_bytes()))
    }

    /// Verify a key ownership signature
    pub async fn verify_key_ownership_signature(
        nonce: &str,
        signature: &str,
        public_key: &str,
    ) -> Result<bool, WalletError> {
        let message = format!(
            "Signing this message to prove ownership of key.\n\nNonce: {}",
            nonce
        );

        let sig_bytes =
            hex::decode(signature).map_err(|e| WalletError::CryptoError(e.to_string()))?;

        let pk_bytes =
            hex::decode(public_key).map_err(|e| WalletError::CryptoError(e.to_string()))?;

        if pk_bytes.len() != 48 {
            return Err(WalletError::CryptoError(
                "Invalid public key length".to_string(),
            ));
        }

        let mut pk_array = [0u8; 48];
        pk_array.copy_from_slice(&pk_bytes);

        let public_key = PublicKey::from_bytes(&pk_array)
            .map_err(|e| WalletError::CryptoError(e.to_string()))?;

        if sig_bytes.len() != 96 {
            return Err(WalletError::CryptoError(
                "Invalid signature length".to_string(),
            ));
        }

        let mut sig_array = [0u8; 96];
        sig_array.copy_from_slice(&sig_bytes);

        let signature = Signature::from_bytes(&sig_array)
            .map_err(|e| WalletError::CryptoError(e.to_string()))?;

        verify_signature(
            Bytes::from(message.as_bytes().to_vec()),
            public_key,
            signature,
        )
        .map_err(|e| WalletError::CryptoError(e.to_string()))
    }

    /// Get all unspent DIG Token coins
    // todo: this should be moved to the driver
    pub async fn get_all_unspent_dig_coins(
        &self,
        peer: &Peer,
        omit_coins: Vec<Coin>,
        verbose: bool,
    ) -> Result<Vec<Coin>, WalletError> {
        let p2 = self.get_owner_puzzle_hash().await?;
        let dig_cat_ph = CatArgs::curry_tree_hash(*DIG_COIN_ASSET_ID, TreeHash::from(p2));
        let dig_cat_ph_bytes = Bytes32::from(dig_cat_ph.to_bytes());

        // Get unspent coin states from the DataLayer-Driver async API
        let unspent_coin_states = datalayer_driver::async_api::get_all_unspent_coins(
            peer,
            dig_cat_ph_bytes,
            None, // previous_height - start from genesis
            datalayer_driver::constants::get_mainnet_genesis_challenge(), // Use mainnet for now
        )
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get unspent coins: {}", e)))?;

        // Convert coin states to coins and filter out omitted coins
        let omit_coin_ids: Vec<Bytes32> = omit_coins.iter().map(get_coin_id).collect();

        let available_coin_states: Vec<CoinState> = unspent_coin_states
            .coin_states
            .into_iter()
            .filter(|coin_state| !omit_coin_ids.contains(&get_coin_id(&coin_state.coin)))
            .collect();

        let mut proved_dig_token_coins: Vec<Coin> = vec![];

        let mut allocator = Allocator::new();

        for coin_state in &available_coin_states {
            let coin = &coin_state.coin;
            let coin_id = coin.coin_id();
            let coin_created_height = match coin_state.created_height {
                Some(height) => height,
                None => {
                    if verbose {
                        eprintln!(
                            "ERROR: coin_id {} | {}",
                            coin_id,
                            WalletError::CoinSetError("Cannot determine coin creation height".to_string())
                        );
                    }
                    continue;
                }
            };


            // 1) Request parent coin state
            let parent_state_result = peer
                .request_coin_state(
                    vec![coin.parent_coin_info],
                    None,
                    MAINNET_CONSTANTS.genesis_challenge,
                    false,
                )
                .await;

            let parent_state_response = match parent_state_result {
                Ok(response) => response,
                Err(error) => {
                    if verbose {
                        eprintln!(
                            "ERROR: coin_id {} | {}",
                            coin_id,
                            WalletError::NetworkError(format!(
                                "Failed to get coin state: {}",
                                error
                            ))
                        );
                    }
                    continue;
                }
            };

            let parent_state = match parent_state_response {
                Ok(state) => state,
                Err(_) => {
                    if verbose {
                        eprintln!(
                            "ERROR: coin_id {} | {}",
                            coin_id,
                            WalletError::CoinSetError("Coin state rejected".to_string())
                        );
                    }
                    continue;
                }
            };

            // 2) Request parent puzzle and solution
            let parent_puzzle_and_solution_result = peer
                .request_puzzle_and_solution(parent_state.coin_ids[0], coin_created_height)
                .await;

            let parent_puzzle_and_solution_response = match parent_puzzle_and_solution_result {
                Ok(response) => response,
                Err(error) => {
                    if verbose {
                        eprintln!(
                            "ERROR: coin_id {} | {}",
                            coin_id,
                            WalletError::NetworkError(format!(
                                "Failed to get puzzle and solution: {}",
                                error
                            ))
                        );
                    }
                    continue;
                }
            };

            let parent_puzzle_and_solution = match parent_puzzle_and_solution_response {
                Ok(v) => v,
                Err(_) => {
                    if verbose {
                        eprintln!(
                            "ERROR: coin_id {} | {}",
                            coin_id,
                            WalletError::CoinSetError("Parent puzzle solution rejected".to_string())
                        );
                    }
                    continue;
                }
            };

            // 3) Convert puzzle to CLVM
            let parent_puzzle_ptr = match parent_puzzle_and_solution.puzzle.to_clvm(&mut allocator) {
                Ok(ptr) => ptr,
                Err(error) => {
                    if verbose {
                        eprintln!(
                            "ERROR: coin_id {} | {}",
                            coin_id,
                            WalletError::CoinSetError(format!(
                                "Failed to parse puzzle and solution: {}",
                                error
                            ))
                        );
                    }
                    continue;
                }
            };

            let parent_puzzle = Puzzle::parse(&allocator, parent_puzzle_ptr);

            // 4) Convert solution to CLVM
            let parent_solution = match parent_puzzle_and_solution.solution.to_clvm(&mut allocator) {
                Ok(solution) => solution,
                Err(error) => {
                    if verbose {
                        eprintln!(
                            "ERROR: coin_id {} | {}",
                            coin_id,
                            WalletError::CoinSetError(format!(
                                "Failed to parse puzzle and solution: {}",
                                error
                            ))
                        );
                    }
                    continue;
                }
            };

            // 5) Parse CAT to prove lineage
            let cat_parse_result = Cat::parse_children(
                &mut allocator,
                parent_state.coin_states[0].coin,
                parent_puzzle,
                parent_solution,
            );
            match cat_parse_result {
                Ok(_) => {
                    // lineage proved. append coin in question
                    proved_dig_token_coins.push(*coin);
                }
                Err(error) => {
                    if verbose {
                        eprintln!(
                            "ERROR: coin_id {} | {}",
                            coin_id,
                            WalletError::CoinSetError(format!(
                                "Failed to parse CAT and prove lineage: {}",
                                error
                            ))
                        );
                    }
                    continue;
                }
            }
        }

        Ok(proved_dig_token_coins)
    }

    pub async fn select_unspent_dig_token_coins(
        &self,
        peer: &Peer,
        coin_amount: u64,
        fee: u64,
        omit_coins: Vec<Coin>,
        verbose: bool,
    ) -> Result<Vec<Coin>, WalletError> {
        let total_needed = coin_amount + fee;
        let available_dig_coins = self
            .get_all_unspent_dig_coins(peer, omit_coins, verbose)
            .await?;

        // Use the DataLayer-Driver's select_coins function
        let selected_coins = datalayer_driver::select_coins(&available_dig_coins, total_needed)
            .map_err(|e| WalletError::DataLayerError(format!("Coin selection failed: {}", e)))?;

        if selected_coins.is_empty() {
            return Err(WalletError::NoUnspentCoins);
        }

        Ok(selected_coins)
    }

    pub async fn get_dig_balance(&self, peer: &Peer, verbose: bool) -> Result<u64, WalletError> {
        let dig_coins = self
            .get_all_unspent_dig_coins(peer, vec![], verbose)
            .await?;
        let dig_balance = dig_coins.iter().map(|c| c.amount).sum::<u64>();
        Ok(dig_balance)
    }

    pub async fn get_all_unspent_xch_coins(
        &self,
        peer: &Peer,
        omit_coins: Vec<Coin>,
    ) -> Result<Vec<Coin>, WalletError> {
        let owner_puzzle_hash = self.get_owner_puzzle_hash().await?;

        let coin_states = datalayer_driver::async_api::get_all_unspent_coins(
            peer,
            owner_puzzle_hash,
            None, // previous_height - start from genesis
            datalayer_driver::constants::get_mainnet_genesis_challenge(), // Use mainnet for now
        )
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get unspent coins: {}", e)))?;

        // Convert coin states to coins and filter out omitted coins
        let omit_coin_ids: Vec<Bytes32> = omit_coins.iter().map(get_coin_id).collect();

        Ok(coin_states
            .coin_states
            .into_iter()
            .map(|cs| cs.coin)
            .filter(|coin| !omit_coin_ids.contains(&get_coin_id(coin)))
            .collect())
    }

    /// Select unspent coins for spending
    pub async fn select_unspent_coins(
        &self,
        peer: &Peer,
        coin_amount: u64,
        fee: u64,
        omit_coins: Vec<Coin>,
    ) -> Result<Vec<Coin>, WalletError> {
        let total_needed = coin_amount + fee;

        let available_coins = self.get_all_unspent_xch_coins(peer, omit_coins).await?;

        // Use the DataLayer-Driver's select_coins function
        let selected_coins = datalayer_driver::select_coins(&available_coins, total_needed)
            .map_err(|e| WalletError::DataLayerError(format!("Coin selection failed: {}", e)))?;

        if selected_coins.is_empty() {
            return Err(WalletError::NoUnspentCoins);
        }

        Ok(selected_coins)
    }

    pub async fn get_xch_balance(&self, peer: &Peer) -> Result<u64, WalletError> {
        let xch_coins = self.get_all_unspent_xch_coins(peer, vec![]).await?;
        let xch_balance = xch_coins.iter().map(|c| c.amount).sum::<u64>();
        Ok(xch_balance)
    }

    /// Calculate fee for coin spends
    pub async fn calculate_fee_for_coin_spends(
        _peer: &Peer,
        _coin_spends: Option<&[CoinSpend]>,
    ) -> Result<u64, WalletError> {
        // Simplified fee calculation - in practice this would be more complex
        Ok(1_000_000) // 1 million mojos
    }

    /// Check if a coin is spendable
    pub async fn is_coin_spendable(peer: &Peer, coin_id: &Bytes32) -> Result<bool, WalletError> {
        // Check if coin is spent using the DataLayer-Driver API
        let is_spent = datalayer_driver::is_coin_spent(
            peer,
            *coin_id,
            None,                                                         // last_height
            datalayer_driver::constants::get_mainnet_genesis_challenge(), // Use mainnet for now
        )
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to check coin status: {}", e)))?;

        // Return true if coin is NOT spent (i.e., is spendable)
        Ok(!is_spent)
    }

    /// Connect to a random peer on the specified network
    pub async fn connect_random_peer(
        network: NetworkType,
        cert_path: &str,
        key_path: &str,
    ) -> Result<Peer, WalletError> {
        connect_random(network, cert_path, key_path)
            .await
            .map_err(|e| WalletError::NetworkError(format!("Failed to connect to peer: {}", e)))
    }

    /// Connect to a random mainnet peer using default Chia SSL paths
    pub async fn connect_mainnet_peer() -> Result<Peer, WalletError> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            WalletError::FileSystemError("Could not find home directory".to_string())
        })?;

        let ssl_dir = home_dir
            .join(".chia")
            .join("mainnet")
            .join("config")
            .join("ssl")
            .join("wallet");
        let cert_path = ssl_dir.join("wallet_node.crt");
        let key_path = ssl_dir.join("wallet_node.key");

        Self::connect_random_peer(
            NetworkType::Mainnet,
            cert_path
                .to_str()
                .ok_or_else(|| WalletError::FileSystemError("Invalid cert path".to_string()))?,
            key_path
                .to_str()
                .ok_or_else(|| WalletError::FileSystemError("Invalid key path".to_string()))?,
        )
        .await
    }

    /// Connect to a random testnet peer using default Chia SSL paths
    pub async fn connect_testnet_peer() -> Result<Peer, WalletError> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            WalletError::FileSystemError("Could not find home directory".to_string())
        })?;

        let ssl_dir = home_dir
            .join(".chia")
            .join("testnet11")
            .join("config")
            .join("ssl")
            .join("wallet");
        let cert_path = ssl_dir.join("wallet_node.crt");
        let key_path = ssl_dir.join("wallet_node.key");

        Self::connect_random_peer(
            NetworkType::Testnet11,
            cert_path
                .to_str()
                .ok_or_else(|| WalletError::FileSystemError("Invalid cert path".to_string()))?,
            key_path
                .to_str()
                .ok_or_else(|| WalletError::FileSystemError("Invalid key path".to_string()))?,
        )
        .await
    }

    /// Convert an address to a puzzle hash
    pub fn address_to_puzzle_hash(address: &str) -> Result<Bytes32, WalletError> {
        address_to_puzzle_hash(address)
            .map_err(|e| WalletError::CryptoError(format!("Failed to decode address: {}", e)))
    }

    /// Convert a puzzle hash to an address
    pub fn puzzle_hash_to_address(
        puzzle_hash: Bytes32,
        prefix: &str,
    ) -> Result<String, WalletError> {
        puzzle_hash_to_address(puzzle_hash, prefix)
            .map_err(|e| WalletError::CryptoError(format!("Failed to encode address: {}", e)))
    }

    // Private helper methods

    async fn get_wallet_from_keyring(wallet_name: &str) -> Result<Option<String>, WalletError> {
        let keyring_path = Self::get_keyring_path()?;

        if !keyring_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&keyring_path)
            .map_err(|e| WalletError::FileSystemError(e.to_string()))?;

        let keyring: KeyringData = serde_json::from_str(&content)
            .map_err(|e| WalletError::SerializationError(e.to_string()))?;

        if let Some(encrypted_data) = keyring.wallets.get(wallet_name) {
            let decrypted = Self::decrypt_data(encrypted_data)?;
            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }

    async fn save_wallet_to_keyring(wallet_name: &str, mnemonic: &str) -> Result<(), WalletError> {
        let keyring_path = Self::get_keyring_path()?;

        // Ensure the directory exists
        if let Some(parent) = keyring_path.parent() {
            fs::create_dir_all(parent).map_err(|e| WalletError::FileSystemError(e.to_string()))?;
        }

        let mut keyring = if keyring_path.exists() {
            let content = fs::read_to_string(&keyring_path)
                .map_err(|e| WalletError::FileSystemError(e.to_string()))?;
            serde_json::from_str(&content)
                .map_err(|e| WalletError::SerializationError(e.to_string()))?
        } else {
            KeyringData {
                wallets: HashMap::new(),
            }
        };

        let encrypted_data = Self::encrypt_data(mnemonic)?;

        keyring
            .wallets
            .insert(wallet_name.to_string(), encrypted_data);

        let content = serde_json::to_string_pretty(&keyring)
            .map_err(|e| WalletError::SerializationError(e.to_string()))?;

        fs::write(&keyring_path, content)
            .map_err(|e| WalletError::FileSystemError(e.to_string()))?;

        Ok(())
    }

    fn get_keyring_path() -> Result<PathBuf, WalletError> {
        // Check if we're in test mode by looking for TEST_KEYRING_PATH env var
        if let Ok(test_path) = env::var("TEST_KEYRING_PATH") {
            return Ok(PathBuf::from(test_path));
        }

        let home_dir = dirs::home_dir().ok_or_else(|| {
            WalletError::FileSystemError("Could not find home directory".to_string())
        })?;

        Ok(home_dir.join(".dig").join(KEYRING_FILE))
    }

    /// Encrypt data using AES-256-GCM
    fn encrypt_data(data: &str) -> Result<EncryptedData, WalletError> {
        // Generate a random salt
        let salt = rand::random::<[u8; 16]>();

        // Derive key from a fixed password and salt using a simple method
        // In production, you'd want to use a proper key derivation function like PBKDF2
        let mut key_bytes = [0u8; 32];
        let password = b"mnemonic-seed"; // This should be derived from user input in practice

        // Simple key derivation (not cryptographically secure - use PBKDF2 in production)
        for i in 0..32 {
            key_bytes[i] = password[i % password.len()] ^ salt[i % salt.len()];
        }

        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        // Generate a random nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt the data
        let ciphertext = cipher
            .encrypt(&nonce, data.as_bytes())
            .map_err(|e| WalletError::CryptoError(format!("Encryption failed: {}", e)))?;

        Ok(EncryptedData {
            data: general_purpose::STANDARD.encode(&ciphertext),
            nonce: general_purpose::STANDARD.encode(nonce),
            salt: general_purpose::STANDARD.encode(salt),
        })
    }

    /// Decrypt data using AES-256-GCM
    fn decrypt_data(encrypted_data: &EncryptedData) -> Result<String, WalletError> {
        let ciphertext = general_purpose::STANDARD
            .decode(&encrypted_data.data)
            .map_err(|e| WalletError::CryptoError(format!("Failed to decode ciphertext: {}", e)))?;

        let nonce_bytes = general_purpose::STANDARD
            .decode(&encrypted_data.nonce)
            .map_err(|e| WalletError::CryptoError(format!("Failed to decode nonce: {}", e)))?;

        let salt = general_purpose::STANDARD
            .decode(&encrypted_data.salt)
            .map_err(|e| WalletError::CryptoError(format!("Failed to decode salt: {}", e)))?;

        // Derive the same key using the salt
        let mut key_bytes = [0u8; 32];
        let password = b"mnemonic-seed";

        for i in 0..32 {
            key_bytes[i] = password[i % password.len()] ^ salt[i % salt.len()];
        }

        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decrypt the data
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| WalletError::CryptoError(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext).map_err(|e| {
            WalletError::CryptoError(format!("Failed to convert decrypted data to string: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    // Test helper to set up a temporary directory for tests
    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Set up isolated keyring path for this test
        let keyring_path = temp_dir.path().join("test_keyring.json");
        env::set_var(
            "TEST_KEYRING_PATH",
            keyring_path.to_string_lossy().to_string(),
        );

        // Also set HOME for any other path operations
        env::set_var("HOME", temp_dir.path());

        temp_dir
    }

    #[tokio::test]
    async fn test_wallet_creation() {
        let _temp_dir = setup_test_env();

        // Create a new wallet
        let mnemonic = Wallet::create_new_wallet("test_wallet").await.unwrap();

        // Verify mnemonic is valid BIP39
        assert!(bip39::Mnemonic::parse_in_normalized(Language::English, &mnemonic).is_ok());

        // Verify mnemonic has 24 words
        assert_eq!(mnemonic.split_whitespace().count(), 24);

        // Verify wallet appears in list
        let wallets = Wallet::list_wallets().await.unwrap();
        assert!(wallets.contains(&"test_wallet".to_string()));
    }

    #[tokio::test]
    async fn test_wallet_import() {
        let _temp_dir = setup_test_env();

        // Known valid 24-word mnemonic
        let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

        // Import the wallet
        let imported_mnemonic = Wallet::import_wallet("imported_wallet", Some(test_mnemonic))
            .await
            .unwrap();

        // Verify the mnemonic matches
        assert_eq!(imported_mnemonic, test_mnemonic);

        // Load the wallet and verify mnemonic
        let wallet = Wallet::load(Some("imported_wallet".to_string()), false)
            .await
            .unwrap();
        assert_eq!(wallet.get_mnemonic().unwrap(), test_mnemonic);
    }

    #[tokio::test]
    async fn test_wallet_import_invalid_mnemonic() {
        let _temp_dir = setup_test_env();

        // Invalid mnemonic
        let invalid_mnemonic = "invalid mnemonic phrase that should fail validation";

        // Should fail with InvalidMnemonic error
        let result = Wallet::import_wallet("invalid_wallet", Some(invalid_mnemonic)).await;
        assert!(matches!(result, Err(WalletError::InvalidMnemonic)));
    }

    #[tokio::test]
    async fn test_wallet_load_nonexistent() {
        let _temp_dir = setup_test_env();

        // Try to load non-existent wallet without creating
        let result = Wallet::load(Some("nonexistent".to_string()), false).await;
        assert!(matches!(result, Err(WalletError::WalletNotFound(_))));
    }

    #[tokio::test]
    async fn test_wallet_load_with_creation() {
        let _temp_dir = setup_test_env();

        // Load wallet with auto-creation
        let wallet = Wallet::load(Some("auto_created".to_string()), true)
            .await
            .unwrap();

        // Verify wallet was created and has valid mnemonic
        let mnemonic = wallet.get_mnemonic().unwrap();
        assert!(bip39::Mnemonic::parse_in_normalized(Language::English, mnemonic).is_ok());

        // Verify wallet name
        assert_eq!(wallet.get_wallet_name(), "auto_created");
    }

    #[tokio::test]
    async fn test_key_derivation() {
        let _temp_dir = setup_test_env();

        // Use known mnemonic for deterministic testing
        let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

        Wallet::import_wallet("key_test", Some(test_mnemonic))
            .await
            .unwrap();
        let wallet = Wallet::load(Some("key_test".to_string()), false)
            .await
            .unwrap();

        // Test key derivation
        let master_sk = wallet.get_master_secret_key().await.unwrap();
        let public_synthetic_key = wallet.get_public_synthetic_key().await.unwrap();
        let private_synthetic_key = wallet.get_private_synthetic_key().await.unwrap();
        let puzzle_hash = wallet.get_owner_puzzle_hash().await.unwrap();

        // Verify keys are consistent
        assert_eq!(
            secret_key_to_public_key(&private_synthetic_key),
            public_synthetic_key
        );

        // Verify puzzle hash is 32 bytes
        assert_eq!(puzzle_hash.as_ref().len(), 32);

        // Test that keys are deterministic (same mnemonic = same keys)
        let wallet2 = Wallet::load(Some("key_test".to_string()), false)
            .await
            .unwrap();
        let master_sk2 = wallet2.get_master_secret_key().await.unwrap();
        assert_eq!(master_sk.to_bytes(), master_sk2.to_bytes());
    }

    #[tokio::test]
    async fn test_address_generation() {
        let _temp_dir = setup_test_env();

        let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

        Wallet::import_wallet("address_test", Some(test_mnemonic))
            .await
            .unwrap();
        let wallet = Wallet::load(Some("address_test".to_string()), false)
            .await
            .unwrap();

        // Generate address
        let address = wallet.get_owner_public_key().await.unwrap();

        // Verify address format (should start with "xch1")
        assert!(address.starts_with("xch1"));

        // Verify address length (Chia addresses are typically 62 characters)
        assert!(address.len() >= 60 && address.len() <= 65);

        // Test address conversion roundtrip
        let puzzle_hash = Wallet::address_to_puzzle_hash(&address).unwrap();
        let converted_address = Wallet::puzzle_hash_to_address(puzzle_hash, "xch").unwrap();
        assert_eq!(address, converted_address);
    }

    #[tokio::test]
    async fn test_signature_creation_and_verification() {
        let _temp_dir = setup_test_env();

        let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

        Wallet::import_wallet("sig_test", Some(test_mnemonic))
            .await
            .unwrap();
        let wallet = Wallet::load(Some("sig_test".to_string()), false)
            .await
            .unwrap();

        // Create signature
        let nonce = "test_nonce_12345";
        let signature = wallet.create_key_ownership_signature(nonce).await.unwrap();

        // Verify signature format (should be hex string)
        assert!(hex::decode(&signature).is_ok());

        // Get public key for verification
        let public_key = wallet.get_public_synthetic_key().await.unwrap();
        let public_key_hex = hex::encode(public_key.to_bytes());

        // Verify signature
        let is_valid = Wallet::verify_key_ownership_signature(nonce, &signature, &public_key_hex)
            .await
            .unwrap();
        assert!(is_valid);

        // Test with wrong nonce (should fail)
        let is_valid_wrong =
            Wallet::verify_key_ownership_signature("wrong_nonce", &signature, &public_key_hex)
                .await
                .unwrap();
        assert!(!is_valid_wrong);
    }

    #[tokio::test]
    async fn test_wallet_deletion() {
        let _temp_dir = setup_test_env();

        // Create wallet
        Wallet::create_new_wallet("delete_test").await.unwrap();

        // Verify it exists
        let wallets_before = Wallet::list_wallets().await.unwrap();
        assert!(wallets_before.contains(&"delete_test".to_string()));

        // Delete wallet
        let deleted = Wallet::delete_wallet("delete_test").await.unwrap();
        assert!(deleted);

        // Verify it's gone
        let wallets_after = Wallet::list_wallets().await.unwrap();
        assert!(!wallets_after.contains(&"delete_test".to_string()));

        // Try to delete non-existent wallet
        let not_deleted = Wallet::delete_wallet("nonexistent").await.unwrap();
        assert!(!not_deleted);
    }

    #[tokio::test]
    async fn test_multiple_wallets() {
        let _temp_dir = setup_test_env();

        // Create multiple wallets
        Wallet::create_new_wallet("wallet1").await.unwrap();
        Wallet::create_new_wallet("wallet2").await.unwrap();
        Wallet::create_new_wallet("wallet3").await.unwrap();

        // List wallets
        let mut wallets = Wallet::list_wallets().await.unwrap();
        wallets.sort(); // Sort for consistent testing

        assert_eq!(wallets.len(), 3);
        assert!(wallets.contains(&"wallet1".to_string()));
        assert!(wallets.contains(&"wallet2".to_string()));
        assert!(wallets.contains(&"wallet3".to_string()));

        // Load each wallet and verify they have different mnemonics
        let w1 = Wallet::load(Some("wallet1".to_string()), false)
            .await
            .unwrap();
        let w2 = Wallet::load(Some("wallet2".to_string()), false)
            .await
            .unwrap();
        let w3 = Wallet::load(Some("wallet3".to_string()), false)
            .await
            .unwrap();

        assert_ne!(w1.get_mnemonic().unwrap(), w2.get_mnemonic().unwrap());
        assert_ne!(w2.get_mnemonic().unwrap(), w3.get_mnemonic().unwrap());
        assert_ne!(w1.get_mnemonic().unwrap(), w3.get_mnemonic().unwrap());
    }

    #[tokio::test]
    async fn test_encryption_decryption() {
        // Test encryption/decryption directly
        let test_data = "test mnemonic phrase for encryption";

        let encrypted = Wallet::encrypt_data(test_data).unwrap();

        // Verify encrypted data is different from original
        assert_ne!(encrypted.data, test_data);
        assert!(!encrypted.nonce.is_empty());
        assert!(!encrypted.salt.is_empty());

        // Decrypt and verify
        let decrypted = Wallet::decrypt_data(&encrypted).unwrap();
        assert_eq!(decrypted, test_data);
    }

    #[tokio::test]
    async fn test_encryption_with_different_salts() {
        let test_data = "same data";

        // Encrypt same data twice
        let encrypted1 = Wallet::encrypt_data(test_data).unwrap();
        let encrypted2 = Wallet::encrypt_data(test_data).unwrap();

        // Should produce different ciphertexts due to random salt/nonce
        assert_ne!(encrypted1.data, encrypted2.data);
        assert_ne!(encrypted1.salt, encrypted2.salt);
        assert_ne!(encrypted1.nonce, encrypted2.nonce);

        // But both should decrypt to same data
        let decrypted1 = Wallet::decrypt_data(&encrypted1).unwrap();
        let decrypted2 = Wallet::decrypt_data(&encrypted2).unwrap();
        assert_eq!(decrypted1, test_data);
        assert_eq!(decrypted2, test_data);
    }

    #[tokio::test]
    async fn test_invalid_signature_verification() {
        let _temp_dir = setup_test_env();

        // Create wallet
        let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
        Wallet::import_wallet("invalid_sig_test", Some(test_mnemonic))
            .await
            .unwrap();
        let wallet = Wallet::load(Some("invalid_sig_test".to_string()), false)
            .await
            .unwrap();

        let public_key = wallet.get_public_synthetic_key().await.unwrap();
        let public_key_hex = hex::encode(public_key.to_bytes());

        // Test with invalid signature format
        let result =
            Wallet::verify_key_ownership_signature("nonce", "invalid_hex", &public_key_hex).await;
        assert!(result.is_err());

        // Test with wrong signature length
        let short_sig = "deadbeef";
        let result =
            Wallet::verify_key_ownership_signature("nonce", short_sig, &public_key_hex).await;
        assert!(result.is_err());

        // Test with invalid public key
        let result =
            Wallet::verify_key_ownership_signature("nonce", &"a".repeat(192), "invalid_key").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_address_conversion_errors() {
        // Test invalid address
        let result = Wallet::address_to_puzzle_hash("invalid_address");
        assert!(result.is_err());

        // Test empty address
        let result = Wallet::address_to_puzzle_hash("");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mnemonic_not_loaded_error() {
        // Create wallet without mnemonic
        let wallet = Wallet::new(None, "empty_wallet".to_string());

        // Should fail when trying to get mnemonic
        let result = wallet.get_mnemonic();
        assert!(matches!(result, Err(WalletError::MnemonicNotLoaded)));

        // Should fail when trying to derive keys
        let result = wallet.get_master_secret_key().await;
        assert!(matches!(result, Err(WalletError::MnemonicNotLoaded)));
    }

    #[tokio::test]
    async fn test_default_wallet_name() {
        let _temp_dir = setup_test_env();

        // Load wallet without specifying name (should use "default")
        let wallet = Wallet::load(None, true).await.unwrap();
        assert_eq!(wallet.get_wallet_name(), "default");

        // Verify it appears in wallet list
        let wallets = Wallet::list_wallets().await.unwrap();
        assert!(wallets.contains(&"default".to_string()));
    }
}
