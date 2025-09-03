#!/bin/bash

# Deploy happytest reader to remote hosts

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting HappyTest Reader Deployment${NC}"
echo "----------------------------------------"

# Check if inventory file exists
if [ ! -f "$SCRIPT_DIR/inventory.ini" ]; then
    echo -e "${RED}Error: inventory.ini not found in $SCRIPT_DIR${NC}"
    exit 1
fi

# Build the project for both architectures
echo -e "${YELLOW}Building the project for both architectures...${NC}"
cd "$PROJECT_DIR"

# Build for x86_64
echo -e "${YELLOW}Building for x86_64-apple-darwin...${NC}"
cargo build --release --target x86_64-apple-darwin
if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed for x86_64-apple-darwin! Please fix build errors before deploying.${NC}"
    exit 1
fi
echo -e "${GREEN}x86_64 build successful!${NC}"

# Build for aarch64
echo -e "${YELLOW}Building for aarch64-apple-darwin...${NC}"
cargo build --release --target aarch64-apple-darwin
if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed for aarch64-apple-darwin! Please fix build errors before deploying.${NC}"
    exit 1
fi
echo -e "${GREEN}aarch64 build successful!${NC}"

# Get target architecture from inventory
TARGET_ARCH=$(grep "target_architecture=" "$SCRIPT_DIR/inventory.ini" | cut -d'=' -f2)
if [ -z "$TARGET_ARCH" ]; then
    echo -e "${YELLOW}Warning: target_architecture not specified in inventory.ini, defaulting to x86_64${NC}"
    TARGET_ARCH="x86_64"
fi

# Set binary path based on architecture
if [ "$TARGET_ARCH" = "aarch64" ]; then
    BINARY_PATH="$PROJECT_DIR/target/aarch64-apple-darwin/release"
    ARCH_DISPLAY="aarch64-apple-darwin"
else
    BINARY_PATH="$PROJECT_DIR/target/x86_64-apple-darwin/release"
    ARCH_DISPLAY="x86_64-apple-darwin"
fi

# Check if binaries exist for selected architecture
if [ ! -f "$BINARY_PATH/reader" ]; then
    echo -e "${RED}Error: reader binary not found at $BINARY_PATH/reader${NC}"
    exit 1
fi

if [ ! -f "$BINARY_PATH/happytest" ]; then
    echo -e "${RED}Error: happytest binary not found at $BINARY_PATH/happytest${NC}"
    exit 1
fi

echo -e "${GREEN}Using $ARCH_DISPLAY binaries:${NC}"
echo -e "  - reader: $BINARY_PATH/reader"
echo -e "  - happytest: $BINARY_PATH/happytest"

# Run ansible playbook
echo -e "${YELLOW}Running Ansible deployment...${NC}"
cd "$SCRIPT_DIR"

ansible-playbook \
    -i inventory.ini \
    deploy.yml \
    --ask-become-pass

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Deployment completed successfully!${NC}"
else
    echo -e "${RED}Deployment failed!${NC}"
    exit 1
fi