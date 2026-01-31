#!/bin/bash
set -e

# Colors and formatting
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m' # No Color

# Symbols
CHECK="${GREEN}✓${NC}"
CROSS="${RED}✗${NC}"
ARROW="${CYAN}→${NC}"
SPINNER_CHARS="⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"

# Print functions
print_header() {
    echo ""
    echo -e "${MAGENTA}${BOLD}╔════════════════════════════════════════════╗${NC}"
    echo -e "${MAGENTA}${BOLD}║           NextLine Setup Wizard            ║${NC}"
    echo -e "${MAGENTA}${BOLD}╚════════════════════════════════════════════╝${NC}"
    echo ""
}

print_step() {
    echo -e "\n${BLUE}${BOLD}[$1/7]${NC} ${BOLD}$2${NC}"
}

print_success() {
    echo -e "  ${CHECK} $1"
}

print_error() {
    echo -e "  ${CROSS} ${RED}$1${NC}"
}

print_info() {
    echo -e "  ${ARROW} ${DIM}$1${NC}"
}

print_warning() {
    echo -e "  ${YELLOW}⚠${NC}  ${YELLOW}$1${NC}"
}

# Spinner function
spinner() {
    local pid=$1
    local msg=$2
    local i=0
    while kill -0 $pid 2>/dev/null; do
        printf "\r  ${CYAN}${SPINNER_CHARS:$i:1}${NC} ${DIM}$msg${NC}"
        i=$(( (i + 1) % 10 ))
        sleep 0.1
    done
    printf "\r"
}

# Check if command exists
check_command() {
    if command -v $1 &> /dev/null; then
        print_success "$1 found"
        return 0
    else
        print_error "$1 not found"
        return 1
    fi
}

# Ask yes/no question
ask_yes_no() {
    local prompt="$1"
    local default="${2:-y}"

    if [[ "$default" == "y" ]]; then
        prompt="$prompt [Y/n] "
    else
        prompt="$prompt [y/N] "
    fi

    echo -ne "  ${ARROW} ${prompt}"
    read -r answer
    answer=${answer:-$default}

    [[ "$answer" =~ ^[Yy] ]]
}

