#!/bin/bash

# Demonstration script showing the secure API key storage workflow
# This script demonstrates the complete flow without requiring GTK dependencies

echo "=== OpenWeatherMap API Key Secure Storage Demo ==="
echo

# Step 1: Check if an API key is already configured
echo "Step 1: Checking current API key status..."
if command -v cargo >/dev/null 2>&1; then
    cd /home/runner/work/open-weather-wizard/open-weather-wizard
    
    # Try to check API key status (this will fail in mock environment but shows the interface)
    echo "$ cargo run --bin configure-api-key check"
    echo "(In a real environment, this would check your OS keyring)"
    echo "✗ No API key is configured"
    echo "Use 'configure-api-key set <your-api-key>' to set one"
    echo
    
    # Step 2: Set an API key
    echo "Step 2: Setting an API key..."
    echo "$ cargo run --bin configure-api-key set a836db2d273c0b50a2376d6a31750064"
    echo "(In a real environment, this would store the key in your OS keyring)"
    echo "Configuring OpenWeatherMap API key..."
    echo "✓ API key stored successfully in secure storage"
    echo "You can now use the Weather Wizard application"
    echo
    
    # Step 3: Verify the API key was stored
    echo "Step 3: Verifying API key storage..."
    echo "$ cargo run --bin configure-api-key check"
    echo "Checking API key configuration..."
    echo "✓ API key is configured: a836****0064"
    echo
    
    # Step 4: Show how the application would use it
    echo "Step 4: Application usage..."
    echo "When you run the Weather Wizard application:"
    echo "- It automatically retrieves the API key from secure storage"
    echo "- Makes weather API calls using the stored key"
    echo "- No API key is ever visible in the source code or configuration files"
    echo
    
    echo "=== Key Benefits ==="
    echo "✓ Secure: API keys stored in OS-native secure storage"
    echo "✓ Per-user: Each user has their own API key"
    echo "✓ Cross-platform: Works on Windows, macOS, and Linux"
    echo "✓ Easy management: Simple CLI for key management"
    echo "✓ No hardcoded secrets: API keys never in source code"
    
else
    echo "Cargo not found. This demo requires Rust and Cargo to be installed."
fi

echo
echo "=== Architecture Overview ==="
echo "secure_storage.rs     - Core keyring integration"
echo "configure-api-key.rs  - CLI utility for key management"
echo "openweather_api.rs    - Weather API with secure key retrieval"
echo "main.rs               - Application with error handling for missing keys"