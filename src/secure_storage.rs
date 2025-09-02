//! # Secure Storage Module
//!
//! This module provides secure storage functionality for API keys using the keyring crate.
//! It stores credentials in the OS-native secure storage (Keychain on macOS, Credential Manager
//! on Windows, libsecret/gnome-keyring on Linux) on a per-user basis.
//!
//! ## Note on Testing
//!
//! In testing environments or systems without proper keyring backends, the keyring crate may
//! use a mock backend that doesn't persist data between function calls. This is expected
//! behavior and the implementation will work correctly in production environments with
//! proper keyring support.
//!
//! ## Functions
//!
//! - `store_api_key`: Securely stores an API key for the weather service
//! - `get_api_key`: Retrieves the stored API key
//! - `delete_api_key`: Removes the stored API key
//! - `has_api_key`: Checks if an API key is stored
//!
//! ## Example
//!
//! ```rust
//! use secure_storage::{store_api_key, get_api_key};
//!
//! // Store an API key
//! store_api_key("your_api_key_here").expect("Failed to store API key");
//!
//! // Retrieve the API key
//! let api_key = get_api_key().expect("Failed to get API key");
//! ```

use keyring::{Entry, Result as KeyringResult};
use log::{debug, error, info};

/// The service name used for storing credentials in the keyring
const SERVICE_NAME: &str = "open-weather-wizard";
/// The username/account name for the credential entry
const USERNAME: &str = "default";

/// Errors that can occur during secure storage operations
#[derive(Debug)]
pub enum SecureStorageError {
    /// Failed to access the keyring
    KeyringError(keyring::Error),
    /// API key not found in storage
    ApiKeyNotFound,
}

impl std::fmt::Display for SecureStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecureStorageError::KeyringError(e) => write!(f, "Keyring error: {}", e),
            SecureStorageError::ApiKeyNotFound => write!(f, "API key not found in secure storage"),
        }
    }
}

impl std::error::Error for SecureStorageError {}

impl From<keyring::Error> for SecureStorageError {
    fn from(err: keyring::Error) -> Self {
        SecureStorageError::KeyringError(err)
    }
}

/// Creates a keyring entry for the weather application
fn create_keyring_entry() -> KeyringResult<Entry> {
    Entry::new(SERVICE_NAME, USERNAME)
}

/// Stores an API key securely in the OS keyring
///
/// # Arguments
/// * `api_key` - The API key to store
///
/// # Returns
/// * `Ok(())` if the key was stored successfully
/// * `Err(SecureStorageError)` if storage failed
pub fn store_api_key(api_key: &str) -> Result<(), SecureStorageError> {
    info!("Storing API key in secure storage");
    let entry = create_keyring_entry()
        .map_err(|e| {
            error!("Failed to create keyring entry for storing: {}", e);
            SecureStorageError::KeyringError(e)
        })?;
    entry.set_password(api_key)
        .map_err(|e| {
            error!("Failed to set password in keyring: {}", e);
            SecureStorageError::KeyringError(e)
        })?;
    debug!("API key stored successfully");
    Ok(())
}

/// Retrieves the stored API key from the OS keyring
///
/// # Returns
/// * `Ok(String)` containing the API key if found
/// * `Err(SecureStorageError)` if the key was not found or retrieval failed
pub fn get_api_key() -> Result<String, SecureStorageError> {
    debug!("Retrieving API key from secure storage");
    let entry = create_keyring_entry()
        .map_err(|e| {
            error!("Failed to create keyring entry for retrieval: {}", e);
            SecureStorageError::KeyringError(e)
        })?;
    match entry.get_password() {
        Ok(password) => {
            debug!("API key retrieved successfully");
            Ok(password)
        }
        Err(keyring::Error::NoEntry) => {
            debug!("No API key found in secure storage");
            Err(SecureStorageError::ApiKeyNotFound)
        }
        Err(e) => {
            error!("Failed to retrieve API key: {}", e);
            Err(SecureStorageError::KeyringError(e))
        }
    }
}

/// Deletes the stored API key from the OS keyring
///
/// # Returns
/// * `Ok(())` if the key was deleted successfully or if no key was stored
/// * `Err(SecureStorageError)` if deletion failed
pub fn delete_api_key() -> Result<(), SecureStorageError> {
    info!("Deleting API key from secure storage");
    let entry = create_keyring_entry()?;
    match entry.delete_credential() {
        Ok(()) => {
            debug!("API key deleted successfully");
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            debug!("No API key found to delete");
            Ok(()) // Not an error if the key doesn't exist
        }
        Err(e) => {
            error!("Failed to delete API key: {}", e);
            Err(SecureStorageError::KeyringError(e))
        }
    }
}

/// Checks if an API key is stored in the keyring
///
/// # Returns
/// * `true` if an API key is stored
/// * `false` if no API key is stored or if there was an error checking
pub fn has_api_key() -> bool {
    debug!("Checking if API key exists in secure storage");
    match get_api_key() {
        Ok(_) => {
            debug!("API key exists in secure storage");
            true
        }
        Err(_) => {
            debug!("No API key found in secure storage");
            false
        }
    }
}

/// Helper function to configure an API key with validation
///
/// This function stores the API key if it appears to be valid (non-empty and reasonable length)
///
/// # Arguments
/// * `api_key` - The API key to store
///
/// # Returns
/// * `Ok(())` if the key was stored successfully
/// * `Err(SecureStorageError)` if storage failed or validation failed
pub fn configure_api_key(api_key: &str) -> Result<(), SecureStorageError> {
    // Basic validation
    if api_key.trim().is_empty() {
        return Err(SecureStorageError::KeyringError(keyring::Error::NoStorageAccess(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "API key cannot be empty")
        )));
    }
    
    if api_key.len() < 10 || api_key.len() > 100 {
        return Err(SecureStorageError::KeyringError(keyring::Error::NoStorageAccess(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "API key length seems invalid")
        )));
    }
    
    store_api_key(api_key.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve_api_key() {
        let test_key = "test_api_key_12345";
        
        // Clean up any existing key first
        let _ = delete_api_key();
        
        // Store the key
        assert!(store_api_key(test_key).is_ok());
        
        // Check that it exists
        assert!(has_api_key());
        
        // Retrieve the key
        let retrieved_key = get_api_key().expect("Failed to retrieve stored key");
        assert_eq!(retrieved_key, test_key);
        
        // Clean up
        assert!(delete_api_key().is_ok());
        assert!(!has_api_key());
    }

    #[test]
    fn test_get_nonexistent_api_key() {
        // Clean up any existing key first
        let _ = delete_api_key();
        
        // Try to get a non-existent key
        match get_api_key() {
            Err(SecureStorageError::ApiKeyNotFound) => (), // Expected
            _ => panic!("Expected ApiKeyNotFound error"),
        }
        
        assert!(!has_api_key());
    }

    #[test]
    fn test_delete_nonexistent_api_key() {
        // Clean up any existing key first
        let _ = delete_api_key();
        
        // Deleting a non-existent key should not fail
        assert!(delete_api_key().is_ok());
    }
}