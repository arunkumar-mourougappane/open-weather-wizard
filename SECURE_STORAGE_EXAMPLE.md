# Secure API Key Storage Example

This document demonstrates how the secure storage implementation works.

## Example Usage

### Setting up an API key

```bash
# Configure your OpenWeatherMap API key
cargo run --bin configure-api-key set a836db2d273c0b50a2376d6a31750064

# Output:
# Configuring OpenWeatherMap API key...
# ✓ API key stored successfully in secure storage
# You can now use the Weather Wizard application
```

### Checking API key status

```bash
# Check if an API key is configured
cargo run --bin configure-api-key check

# Output:
# Checking API key configuration...
# ✓ API key is configured: a836****0064
```

### Using the API in the application

When you run the main application, it will automatically:

1. Try to retrieve the API key from secure storage
2. If no key is found, show a helpful error message
3. If the key is found, use it to fetch weather data

### Error Handling

If no API key is configured, the application will show:

```
API key not configured. Please set up your OpenWeatherMap API key.
```

Users can then use the configuration utility to set up their key.

## Security Benefits

- **No hardcoded secrets**: API keys are never stored in source code
- **OS-native security**: Uses Keychain (macOS), Credential Manager (Windows), or libsecret (Linux)
- **Per-user storage**: Each user has their own securely stored API key
- **Easy management**: Simple command-line interface for key management

## Implementation Details

The secure storage module provides these key functions:

- `store_api_key(key)` - Securely store an API key
- `get_api_key()` - Retrieve the stored API key
- `has_api_key()` - Check if a key is stored
- `delete_api_key()` - Remove the stored key
- `configure_api_key(key)` - Store with validation

All storage operations use the OS-native secure storage backend automatically.