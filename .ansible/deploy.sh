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

# Build the project first
#echo -e "${YELLOW}Building the project...${NC}"
#cd "$PROJECT_DIR"
#cargo build --release
#
#if [ $? -ne 0 ]; then
#    echo -e "${RED}Build failed! Please fix build errors before deploying.${NC}"
#    exit 1
#fi
#
#echo -e "${GREEN}Build successful!${NC}"

# Check if x86_64-apple-darwin binaries exist
if [ ! -f "$PROJECT_DIR/target/x86_64-apple-darwin/release/reader" ]; then
    echo -e "${RED}Error: reader binary not found at $PROJECT_DIR/target/x86_64-apple-darwin/release/reader${NC}"
    echo -e "${YELLOW}Please run: cargo build --release --target x86_64-apple-darwin${NC}"
    exit 1
fi

if [ ! -f "$PROJECT_DIR/target/x86_64-apple-darwin/release/happytest" ]; then
    echo -e "${RED}Error: happytest binary not found at $PROJECT_DIR/target/x86_64-apple-darwin/release/happytest${NC}"
    echo -e "${YELLOW}Please run: cargo build --release --target x86_64-apple-darwin${NC}"
    exit 1
fi

echo -e "${GREEN}Found x86_64-apple-darwin binaries:${NC}"
echo -e "  - reader: $PROJECT_DIR/target/x86_64-apple-darwin/release/reader"
echo -e "  - happytest: $PROJECT_DIR/target/x86_64-apple-darwin/release/happytest"

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