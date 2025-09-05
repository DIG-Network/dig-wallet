# Dig Wallet Rust

A comprehensive Rust implementation of a Chia wallet with full feature parity to the TypeScript version, built using the DataLayer-Driver v0.1.50.

## 🚀 Features

### ✅ **Complete Wallet Management**
- **Wallet Creation**: Generate new wallets with secure 24-word BIP39 mnemonics
- **Wallet Import**: Import existing wallets from mnemonic seed phrases
- **Multiple Wallets**: Support for managing multiple named wallets
- **Secure Storage**: AES-256-GCM encrypted keyring storage

### ✅ **Full Cryptographic Support**
- **Key Derivation**: BIP39 compliant mnemonic to key derivation
- **Digital Signatures**: BLS signature creation and verification
- **Address Generation**: Proper XCH address encoding using bech32m
- **Deterministic**: Same mnemonic always generates same keys/addresses

### ✅ **Blockchain Integration**
- **Peer Connection**: Connect to random Chia peers using `connect_random`
- **Coin Operations**: Select unspent coins and check spendability
- **Network Support**: Both mainnet and testnet11 support
- **SSL Integration**: Automatic Chia SSL certificate detection

### ✅ **Advanced Features**
- **File Caching**: Generic file-based caching system
- **Error Handling**: Comprehensive error types and handling
- **Address Conversion**: Bidirectional puzzle hash ↔ address conversion
- **Memory Safety**: Rust's ownership system prevents common security issues

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
dig-wallet = "0.1.0"
datalayer-driver = "0.1.50"
tokio = { version = "1.0", features = ["full"] }
```

## 🔧 Usage

### Basic Wallet Operations

```rust
use dig_wallet::{Wallet, WalletError};

#[tokio::main]
async fn main() -> Result<(), WalletError> {
    // Create or load a wallet
    let wallet = Wallet::load(Some("my_wallet".to_string()), true).await?;
    
    // Get wallet information
    let mnemonic = wallet.get_mnemonic()?;
    let address = wallet.get_owner_public_key().await?;
    
    println!("Address: {}", address);
    Ok(())
}
```

### Peer Connection and Coin Operations

```rust
use dig_wallet::Wallet;
use datalayer_driver::NetworkType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a random mainnet peer
    let peer = Wallet::connect_mainnet_peer().await?;
    
    // Or connect with custom SSL certificates
    let peer = Wallet::connect_random_peer(
        NetworkType::Mainnet,
        "/path/to/cert.crt",
        "/path/to/key.key"
    ).await?;
    
    // Load wallet and select coins
    let wallet = Wallet::load(Some("my_wallet".to_string()), true).await?;
    let coins = wallet.select_unspent_coins(&peer, 1000000, 1000, vec![]).await?;
    
    println!("Selected {} coins", coins.len());
    Ok(())
}
```

### Digital Signatures

```rust
use dig_wallet::Wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wallet = Wallet::load(Some("my_wallet".to_string()), true).await?;
    
    // Create a signature
    let signature = wallet.create_key_ownership_signature("my_nonce").await?;
    
    // Verify the signature
    let public_key = wallet.get_public_synthetic_key().await?;
    let public_key_hex = hex::encode(public_key.to_bytes());
    
    let is_valid = Wallet::verify_key_ownership_signature(
        "my_nonce", 
        &signature, 
        &public_key_hex
    ).await?;
    
    println!("Signature valid: {}", is_valid);
    Ok(())
}
```

### Address Conversion

```rust
use dig_wallet::Wallet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = "xch1qm9g5qxq4xrqqkpvmk6j64ckpjp7xeq78mdmfz48lp8g5zgq4sxs2370ec";
    
    // Convert address to puzzle hash
    let puzzle_hash = Wallet::address_to_puzzle_hash(address)?;
    
    // Convert back to address
    let converted_address = Wallet::puzzle_hash_to_address(puzzle_hash, "xch")?;
    
    assert_eq!(address, converted_address);
    Ok(())
}
```

## 🧪 Testing

The project includes comprehensive test coverage with 24 tests covering all functionality:

```bash
# Run all tests
cargo test -- --test-threads=1

# Run only unit tests
cargo test --lib -- --test-threads=1

# Run only integration tests
cargo test --test integration_tests -- --test-threads=1

# Run example
cargo run --example wallet_usage
```

### Test Coverage
- ✅ **17 Unit Tests**: Core functionality, cryptography, error handling
- ✅ **7 Integration Tests**: Full lifecycle, edge cases, concurrency
- ✅ **100% Pass Rate**: All tests consistently pass
- ✅ **Comprehensive Coverage**: All public APIs and error paths tested

See [TEST_COVERAGE.md](TEST_COVERAGE.md) for detailed test documentation.

## 📚 API Reference

### Core Types

```rust
pub struct Wallet {
    // Private fields
}