# Main setup
main() {
    print_header

    echo -e "${DIM}This wizard will:${NC}"
    echo -e "  • Build the NextLine desktop app"
    echo -e "  • Deploy your personal sync server to Cloudflare"
    echo -e "  • Configure everything automatically"
    echo ""

    if ! ask_yes_no "Ready to begin?"; then
        echo -e "\n${DIM}Setup cancelled.${NC}"
        exit 0
    fi

    # Step 1: Check dependencies
    print_step 1 "Checking dependencies"

    local missing=0
    check_command cargo || missing=1
    check_command npm || missing=1
    check_command npx || missing=1

    if [[ $missing -eq 1 ]]; then
        echo ""
        print_error "Missing dependencies. Please install:"
        echo -e "  ${DIM}• Rust: https://rustup.rs${NC}"
        echo -e "  ${DIM}• Node.js: https://nodejs.org${NC}"
        exit 1
    fi

    # Check for GTK4 dev libraries
    if ! pkg-config --exists gtk4 2>/dev/null; then
        print_warning "GTK4 development libraries may be missing"
        print_info "On Ubuntu/Debian: sudo apt install libgtk-4-dev libadwaita-1-dev"
        print_info "On Fedora: sudo dnf install gtk4-devel libadwaita-devel"
        if ! ask_yes_no "Continue anyway?"; then
            exit 1
        fi
    else
        print_success "GTK4 development libraries found"
    fi

    # Step 2: Build the app
    print_step 2 "Building NextLine"
    print_info "This may take a few minutes on first build..."

    if cargo build --release > /tmp/nextline-build.log 2>&1; then
        print_success "Build complete"
    else
        print_error "Build failed. Check /tmp/nextline-build.log"
        exit 1
    fi

    # Step 3: Setup Cloudflare
    print_step 3 "Setting up Cloudflare"

    # Install worker dependencies
    print_info "Installing worker dependencies..."
    cd nextline-worker
    npm install --silent > /dev/null 2>&1
    print_success "Dependencies installed"

    # Check if already logged in
    if ! npx wrangler whoami > /dev/null 2>&1; then
        echo ""
        print_info "Opening browser for Cloudflare login..."
        print_info "Please authorize the application in your browser"
        echo ""

        if ! npx wrangler login; then
            print_error "Cloudflare login failed"
            exit 1
        fi
    fi
    print_success "Logged into Cloudflare"

    # Get account info
    ACCOUNT_INFO=$(npx wrangler whoami 2>/dev/null | grep -E "account|Account" | head -1 || echo "")
    if [[ -n "$ACCOUNT_INFO" ]]; then
        print_info "$ACCOUNT_INFO"
    fi

    # Step 4: Create R2 bucket
    print_step 4 "Creating R2 storage bucket"

    if npx wrangler r2 bucket list 2>/dev/null | grep -q "nextline-sync"; then
        print_success "Bucket 'nextline-sync' already exists"
    else
        if npx wrangler r2 bucket create nextline-sync > /dev/null 2>&1; then
            print_success "Created bucket 'nextline-sync'"
        else
            print_warning "Bucket may already exist, continuing..."
        fi
    fi

    # Step 5: Deploy worker
    print_step 5 "Deploying sync worker"

    DEPLOY_OUTPUT=$(npx wrangler deploy 2>&1)
    WORKER_URL=$(echo "$DEPLOY_OUTPUT" | grep -oE 'https://[a-zA-Z0-9.-]+\.workers\.dev' | head -1)

    if [[ -z "$WORKER_URL" ]]; then
        print_error "Failed to deploy worker"
        echo "$DEPLOY_OUTPUT"
        exit 1
    fi
    print_success "Worker deployed"
    print_info "URL: ${CYAN}$WORKER_URL${NC}"

    # Step 6: Generate and set API key
    print_step 6 "Configuring authentication"

    API_KEY=$(openssl rand -hex 32)
    print_success "Generated secure API key"

    echo "$API_KEY" | npx wrangler secret put API_KEY > /dev/null 2>&1
    print_success "API key stored in Cloudflare"

    # Create local config
    mkdir -p ~/.config/nextline
    cat > ~/.config/nextline/config.json << EOF
{
  "sync": {
    "enabled": true,
    "url": "$WORKER_URL",
    "api_key": "$API_KEY"
  }
}
EOF
    print_success "Local config saved to ~/.config/nextline/config.json"

    cd ..

    # Step 7: Install application
    print_step 7 "Installing application"

    # Create local bin directory
    mkdir -p ~/.local/bin

    # Copy binary
    cp target/release/nextline ~/.local/bin/
    print_success "Binary installed to ~/.local/bin/nextline"

    # Create desktop file
    mkdir -p ~/.local/share/applications
    cat > ~/.local/share/applications/nextline.desktop << EOF
[Desktop Entry]
Name=NextLine
Comment=Simple task management
Exec=$HOME/.local/bin/nextline
Icon=checkbox-checked-symbolic
Terminal=false
Type=Application
Categories=Utility;GTK;
Keywords=tasks;todo;productivity;
EOF
    print_success "Desktop entry created"

    # Update desktop database
    if command -v update-desktop-database &> /dev/null; then
        update-desktop-database ~/.local/share/applications 2>/dev/null || true
    fi

    # Check PATH
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        print_warning "~/.local/bin is not in your PATH"
        print_info "Add this to your ~/.bashrc or ~/.zshrc:"
        echo -e "    ${DIM}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
    fi

    # Print summary
    echo ""
    echo -e "${GREEN}${BOLD}╔════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}${BOLD}║            Setup Complete! 🎉              ║${NC}"
    echo -e "${GREEN}${BOLD}╚════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${BOLD}Your sync server:${NC}"
    echo -e "  ${CYAN}$WORKER_URL${NC}"
    echo ""
    echo -e "${BOLD}Cloudflare Dashboard:${NC}"
    echo -e "  ${CYAN}https://dash.cloudflare.com/${NC}"
    echo -e "  ${DIM}(View Workers & R2 storage there)${NC}"
    echo ""
    echo -e "${BOLD}To run NextLine:${NC}"
    echo -e "  ${DIM}•${NC} Search for 'NextLine' in your app launcher"
    echo -e "  ${DIM}•${NC} Or run: ${CYAN}nextline${NC}"
    echo ""
    echo -e "${BOLD}To sync on another device:${NC}"
    echo -e "  Create ${CYAN}~/.config/nextline/config.json${NC} with:"
    echo -e "  ${DIM}{"
    echo -e "    \"sync\": {"
    echo -e "      \"enabled\": true,"
    echo -e "      \"url\": \"$WORKER_URL\","
    echo -e "      \"api_key\": \"$API_KEY\""
    echo -e "    }"
    echo -e "  }${NC}"
    echo ""
    echo -e "${DIM}Your API key is stored securely in Cloudflare and locally.${NC}"
    echo -e "${DIM}Free tier: 100k requests/day, 10GB storage.${NC}"
    echo ""
}

# Run main
main "$@"
