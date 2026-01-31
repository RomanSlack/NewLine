#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

CHECK="${GREEN}✓${NC}"
CROSS="${RED}✗${NC}"
ARROW="${CYAN}→${NC}"

print_header() {
    echo ""
    echo -e "${RED}${BOLD}╔════════════════════════════════════════════╗${NC}"
    echo -e "${RED}${BOLD}║         NextLine Teardown Script           ║${NC}"
    echo -e "${RED}${BOLD}╚════════════════════════════════════════════╝${NC}"
    echo ""
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

ask_yes_no() {
    local prompt="$1"
    local default="${2:-n}"

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

main() {
    print_header

    echo -e "${DIM}This will remove:${NC}"
    echo -e "  • Cloudflare Worker (sync server)"
    echo -e "  • R2 Storage Bucket (synced files in the cloud)"
    echo -e "  • Local config file"
    echo -e "  • Optionally: installed binary and desktop entry"
    echo ""
    echo -e "${RED}${BOLD}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}${BOLD}║  WARNING: All synced data in the cloud will be deleted! ║${NC}"
    echo -e "${RED}${BOLD}╚════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${YELLOW}Before continuing, make sure your local files are up to date:${NC}"
    echo -e "  ${DIM}Your tasks are stored locally in: ~/Documents/NextLine/${NC}"
    echo -e "  ${DIM}These local files will NOT be deleted.${NC}"
    echo ""
    echo -e "${YELLOW}If you have changes on other devices that aren't synced here,${NC}"
    echo -e "${YELLOW}sync this device first or those changes will be lost!${NC}"
    echo ""

    if ! ask_yes_no "Have you backed up / synced your data locally?"; then
        echo -e "\n${DIM}Please sync your data first, then run this script again.${NC}"
        exit 0
    fi

    echo ""

    if ! ask_yes_no "Are you sure you want to delete the cloud sync server?"; then
        echo -e "\n${DIM}Teardown cancelled.${NC}"
        exit 0
    fi

    echo ""

    # Check if wrangler is available
    if ! command -v npx &> /dev/null; then
        print_error "npx not found. Cannot remove Cloudflare resources."
        print_info "You can manually delete resources at https://dash.cloudflare.com"
    else
        SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
        cd "$SCRIPT_DIR/nextline-worker" 2>/dev/null || true

        # Delete worker first
        echo -e "${BOLD}Removing Cloudflare Worker...${NC}"
        if npx wrangler delete --name nextline-sync --force 2>/dev/null; then
            print_success "Worker 'nextline-sync' deleted"
        else
            print_warning "Worker may not exist or already deleted"
        fi

        # Delete R2 bucket
        echo -e "\n${BOLD}Removing R2 Bucket...${NC}"

        # First check if bucket exists
        BUCKET_LIST=$(npx wrangler r2 bucket list 2>&1 || true)
        if ! echo "$BUCKET_LIST" | grep -q "nextline-sync"; then
            print_info "Bucket 'nextline-sync' does not exist (already deleted)"
        else
            # Try to delete the bucket directly first
            DELETE_OUTPUT=$(npx wrangler r2 bucket delete nextline-sync 2>&1 || true)
            DELETE_EXIT=$?
            if [[ $DELETE_EXIT -eq 0 ]] && ! echo "$DELETE_OUTPUT" | grep -qi "error"; then
                print_success "Bucket 'nextline-sync' deleted"
            elif echo "$DELETE_OUTPUT" | grep -qi "not empty\|contains objects"; then
                print_info "Bucket is not empty, deploying cleanup worker..."

                # Create temporary cleanup worker
                CLEANUP_DIR=$(mktemp -d)
                cat > "$CLEANUP_DIR/index.js" << 'JSEOF'
export default {
  async fetch(request, env) {
    let deleted = 0;
    let cursor = undefined;
    do {
      const listed = await env.BUCKET.list({ cursor });
      for (const obj of listed.objects) {
        await env.BUCKET.delete(obj.key);
        deleted++;
      }
      cursor = listed.truncated ? listed.cursor : undefined;
    } while (cursor);
    return new Response(JSON.stringify({ deleted }), {
      headers: { 'Content-Type': 'application/json' }
    });
  }
}
JSEOF
                cat > "$CLEANUP_DIR/wrangler.toml" << 'TOMLEOF'
name = "nextline-cleanup-temp"
main = "index.js"
compatibility_date = "2024-01-01"

[[r2_buckets]]
binding = "BUCKET"
bucket_name = "nextline-sync"
TOMLEOF

                # Deploy cleanup worker
                DEPLOY_OUTPUT=$(cd "$CLEANUP_DIR" && npx wrangler deploy 2>&1 || true)
                CLEANUP_URL=$(echo "$DEPLOY_OUTPUT" | grep -oE 'https://[a-zA-Z0-9.-]+\.workers\.dev' | head -1 || true)

                if [[ -n "$CLEANUP_URL" ]]; then
                    print_info "Emptying bucket contents..."
                    # Call the worker to delete all objects
                    RESULT=$(curl -s --max-time 30 "$CLEANUP_URL" 2>/dev/null || echo "{}")
                    DELETED=$(echo "$RESULT" | grep -oE '"deleted":[0-9]+' | grep -oE '[0-9]+' || echo "0")
                    print_success "Deleted $DELETED objects from bucket"

                    # Wait for deletions to propagate
                    sleep 2

                    # Delete the cleanup worker
                    npx wrangler delete --name nextline-cleanup-temp --force 2>/dev/null || true
                    print_success "Cleanup worker removed"

                    # Now delete the empty bucket
                    FINAL_DELETE=$(npx wrangler r2 bucket delete nextline-sync 2>&1 || true)
                    if echo "$FINAL_DELETE" | grep -qi "deleted\|success"; then
                        print_success "Bucket 'nextline-sync' deleted"
                    elif echo "$FINAL_DELETE" | grep -qi "not found\|does not exist"; then
                        print_success "Bucket 'nextline-sync' deleted"
                    else
                        print_warning "Could not delete bucket after emptying"
                        print_info "It may take a moment for deletions to propagate"
                        print_info "Try running teardown again, or delete manually at: https://dash.cloudflare.com"
                    fi
                else
                    print_warning "Could not deploy cleanup worker"
                    echo ""
                    echo -e "  ${YELLOW}To complete teardown manually:${NC}"
                    echo -e "  ${DIM}1. Go to: ${CYAN}https://dash.cloudflare.com${NC}"
                    echo -e "  ${DIM}2. Navigate to: R2 Object Storage${NC}"
                    echo -e "  ${DIM}3. Click on bucket: ${CYAN}nextline-sync${NC}"
                    echo -e "  ${DIM}4. Select all objects and delete them${NC}"
                    echo -e "  ${DIM}5. Go to Settings tab and delete the bucket${NC}"
                    echo ""
                fi

                # Clean up temp directory
                rm -rf "$CLEANUP_DIR"
            else
                print_warning "Could not delete bucket"
                print_info "Delete it manually at: https://dash.cloudflare.com"
                print_info "Go to R2 > nextline-sync > Settings > Delete bucket"
            fi
        fi

        cd - > /dev/null 2>&1 || true
    fi

    # Remove local config
    echo -e "\n${BOLD}Removing local configuration...${NC}"
    if [[ -f ~/.config/nextline/config.json ]]; then
        rm ~/.config/nextline/config.json
        print_success "Config file removed"
    else
        print_info "No config file found"
    fi

    # Remove sync state
    PROJECTS_DIR="$HOME/Documents/NextLine"
    if [[ -f "$PROJECTS_DIR/.sync_state.json" ]]; then
        rm "$PROJECTS_DIR/.sync_state.json"
        print_success "Sync state file removed"
    fi

    # Ask about uninstalling the app
    echo ""
    if ask_yes_no "Also uninstall the NextLine app?"; then
        echo -e "\n${BOLD}Uninstalling application...${NC}"

        if [[ -f ~/.local/bin/nextline ]]; then
            rm ~/.local/bin/nextline
            print_success "Binary removed from ~/.local/bin/"
        else
            print_info "Binary not found in ~/.local/bin/"
        fi

        if [[ -f ~/.local/share/applications/nextline.desktop ]]; then
            rm ~/.local/share/applications/nextline.desktop
            print_success "Desktop entry removed"

            # Update desktop database
            if command -v update-desktop-database &> /dev/null; then
                update-desktop-database ~/.local/share/applications 2>/dev/null || true
            fi
        else
            print_info "Desktop entry not found"
        fi
    fi

    # Summary
    echo ""
    echo -e "${GREEN}${BOLD}╔════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}${BOLD}║           Teardown Complete                ║${NC}"
    echo -e "${GREEN}${BOLD}╚════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${DIM}Cloud sync has been disabled.${NC}"
    echo -e "${DIM}Your local task files in ~/Documents/NextLine are safe.${NC}"
    echo ""
    echo -e "${DIM}To set up sync again, run: ${CYAN}./setup.sh${NC}"
    echo ""
}

main "$@"
