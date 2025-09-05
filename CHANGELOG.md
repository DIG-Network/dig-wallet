# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-09-05

### Added
- Initial release of Dig Wallet Rust
- Complete Rust implementation of Chia wallet functionality
- Integration with DataLayer-Driver v0.1.50
- Comprehensive wallet management features:
  - Wallet creation with secure BIP39 mnemonic generation
  - Wallet import from existing mnemonics
  - Multiple wallet support with encrypted storage
  - Wallet deletion and listing functionality
- Full cryptographic operations:
  - BIP39 compliant key derivation
  - BLS digital signatures (creation and verification)
  - AES-256-GCM encryption for secure storage
  - Deterministic key generation
- Blockchain integration:
  - Peer connection using `connect_random` functionality
  - Unspent coin selection and management
  - Coin spendability verification
  - Support for both mainnet and testnet11
- Address handling:
  - XCH address generation using bech32m encoding
  - Address to puzzle hash conversion
  - Multi-network address support (xch/txch prefixes)
- Advanced features:
  - Generic file-based caching system
  - Comprehensive error handling with detailed error types
  - Memory-safe implementation using Rust's ownership system
  - Concurrent operation support
- Testing and documentation:
  - 24 comprehensive tests (17 unit + 7 integration)
  - 100% test pass rate with full coverage
  - Complete API documentation with examples
  - Usage examples and integration guides

### Security Features
- AES-256-GCM encryption for mnemonic storage (improvement over TypeScript version)
- Secure random number generation for entropy and salts
- Memory safety through Rust's ownership system
- Type safety with compile-time error prevention
- Proper input validation and sanitization

### Performance Improvements
- Native compiled performance (vs interpreted JavaScript)
- Zero-cost abstractions
- Efficient memory usage
- Fast cryptographic operations

### Dependencies
- `datalayer-driver = "0.1.50"` - Core Chia blockchain integration
- `bip39 = "2.0"` - Mnemonic generation and validation
- `aes-gcm = "0.10"` - AES-256-GCM encryption
- `tokio = "1.0"` - Async runtime for network operations
- `serde = "1.0"` - Serialization for data persistence
- `dirs = "5.0"` - Cross-platform directory detection
- `hex = "0.4"` - Hexadecimal encoding/decoding
- `rand = "0.8"` - Cryptographically secure random number generation
- `base64 = "0.21"` - Base64 encoding for encrypted data
- `thiserror = "1.0"` - Error handling macros

### Documentation
- Complete README with usage examples
- Comprehensive test coverage documentation
- API reference with code examples
- Implementation analysis comparing TypeScript and Rust versions

[Unreleased]: https://github.com/DIG-Network/digwallet-rust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/DIG-Network/digwallet-rust/releases/tag/v0.1.0
