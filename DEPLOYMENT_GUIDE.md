# Deployment Guide

This comprehensive guide covers all deployment options for the Inflyte DJ Monitor on Azure.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Azure Storage Setup](#azure-storage-setup)
3. [Deployment Options](#deployment-options)
   - [Option A: GitHub Actions (Recommended)](#option-a-github-actions-recommended)
   - [Option B: Manual Azure Container Instances](#option-b-manual-azure-container-instances)
   - [Option C: Local Deployment](#option-c-local-deployment)
4. [Monitoring and Management](#monitoring-and-management)
5. [Cost Estimates](#cost-estimates)

---

## Prerequisites

* Azure account ([create a free account](https://azure.microsoft.com/free/))
* Azure CLI installed ([installation guide](https://docs.microsoft.com/cli/azure/install-azure-cli))
* For GitHub Actions: GitHub account and repository
* For Manual deployment: Docker installed locally

---

## Azure Storage Setup

### Step 1: Log into Azure

```bash
az login
```

If you have multiple subscriptions:

```bash
az account set --subscription "Your Subscription Name"
```

### Step 2: Create Resource Group

```bash
az group create \
  --name inflyte-monitor-rg \
  --location eastus
```

### Step 3: Create Storage Account

```bash
az storage account create \
  --name inflytemonitstg \
  --resource-group inflyte-monitor-rg \
  --location eastus \
  --sku Standard_LRS \
  --kind StorageV2
```

**Note:** Storage account names must be 3-24 characters, lowercase letters and numbers only, globally unique.

### Step 4: Create Blob Container

```bash
az storage container create \
  --name dj-monitor \
  --account-name inflytemonitstg \
  --account-key $(az storage account keys list \
    --account-name inflytemonitstg \
    --resource-group inflyte-monitor-rg \
    --query '[0].value' \
    --output tsv)
```

### Step 5: Get Storage Access Key

```bash
az storage account keys list \
  --account-name inflytemonitstg \
  --resource-group inflyte-monitor-rg \
  --query '[0].value' \
  --output tsv
```

Save this key - you'll need it later.

---

## Deployment Options

### Option A: GitHub Actions (Recommended)

GitHub Actions provides automatic CI/CD with OIDC-based authentication (most secure).

#### Prerequisites for GitHub Actions

* GitHub repository with this code
* Azure Container Registry
* Mailgun account configured

#### 1. Create Azure Container Registry

```bash
az acr create \
  --resource-group inflyte-monitor-rg \
  --name inflyteacr \
  --sku Basic \
  --location eastus

az acr update \
  --name inflyteacr \
  --admin-enabled true
```

#### 2. Set Up OIDC Authentication

Create an Azure App Registration with federated credentials:

```bash
# Create the app registration
az ad app create --display-name "github-actions-inflyte-monitor"

# Get the app ID (client ID)
APP_ID=$(az ad app list --display-name "github-actions-inflyte-monitor" --query "[0].appId" -o tsv)
echo "AZURE_CLIENT_ID: $APP_ID"

# Create a service principal
az ad sp create --id $APP_ID

# Get the service principal object ID
SP_OBJECT_ID=$(az ad sp show --id $APP_ID --query id -o tsv)

# Assign contributor role
az role assignment create \
  --role contributor \
  --assignee-object-id $SP_OBJECT_ID \
  --assignee-principal-type ServicePrincipal \
  --scope /subscriptions/$(az account show --query id --output tsv)/resourceGroups/inflyte-monitor-rg
```

Configure federated credential for GitHub Actions:

```bash
# Replace with your GitHub username and repo name
GITHUB_ORG="your-github-username"
REPO_NAME="inflyte-dj-monitor"

az ad app federated-credential create \
  --id $APP_ID \
  --parameters "{
    \"name\": \"github-actions-main\",
    \"issuer\": \"https://token.actions.githubusercontent.com\",
    \"subject\": \"repo:${GITHUB_ORG}/${REPO_NAME}:ref:refs/heads/main\",
    \"audiences\": [\"api://AzureADTokenExchange\"]
  }"
```

#### 3. Configure GitHub Secrets

Navigate to your GitHub repository → Settings → Secrets and variables → Actions

Add the following secrets:

| Secret Name | Value | How to Get |
|------------|-------|------------|
| `INFLYTE_URLS` | `https://inflyteapp.com/r/campaign1,https://inflyteapp.com/r/campaign2` | Your campaign URLs (comma-separated) |
| `AZURE_CLIENT_ID` | From `$APP_ID` above | The app registration client ID |
| `AZURE_TENANT_ID` | `az account show --query tenantId -o tsv` | Your Azure tenant ID |
| `AZURE_SUBSCRIPTION_ID` | `az account show --query id -o tsv` | Your subscription ID |
| `AZURE_STORAGE_ACCOUNT` | `inflytemonitstg` | Storage account name |
| `AZURE_STORAGE_CONTAINER` | `dj-monitor` | Container name |
| `AZURE_BLOB_NAME_PREFIX` | `dj_list` | Blob name prefix |
| `AZURE_STORAGE_ACCESS_KEY` | From Step 5 above | Storage access key |
| `ACR_PASSWORD` | `az acr credential show --name inflyteacr --query "passwords[0].value" -o tsv` | ACR password |
| `MAILGUN_DOMAIN` | Your Mailgun domain | From Mailgun dashboard |
| `MAILGUN_API_KEY` | Your Mailgun API key | From Mailgun dashboard |
| `RECIPIENT_EMAIL` | `you@example.com` | Email for notifications |
| `FROM_EMAIL` | `noreply@your-domain.mailgun.org` | Sender email |
| `CHECK_INTERVAL_MINUTES` | `60` | Check frequency in minutes |

#### 4. Deploy

Push your code to the `main` branch:

```bash
git add .
git commit -m "Deploy to Azure"
git push origin main
```

GitHub Actions will automatically:
1. Build the Docker image
2. Push to Azure Container Registry
3. Deploy to Azure Container Instances

Monitor the deployment in the **Actions** tab of your GitHub repository.

#### 5. Verify Deployment

```bash
# Check container status
az container show \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --query "{Name:name, State:instanceView.state}" \
  --output table

# View logs
az container logs \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor
```

---

### Option B: Manual Azure Container Instances

For manual deployment without GitHub Actions.

#### 1. Create Azure Container Registry

```bash
az acr create \
  --resource-group inflyte-monitor-rg \
  --name inflyteacr \
  --sku Basic \
  --location eastus

az acr update \
  --name inflyteacr \
  --admin-enabled true
```

#### 2. Build and Push Docker Image

```bash
# Navigate to project directory
cd /path/to/inflyte

# Build for linux/amd64 (required for Azure Container Instances)
docker build --platform linux/amd64 -t inflyte-monitor:latest .

# Get ACR login server
ACR_LOGIN_SERVER=$(az acr show --name inflyteacr --query loginServer --output tsv)

# Log in to ACR
az acr login --name inflyteacr

# Tag and push
docker tag inflyte-monitor:latest ${ACR_LOGIN_SERVER}/inflyte-monitor:latest
docker push ${ACR_LOGIN_SERVER}/inflyte-monitor:latest
```

#### 3. Get Credentials

```bash
# Get ACR password
ACR_PASSWORD=$(az acr credential show --name inflyteacr --query "passwords[0].value" --output tsv)

# Get storage key
STORAGE_KEY=$(az storage account keys list \
  --account-name inflytemonitstg \
  --resource-group inflyte-monitor-rg \
  --query '[0].value' \
  --output tsv)
```

#### 4. Deploy Container Instance

```bash
az container create \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --image ${ACR_LOGIN_SERVER}/inflyte-monitor:latest \
  --registry-login-server ${ACR_LOGIN_SERVER} \
  --registry-username inflyteacr \
  --registry-password ${ACR_PASSWORD} \
  --cpu 1 \
  --memory 0.5 \
  --restart-policy Always \
  --os-type Linux \
  --environment-variables \
    INFLYTE_URLS=https://inflyteapp.com/r/pmqtne,https://inflyteapp.com/r/campaign2 \
    AZURE_STORAGE_ACCOUNT=inflytemonitstg \
    AZURE_STORAGE_CONTAINER=dj-monitor \
    AZURE_BLOB_NAME_PREFIX=dj_list \
    MAILGUN_DOMAIN=your-domain.mailgun.org \
    RECIPIENT_EMAIL=you@example.com \
    FROM_EMAIL=noreply@inflyte.com \
    CHECK_INTERVAL_MINUTES=60 \
  --secure-environment-variables \
    AZURE_STORAGE_ACCESS_KEY=${STORAGE_KEY} \
    MAILGUN_API_KEY=your-mailgun-api-key
```

**Important:** Replace `your-domain.mailgun.org` , `you@example.com` , and `your-mailgun-api-key` with your actual values.

#### 5. Update Deployment

When you make code changes:

```bash
# Rebuild and push
docker build --platform linux/amd64 -t inflyte-monitor:latest .
docker tag inflyte-monitor:latest ${ACR_LOGIN_SERVER}/inflyte-monitor:latest
docker push ${ACR_LOGIN_SERVER}/inflyte-monitor:latest

# Delete old container
az container delete \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --yes

# Recreate with same command from step 4
```

---

### Option C: Local Deployment

Run on your local machine or server.

#### 1. Configure Environment

Create `.env` file:

```bash
# Azure Storage
AZURE_STORAGE_ACCOUNT=inflytemonitstg
AZURE_STORAGE_CONTAINER=dj-monitor
AZURE_BLOB_NAME_PREFIX=dj_list
AZURE_STORAGE_ACCESS_KEY=your-storage-access-key

# Mailgun
MAILGUN_API_KEY=your-mailgun-api-key
MAILGUN_DOMAIN=your-domain.mailgun.org
RECIPIENT_EMAIL=you@example.com
FROM_EMAIL=noreply@inflyte.com

# App Configuration
CHECK_INTERVAL_MINUTES=60
```

#### 2. Run with Cargo

```bash
cargo run --release -- --url https://inflyteapp.com/r/pmqtne,https://inflyteapp.com/r/campaign2
```

#### 3. Run as Background Service (Linux)

Create systemd service file `/etc/systemd/system/inflyte-monitor.service` :

```ini
[Unit]
Description=Inflyte DJ Monitor
After=network.target

[Service]
Type=simple
User=youruser
WorkingDirectory=/path/to/inflyte
EnvironmentFile=/path/to/inflyte/.env
ExecStart=/path/to/inflyte/target/release/inflyte --url https://inflyteapp.com/r/pmqtne,https://inflyteapp.com/r/campaign2
Restart=always

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable inflyte-monitor
sudo systemctl start inflyte-monitor
sudo systemctl status inflyte-monitor
```

---

## Monitoring and Management

### View Container Logs

```bash
# Follow logs in real-time
az container logs \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --follow
```

### Check Container Status

```bash
az container show \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --query "{Name:name, State:instanceView.state, RestartCount:containers[0].instanceView.restartCount}" \
  --output table
```

### Verify Storage

```bash
# List blob files
az storage blob list \
  --container-name dj-monitor \
  --account-name inflytemonitstg \
  --output table

# Download a blob
az storage blob download \
  --container-name dj-monitor \
  --name dj_list_pmqtne.json \
  --file dj_list_backup.json \
  --account-name inflytemonitstg
```

### Restart Container

```bash
az container restart \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor
```

### View GitHub Actions Workflows

Navigate to your repository → **Actions** tab to see:
* Build status
* Deployment logs
* Error messages

---

## Cost Estimates

### Monthly Costs (24/7 Operation)

| Service | Cost |
|---------|------|
| Azure Container Instances (1 vCPU, 0.5 GB RAM) | ~$3.30 |
| Azure Container Registry (Basic) | ~$3.00 |
| Azure Blob Storage | ~$0.01 |
| Mailgun (Free tier, <5K emails/month) | $0.00 |
| **Total** | **~$6.31/month** |

### Free Tier Eligible

* Azure Blob Storage: Free for first 12 months (5 GB + 20K operations)
* Mailgun: 5, 000 emails/month free forever

---

## Troubleshooting

### Container Keeps Restarting

```bash
# Check logs for errors
az container logs --resource-group inflyte-monitor-rg --name inflyte-monitor

# Check events
az container show \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --query instanceView.events
```

### "Failed to pull image"

Verify ACR credentials:

```bash
az acr credential show --name inflyteacr
```

### No Blob Files Created

Check environment variables are set correctly:

```bash
az container show \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --query containers[0].environmentVariables
```

### GitHub Actions Workflow Fails

1. Check all secrets are configured correctly
2. Verify OIDC federated credential matches your repo
3. Review workflow logs in Actions tab

---

## Security Best Practices

1. **Use OIDC for GitHub Actions** - No long-lived credentials
2. **Rotate storage keys** - Change every 90 days
3. **Use secure environment variables** - For sensitive data in ACI
4. **Enable Azure Monitor** - For alerting and diagnostics
5. **Restrict network access** - Configure firewall rules if needed

---

## Cleanup

Delete all resources:

```bash
# Delete entire resource group
az group delete --name inflyte-monitor-rg --yes
```

---

## Next Steps

* Configure Mailgun: See [MAILGUN_SETUP.md](MAILGUN_SETUP.md)
* Understand migration: See [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md)
* Review detailed Azure setup: See [AZURE_SETUP.md](AZURE_SETUP.md)

## References

* [Azure Container Instances Documentation](https://docs.microsoft.com/azure/container-instances/)
* [Azure Container Registry Documentation](https://docs.microsoft.com/azure/container-registry/)
* [GitHub Actions for Azure](https://github.com/Azure/actions)
* [Azure Login Action (OIDC)](https://github.com/Azure/login)
