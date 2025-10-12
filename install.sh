#!/bin/bash

# Open Weather Wizard Installation Script
# This script installs the open-weather-wizard binary and sets up desktop integration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Rust/Cargo is installed
check_cargo() {
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed. Please install Rust first:"
        echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    print_success "Cargo found: $(cargo --version)"
}

# Install the binary
install_binary() {
    print_status "Installing open-weather-wizard binary..."
    
    if [ -d ".git" ]; then
        # Installing from source directory
        cargo install --path . --force
    else
        # Installing from crates.io (if published)
        cargo install open-weather-wizard --force
    fi
    
    print_success "Binary installed successfully!"
}

# Set up desktop integration
setup_desktop_integration() {
    print_status "Setting up desktop integration..."
    
    # Create directories
    mkdir -p ~/.local/share/applications
    mkdir -p ~/.local/share/icons/hicolor/256x256/apps
    mkdir -p ~/.local/share/icons/hicolor/scalable/apps
    
    # Copy desktop file
    if [ -f "open-weather-wizard.desktop" ]; then
        cp open-weather-wizard.desktop ~/.local/share/applications/
        print_success "Desktop entry installed"
    else
        print_warning "Desktop file not found, skipping desktop integration"
        return
    fi
    
    # Copy icon files
    if [ -f "assets/icon/icon.png" ]; then
        cp assets/icon/icon.png ~/.local/share/icons/hicolor/256x256/apps/open-weather-wizard.png
        print_success "PNG icon installed"
    else
        print_warning "PNG icon not found at assets/icon/icon.png"
    fi
    
    # Create SVG icon from assets if available
    if [ -f "assets/animated/clear-day.svg" ]; then
        cp assets/animated/clear-day.svg ~/.local/share/icons/hicolor/scalable/apps/open-weather-wizard.svg
        print_success "SVG icon installed"
    elif [ -f "assets/static/clear-day.svg" ]; then
        cp assets/static/clear-day.svg ~/.local/share/icons/hicolor/scalable/apps/open-weather-wizard.svg
        print_success "SVG icon installed (from static)"
    else
        print_warning "No SVG icon found in assets"
    fi
    
    # Update desktop database
    if command -v update-desktop-database &> /dev/null; then
        update-desktop-database ~/.local/share/applications
        print_success "Desktop database updated"
    fi
    
    # Update icon cache
    if command -v gtk-update-icon-cache &> /dev/null; then
        gtk-update-icon-cache ~/.local/share/icons/hicolor/ &> /dev/null || true
        print_success "Icon cache updated"
    fi
}

# Create uninstall script
create_uninstall_script() {
    print_status "Creating uninstall script..."
    
    cat > ~/.local/bin/open-weather-wizard-uninstall << 'EOF'
#!/bin/bash

# Open Weather Wizard Uninstall Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_status "Uninstalling open-weather-wizard..."

# Remove binary
cargo uninstall open-weather-wizard 2>/dev/null || true
print_success "Binary removed"

# Remove desktop integration
rm -f ~/.local/share/applications/open-weather-wizard.desktop
rm -f ~/.local/share/icons/hicolor/256x256/apps/open-weather-wizard.png
rm -f ~/.local/share/icons/hicolor/scalable/apps/open-weather-wizard.svg

# Update caches
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database ~/.local/share/applications 2>/dev/null || true
fi

if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache ~/.local/share/icons/hicolor/ &> /dev/null || true
fi

print_success "Desktop integration removed"

# Remove config (optional)
read -p "Remove configuration files? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf ~/.config/open-weather-wizard/
    rm -rf ~/.cache/open-weather-wizard/
    print_success "Configuration files removed"
fi

print_success "Open Weather Wizard uninstalled successfully!"

# Remove this uninstall script
rm -f ~/.local/bin/open-weather-wizard-uninstall
EOF

    chmod +x ~/.local/bin/open-weather-wizard-uninstall
    print_success "Uninstall script created at ~/.local/bin/open-weather-wizard-uninstall"
}

# Main installation process
main() {
    echo
    print_status "🌤️  Open Weather Wizard Installation Script"
    echo
    
    check_cargo
    install_binary
    setup_desktop_integration
    create_uninstall_script
    
    echo
    print_success "🎉 Installation completed successfully!"
    echo
    echo "You can now:"
    echo "  • Run 'open-weather-wizard' from the command line"
    echo "  • Find 'Open Weather Wizard' in your application menu"
    echo "  • Uninstall with 'open-weather-wizard-uninstall'"
    echo
    echo "Note: You may need to log out and back in for the desktop icon to appear."
    echo
}

# Check for help flag
if [[ "$1" == "--help" || "$1" == "-h" ]]; then
    echo "Open Weather Wizard Installation Script"
    echo
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "This script will:"
    echo "  1. Install the open-weather-wizard binary using cargo"
    echo "  2. Set up desktop integration (icons, .desktop file)"
    echo "  3. Create an uninstall script"
    echo
    echo "Options:"
    echo "  -h, --help    Show this help message"
    echo
    echo "Requirements:"
    echo "  • Rust/Cargo installed"
    echo "  • Linux desktop environment"
    echo
    exit 0
fi

# Run main installation
main "$@"