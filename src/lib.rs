//! # Dig Wallet Rust
//!
//! A comprehensive Rust wallet implementation for Chia blockchain with full DataLayer-Driver integration.
//!
//! ## Features
//!
//! - **Complete Wallet Management**: Create, import, and manage multiple wallets
//! - **Cryptographic Operations**: BIP39 mnemonics, BLS signatures, key derivation
//! - **Blockchain Integration**: Connect to Chia peers, select coins, check spendability
//! - **Secure Storage**: AES-256-GCM encrypted keyring storage
//! - **Address Handling**: Bech32m encoding/decoding for XCH addresses
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use dig_wallet::{Wallet, WalletError};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), WalletError> {
//!     // Create or load a wallet
//!     let wallet = Wallet::load(Some("my_wallet".to_string()), true).await?;
//!     
//!     // Get wallet address
//!     let address = wallet.get_owner_public_key().await?;
//!     println!("Wallet address: {}", address);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Peer Connection
//!
//! ```rust,no_run
//! use dig_wallet::Wallet;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to a random mainnet peer
//!     let peer = Wallet::connect_mainnet_peer().await?;
//!     
//!     // Use peer for blockchain operations
//!     let wallet = Wallet::load(Some("my_wallet".to_string()), true).await?;
//!     let coins = wallet.select_unspent_coins(&peer, 1000000, 1000, vec![]).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod wallet;
pub mod file_cache;

// Core exports
pub use error::WalletError;
pub use wallet::Wallet;
pub use file_cache::{FileCache, ReservedCoinCache};

// Re-export commonly used types from DataLayer-Driver
pub use datalayer_driver::{
    Peer, 
    NetworkType, 
    Coin, 
    CoinSpend, 
    Bytes32, 
    PublicKey, 
    SecretKey,
    Signature,
};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
