#!/bin/bash
# Helper script for common Ansible operations

set -e

PLAYBOOK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

show_help() {
    echo "Usage: ./run.sh [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  deploy        Deploy reader to all servers"
    echo "  deploy-one    Deploy to a specific server"
    echo "  status        Check deployment status"
    echo "  collect       Trigger manual collection on all servers"
    echo "  logs          Show recent logs from all servers"
    echo "  vault-edit    Edit encrypted vault file"
    echo "  vault-create  Create new vault file"
    echo "  check         Run deployment in check mode (dry run)"
    echo ""
    echo "Examples:"
    echo "  ./run.sh deploy"
    echo "  ./run.sh deploy-one server1"
    echo "  ./run.sh status"
}

case "$1" in
    deploy)
        echo "Deploying to all servers..."
        ansible-playbook -i inventory.ini deploy.yml --ask-vault-pass
        ;;
    
    deploy-one)
        if [ -z "$2" ]; then
            echo "Error: Please specify a server name"
            echo "Usage: ./run.sh deploy-one <server_name>"
            exit 1
        fi
        echo "Deploying to $2..."
        ansible-playbook -i inventory.ini deploy.yml --limit "$2" --ask-vault-pass
        ;;
    
    status)
        echo "Checking deployment status..."
        ansible reader_servers -i inventory.ini -m shell \
            -a "/opt/happytest/bin/reader.sh status" \
            --become --become-user=happytest
        ;;
    
    collect)
        echo "Triggering manual collection on all servers..."
        ansible reader_servers -i inventory.ini -m shell \
            -a "/opt/happytest/bin/reader.sh collect" \
            --become --become-user=happytest
        ;;
    
    logs)
        echo "Showing recent logs from all servers..."
        ansible reader_servers -i inventory.ini -m shell \
            -a "tail -n 50 /var/log/happytest/reader_cron.log" \
            --become
        ;;
    
    vault-edit)
        ansible-vault edit vault.yml
        ;;
    
    vault-create)
        if [ -f "vault.yml" ]; then
            echo "vault.yml already exists. Use 'vault-edit' to modify it."
            exit 1
        fi
        cat > vault.yml << EOF
---
vault_bybit_api_key: "YOUR_API_KEY_HERE"
vault_bybit_api_secret: "YOUR_API_SECRET_HERE"
EOF
        ansible-vault encrypt vault.yml
        echo "Vault created and encrypted. Use 'vault-edit' to add your actual API credentials."
        ;;
    
    check)
        echo "Running deployment in check mode (dry run)..."
        ansible-playbook -i inventory.ini deploy.yml --check --ask-vault-pass
        ;;
    
    help|--help|-h)
        show_help
        ;;
    
    *)
        echo "Unknown command: $1"
        echo ""
        show_help
        exit 1
        ;;
esac