pub enum WalletError {
    MnemonicRequired,
    InvalidMnemonic,
    MnemonicNotLoaded,
    WalletNotFound(String),
    CryptoError(String),
    NetworkError(String),
    FileSystemError(String),
    // ... more error types
}
```

### Main Methods

#### Wallet Management
- `Wallet::load(name, create_on_undefined)` - Load or create wallet
- `Wallet::create_new_wallet(name)` - Create wallet with new mnemonic
- `Wallet::import_wallet(name, mnemonic)` - Import wallet from mnemonic
- `Wallet::delete_wallet(name)` - Delete wallet from keyring
- `Wallet::list_wallets()` - List all stored wallets

#### Key Operations
- `wallet.get_mnemonic()` - Get mnemonic seed phrase
- `wallet.get_master_secret_key()` - Get master secret key
- `wallet.get_public_synthetic_key()` - Get public synthetic key
- `wallet.get_private_synthetic_key()` - Get private synthetic key
- `wallet.get_owner_puzzle_hash()` - Get puzzle hash
- `wallet.get_owner_public_key()` - Get XCH address

#### Signatures
- `wallet.create_key_ownership_signature(nonce)` - Create signature
- `Wallet::verify_key_ownership_signature(nonce, sig, pubkey)` - Verify signature

#### Peer Operations
- `Wallet::connect_mainnet_peer()` - Connect to mainnet with default SSL
- `Wallet::connect_testnet_peer()` - Connect to testnet with default SSL
- `Wallet::connect_random_peer(network, cert, key)` - Connect with custom SSL
- `wallet.select_unspent_coins(peer, amount, fee, omit)` - Select coins
- `Wallet::is_coin_spendable(peer, coin_id)` - Check coin status

#### Address Utilities
- `Wallet::address_to_puzzle_hash(address)` - Decode address
- `Wallet::puzzle_hash_to_address(hash, prefix)` - Encode address

## 🔐 Security Features

### Encryption
- **AES-256-GCM**: Industry-standard encryption for mnemonic storage
- **Random Salts**: Each encryption uses unique random salt
- **Secure Nonces**: Cryptographically secure random nonces

### Key Management
- **BIP39 Compliance**: Standard mnemonic generation and validation
- **Deterministic Keys**: Same mnemonic always produces same keys
- **Memory Safety**: Rust prevents buffer overflows and memory leaks

### Network Security
- **SSL/TLS**: Encrypted peer connections using Chia SSL certificates
- **Signature Verification**: BLS signature validation for authenticity

## 🏗️ Architecture

### Dependencies
- **DataLayer-Driver v0.1.50**: Core Chia blockchain integration
- **bip39**: Mnemonic generation and validation
- **aes-gcm**: AES-256-GCM encryption
- **tokio**: Async runtime for network operations
- **serde**: Serialization for data persistence

### File Structure
```
src/
├── lib.rs          # Public API exports
├── wallet.rs       # Core wallet implementation
├── error.rs        # Error types and handling
└── file_cache.rs   # Generic file caching system

tests/
└── integration_tests.rs  # Comprehensive integration tests

examples/
└── wallet_usage.rs       # Usage examples
```

## 🆚 Comparison with TypeScript Version

| Feature | TypeScript | Rust | Status |
|---------|------------|------|--------|
| Wallet Management | ✅ | ✅ | **Complete** |
| Cryptographic Operations | ✅ | ✅ | **Complete** |
| Peer Connection | ✅ | ✅ | **Complete** |
| Address Encoding | ✅ | ✅ | **Complete** |
| Coin Operations | ✅ | ✅ | **Complete** |
| Encrypted Storage | ✅ | ✅ | **Enhanced** |
| Error Handling | ✅ | ✅ | **Enhanced** |
| Memory Safety | ❌ | ✅ | **Rust Advantage** |
| Performance | Good | ✅ | **Rust Advantage** |
| Type Safety | Good | ✅ | **Rust Advantage** |

### **Improvements Over TypeScript**
- 🔒 **Better Security**: AES-256-GCM vs simpler encryption
- ⚡ **Higher Performance**: Native compiled code
- 🛡️ **Memory Safety**: No buffer overflows or memory leaks
- 🔍 **Type Safety**: Compile-time error prevention
- 🧪 **Better Testing**: Comprehensive test coverage

## 📈 Performance

- **Fast Compilation**: Optimized for development workflow
- **Efficient Runtime**: Zero-cost abstractions
- **Low Memory Usage**: Rust's ownership system
- **Concurrent Safe**: Built-in thread safety

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add comprehensive tests
4. Ensure all tests pass: `cargo test -- --test-threads=1`
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🔗 Related Projects

- [DataLayer-Driver](https://github.com/DIG-Network/DataLayer-Driver) - Core Chia blockchain integration
- [Chia Blockchain](https://github.com/Chia-Network/chia-blockchain) - Official Chia implementation

## 📞 Support

For issues and questions:
- Create an issue in the GitHub repository
- Check the test coverage documentation
- Review the example usage code

---

**Production Ready**: This implementation provides a complete, secure, and performant Rust wallet with full feature parity to the TypeScript version.
