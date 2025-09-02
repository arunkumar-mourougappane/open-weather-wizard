//! # API Key Configuration Utility
//!
//! This utility helps users configure their OpenWeatherMap API key securely.
//! It stores the API key in the OS-native secure storage for the Weather Wizard application.
//!
//! ## Usage
//!
//! ```bash
//! # Set an API key
//! cargo run --bin configure-api-key set <your-api-key>
//!
//! # Check if an API key is configured
//! cargo run --bin configure-api-key check
//!
//! # Remove the stored API key
//! cargo run --bin configure-api-key remove
//! ```

use std::env;
use std::process;

use open_wearther_wizard::secure_storage;

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "set" => {
            if args.len() != 3 {
                eprintln!("Error: Please provide an API key");
                print_usage();
                process::exit(1);
            }
            set_api_key(&args[2]);
        }
        "check" => {
            check_api_key();
        }
        "remove" => {
            remove_api_key();
        }
        _ => {
            eprintln!("Error: Unknown command '{}'", args[1]);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("OpenWeatherMap API Key Configuration Utility");
    println!();
    println!("USAGE:");
    println!("    configure-api-key <COMMAND> [ARGS]");
    println!();
    println!("COMMANDS:");
    println!("    set <API_KEY>    Store an API key securely");
    println!("    check            Check if an API key is configured");
    println!("    remove           Remove the stored API key");
    println!();
    println!("EXAMPLES:");
    println!("    configure-api-key set a836db2d273c0b50a2376d6a31750064");
    println!("    configure-api-key check");
    println!("    configure-api-key remove");
}

fn set_api_key(api_key: &str) {
    println!("Configuring OpenWeatherMap API key...");
    
    match secure_storage::configure_api_key(api_key) {
        Ok(()) => {
            println!("✓ API key stored successfully in secure storage");
            println!("You can now use the Weather Wizard application");
        }
        Err(e) => {
            eprintln!("✗ Failed to store API key: {}", e);
            eprintln!("Please check that your API key is valid and try again");
            process::exit(1);
        }
    }
}

fn check_api_key() {
    println!("Checking API key configuration...");
    
    if secure_storage::has_api_key() {
        match secure_storage::get_api_key() {
            Ok(key) => {
                let masked_key = mask_api_key(&key);
                println!("✓ API key is configured: {}", masked_key);
            }
            Err(e) => {
                eprintln!("✗ Error retrieving API key: {}", e);
                process::exit(1);
            }
        }
    } else {
        println!("✗ No API key is configured");
        println!("Use 'configure-api-key set <your-api-key>' to set one");
        process::exit(1);
    }
}

fn remove_api_key() {
    println!("Removing API key from secure storage...");
    
    match secure_storage::delete_api_key() {
        Ok(()) => {
            println!("✓ API key removed successfully");
        }
        Err(e) => {
            eprintln!("✗ Failed to remove API key: {}", e);
            process::exit(1);
        }
    }
}

fn mask_api_key(api_key: &str) -> String {
    if api_key.len() <= 8 {
        "*".repeat(api_key.len())
    } else {
        let start = &api_key[..4];
        let end = &api_key[api_key.len()-4..];
        format!("{}****{}", start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key("a836db2d273c0b50a2376d6a31750064"), "a836****0064");
        assert_eq!(mask_api_key("short"), "*****");
        assert_eq!(mask_api_key("12345678"), "1234****5678");
    }
}