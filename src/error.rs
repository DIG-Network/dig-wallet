use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Mnemonic seed phrase is required")]
    MnemonicRequired,
    
    #[error("Provided mnemonic is invalid")]
    InvalidMnemonic,
    
    #[error("Mnemonic seed phrase is not loaded")]
    MnemonicNotLoaded,
    
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),
    
    #[error("Could not get fingerprint")]
    FingerprintError,
    
    #[error("Could not get private key")]
    PrivateKeyError,
    
    #[error("No unspent coins available")]
    NoUnspentCoins,
    
    #[error("File system error: {0}")]
    FileSystemError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Cryptographic error: {0}")]
    CryptoError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("DataLayer driver error: {0}")]
    DataLayerError(String),
}
