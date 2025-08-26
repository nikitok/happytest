# HappyTest Reader Ansible Deployment

This Ansible configuration deploys the HappyTest reader to multiple servers and sets up hourly data collection via cron.

## Directory Structure

```
.ansible/
├── ansible.cfg           # Ansible configuration
├── inventory.ini         # Server inventory
├── deploy.yml           # Main deployment playbook
├── vault.yml            # Encrypted secrets (API keys)
├── README.md            # This file
└── roles/
    └── reader/          # Reader deployment role
        ├── tasks/       # Installation and setup tasks
        ├── templates/   # Configuration templates
        ├── handlers/    # Service handlers
        └── defaults/    # Default variables
```

## Prerequisites

1. Install Ansible on your control machine:
```bash
pip install ansible
```

2. Ensure SSH access to target servers with sudo privileges

3. Set up your Bybit API credentials

## Configuration

### 1. Update Inventory

Edit `inventory.ini` to add your servers:

```ini
[reader_servers]
server1 ansible_host=10.0.1.10 ansible_user=deploy
server2 ansible_host=10.0.1.11 ansible_user=deploy
```

### 2. Configure Secrets

Create and encrypt your vault file with API credentials:

```bash
# Create vault file
cat > vault.yml << EOF
---
vault_bybit_api_key: "your-actual-api-key"
vault_bybit_api_secret: "your-actual-api-secret"
EOF

# Encrypt the vault
ansible-vault encrypt vault.yml
```

### 3. Customize Settings

Edit `inventory.ini` to configure:
- Trading pairs to collect
- Collection interval (default: 3600 seconds)
- Data retention period
- Storage paths

## Deployment

### Deploy to All Servers

```bash
cd .ansible
ansible-playbook deploy.yml --ask-vault-pass
```

### Deploy to Specific Servers

```bash
ansible-playbook deploy.yml --limit server1 --ask-vault-pass
```

### Dry Run

```bash
ansible-playbook deploy.yml --check --ask-vault-pass
```

## Post-Deployment

### Check Status

SSH to any deployed server and run:

```bash
sudo -u happytest /opt/happytest/bin/reader.sh status
```

### Manual Collection

Trigger manual data collection:

```bash
sudo -u happytest /opt/happytest/bin/reader.sh collect
```

### View Logs

```bash
tail -f /var/log/happytest/reader_cron.log
```

### Check Cron Jobs

```bash
sudo crontab -u happytest -l
cat /etc/cron.d/happytest-reader
```

## Cron Schedule

The reader runs every hour at minute 5:
- `:05` - Collect orderbook data for all configured pairs

Daily cleanup runs at 2:00 AM:
- `02:00` - Remove data files older than retention period

## Data Storage

Collected data is stored in:
- Path: `/var/lib/happytest/data/`
- Format: `{SYMBOL}_{TIMESTAMP}_{DURATION}s_mainnet.jsonl`
- Optional: Parquet conversion for better compression

## Monitoring

### Check Recent Collections

```bash
ls -lht /var/lib/happytest/data/ | head -10
```

### Check Disk Usage

```bash
du -sh /var/lib/happytest/data/
```

### View Service Status (if using systemd)

```bash
systemctl status happytest-reader.timer
journalctl -u happytest-reader -f
```

## Troubleshooting

### Connection Issues

1. Check API credentials in vault
2. Verify network connectivity to Bybit API
3. Check logs for rate limiting

### Storage Issues

1. Check disk space: `df -h`
2. Verify permissions: `ls -la /var/lib/happytest/`
3. Check cleanup script: `/opt/happytest/bin/cleanup.sh`

### Cron Not Running

1. Check cron service: `systemctl status cron`
2. Verify cron file: `cat /etc/cron.d/happytest-reader`
3. Check user crontab: `crontab -u happytest -l`

## Maintenance

### Update Reader

```bash
# Update code and redeploy
ansible-playbook deploy.yml --tags update --ask-vault-pass
```

### Rotate API Keys

```bash
# Edit vault
ansible-vault edit vault.yml

# Redeploy configuration
ansible-playbook deploy.yml --tags config --ask-vault-pass
```

### Clean Old Data Manually

```bash
ansible all -m shell -a "/opt/happytest/bin/cleanup.sh" -b
```

## Security Notes

1. Always keep `vault.yml` encrypted
2. Use SSH keys for server access
3. Restrict API key permissions in Bybit
4. Monitor log files for suspicious activity
5. Keep servers and dependencies updated