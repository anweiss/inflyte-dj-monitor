# Azure Storage Setup Guide

This guide walks you through setting up Azure Blob Storage for the Inflyte DJ Monitor.

## Prerequisites

* Azure account ([create a free account](https://azure.microsoft.com/free/))
* Azure CLI installed ([installation guide](https://docs.microsoft.com/cli/azure/install-azure-cli))

## Step 1: Log into Azure

```bash
az login
```

This will open your browser for authentication. After logging in, the CLI will show your subscriptions.

If you have multiple subscriptions, set the one you want to use:

```bash
az account set --subscription "Your Subscription Name"
```

## Step 2: Create a Resource Group

A resource group is a logical container for Azure resources.

```bash
az group create \
  --name inflyte-monitor-rg \
  --location eastus
```

**Output:**

```json
{
  "id": "/subscriptions/xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx/resourceGroups/inflyte-monitor-rg",
  "location": "eastus",
  "name": "inflyte-monitor-rg",
  "properties": {
    "provisioningState": "Succeeded"
  }
}
```

**Note:** Choose a region close to you. Common options:
* `eastus` - US East
* `westus2` - US West
* `westeurope` - Europe
* `southeastasia` - Asia

## Step 3: Create a Storage Account

Storage account names must be:
* 3-24 characters
* Lowercase letters and numbers only
* Globally unique across all Azure

```bash
az storage account create \
  --name inflytemonitstg \
  --resource-group inflyte-monitor-rg \
  --location eastus \
  --sku Standard_LRS \
  --kind StorageV2
```

**Output:**

```json
{
  "id": "/subscriptions/.../storageAccounts/inflytemonitstg",
  "kind": "StorageV2",
  "location": "eastus",
  "name": "inflytemonitstg",
  "primaryLocation": "eastus",
  "provisioningState": "Succeeded",
  "sku": {
    "name": "Standard_LRS",
    "tier": "Standard"
  }
}
```

**SKU Options:**
* `Standard_LRS` - Locally redundant storage (cheapest, recommended for this use case)
* `Standard_GRS` - Geo-redundant storage
* `Premium_LRS` - Premium performance

## Step 4: Assign RBAC Permissions

Before creating containers, you need the proper permissions. Assign yourself the Storage Blob Data Contributor role:

```bash
# Get your Azure user principal ID
USER_PRINCIPAL_ID=$(az ad signed-in-user show --query id --output tsv)

# Assign the role
az role assignment create \
  --role "Storage Blob Data Contributor" \
  --assignee $USER_PRINCIPAL_ID \
  --scope "/subscriptions/$(az account show --query id --output tsv)/resourceGroups/inflyte-monitor-rg/providers/Microsoft.Storage/storageAccounts/inflytemonitstg"
```

**Output:**

```json
{
  "id": "/subscriptions/.../roleAssignments/...",
  "name": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "principalId": "...",
  "principalType": "User",
  "roleDefinitionId": "/subscriptions/.../roleDefinitions/ba92f5b4-2d11-453d-a403-e96b0029c9fe",
  "scope": "/subscriptions/.../storageAccounts/inflytemonitstg"
}
```

**Note:** Role assignments may take a few minutes to propagate. If the next step fails, wait 1-2 minutes and retry.

## Step 5: Create a Blob Container

You can create the container using either method below:

**Method A: Using Account Key (Works Immediately)**

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

**Method B: Using RBAC (May require 1-2 minute wait after Step 4)**

```bash
az storage container create \
  --name dj-monitor \
  --account-name inflytemonitstg \
  --auth-mode login
```

**Output:**

```json
{
  "created": true
}
```

Verify the container was created:

```bash
az storage container list \
  --account-name inflytemonitstg \
  --auth-mode login \
  --output table
```

If the `--auth-mode login` command fails with a network rules error, the role assignment from Step 4 hasn't propagated yet. Either wait 1-2 minutes and retry, or use the account key method:

```bash
az storage container list \
  --account-name inflytemonitstg \
  --account-key $(az storage account keys list \
    --account-name inflytemonitstg \
    --resource-group inflyte-monitor-rg \
    --query '[0].value' \
    --output tsv) \
  --output table
```

## Step 6: Get Storage Account Access Key

The application needs an access key to authenticate:

```bash
az storage account keys list \
  --account-name inflytemonitstg \
  --resource-group inflyte-monitor-rg \
  --output json
```

**Output:**

```json
{
  "keys": [
    {
      "creationTime": "2024-01-15T10:30:00.000000+00:00",
      "keyName": "key1",
      "permissions": "FULL",
      "value": "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrs=="
    },
    {
      "creationTime": "2024-01-15T10:30:00.000000+00:00",
      "keyName": "key2",
      "permissions": "FULL",
      "value": "zyxwvutsrqponmlkjihgfedcba9876543210ZYXWVUTSRQPONMLKJIHGFEDCBA9876543210zyxwvutsrqponmlkjih=="
    }
  ]
}
```

Extract just the first key value using jq:

```bash
az storage account keys list \
  --account-name inflytemonitstg \
  --resource-group inflyte-monitor-rg \
  --query '[0].value' \
  --output tsv
```

**Copy this key value** - you'll need it for your `.env` file.

## Step 7: Configure Application Environment Variables

Create or update your `.env` file with the Azure credentials:

```bash
# Azure Storage Configuration
AZURE_STORAGE_ACCOUNT=inflytemonitstg
AZURE_STORAGE_CONTAINER=dj-monitor
AZURE_BLOB_NAME=dj_list.json
AZURE_STORAGE_ACCESS_KEY=your-access-key-from-step-5

# Mailgun Configuration
MAILGUN_API_KEY=your-mailgun-api-key
MAILGUN_DOMAIN=your-domain.mailgun.org
RECIPIENT_EMAIL=you@example.com
FROM_EMAIL=noreply@inflyte.com

# Optional: Override default check interval (in minutes)
# CHECK_INTERVAL_MINUTES=60
```

**Security Note:** Never commit `.env` files to version control. The `.gitignore` file already excludes it.

## Alternative: Using SAS Tokens (More Secure)

Instead of using the full access key, you can generate a Shared Access Signature (SAS) token with limited permissions.

**Note:** When using `--as-user` authentication, SAS tokens are limited to 7 days maximum. For longer-lived tokens, use account key authentication.

### Option 1: User Delegation SAS (7 days max, most secure)

```bash
# Set expiry to 7 days from now (macOS)
EXPIRY=$(date -u -v+7d '+%Y-%m-%dT%H:%MZ')

# Or on Linux:
# EXPIRY=$(date -u -d "7 days" '+%Y-%m-%dT%H:%MZ')

az storage container generate-sas \
  --account-name inflytemonitstg \
  --name dj-monitor \
  --permissions rw \
  --expiry $EXPIRY \
  --auth-mode login \
  --as-user \
  --output tsv
```

### Option 2: Account Key SAS (up to 1 year, less secure)

For longer-lived tokens, use the account key method:

```bash
# Set expiry to 1 year from now (macOS)
EXPIRY=$(date -u -v+1y '+%Y-%m-%dT%H:%MZ')

# Or on Linux:
# EXPIRY=$(date -u -d "1 year" '+%Y-%m-%dT%H:%MZ')

az storage container generate-sas \
  --account-name inflytemonitstg \
  --name dj-monitor \
  --permissions rw \
  --expiry $EXPIRY \
  --account-key $(az storage account keys list \
    --account-name inflytemonitstg \
    --resource-group inflyte-monitor-rg \
    --query '[0].value' \
    --output tsv) \
  --output tsv
```

**Output:**

```
se=2025-01-15T10%3A30Z&sp=rw&sv=2023-01-03&sr=c&sig=abcdefghijklmnopqrstuvwxyz...
```

Then use this in your `.env` file instead of the access key:

```bash
AZURE_STORAGE_SAS_TOKEN=se=2025-01-15T10%3A30Z&sp=rw&sv=2023-01-03&sr=c&sig=...
```

Remove the `AZURE_STORAGE_ACCESS_KEY` line if using SAS token.

## Step 8: Verify Setup

Test that your configuration works:

```bash
cargo run
```

You should see:

```
🎵 Inflyte DJ Monitor Starting...
Monitoring: https://inflyteapp.com/r/pmqtne

Configuration:
  Storage Account: inflytemonitstg
  Storage Container: dj-monitor
  Blob Name: dj_list.json
  ...

Azure Blob Storage configured
Checking for new DJs...
Initial run - found XX DJs
```

You can also verify the blob was created in Azure:

```bash
az storage blob list \
  --container-name dj-monitor \
  --account-name inflytemonitstg \
  --auth-mode login \
  --output table
```

You should see `dj_list.json` in the list.

## Additional RBAC Notes for Production

The Storage Blob Data Contributor role was already assigned in Step 4 to allow container creation and blob operations.

For production deployments with managed identity in Azure (e.g., Azure Container Instances, Azure Functions), the application can authenticate automatically without storing credentials in environment variables. Simply assign the same role to your managed identity instead of your user account.

## Cost Breakdown

### Azure Blob Storage (Hot Tier)

* **Storage:** $0.0184 per GB/month
  + 1 KB JSON file = ~$0.00000002/month
* **Write Operations:** $0.05 per 10, 000 operations
  + 48 writes/day × 30 days = 1, 440 writes/month = ~$0.007
* **Read Operations:** $0.004 per 10, 000 operations
  + 48 reads/day × 30 days = 1, 440 reads/month = ~$0.0006

**Total:** ~$0.01/month (essentially free)

### Free Tier

Azure offers 5 GB free storage and 20, 000 free read/write operations per month for the first 12 months. This app will use:
* Storage: 0.000001 GB (well within free tier)
* Operations: ~2, 880/month (well within free tier)

**Result: FREE for 12 months on new Azure accounts!**

## Cleaning Up Resources

If you want to delete everything to avoid charges:

```bash
# Delete the entire resource group (removes storage account, container, and all data)
az group delete --name inflyte-monitor-rg --yes
```

## Troubleshooting

### "Storage account name already taken"

Storage account names are globally unique. Try a different name:

```bash
az storage account create --name inflytedjmon$(date +%s) ...
```

### "Authorization permission mismatch"

If using `--auth-mode login` , ensure you have the correct RBAC role assigned:

```bash
az role assignment create \
  --role "Storage Blob Data Contributor" \
  --assignee $(az ad signed-in-user show --query id --output tsv) \
  --scope "/subscriptions/$(az account show --query id --output tsv)/resourceGroups/inflyte-monitor-rg"
```

### "The specified container does not exist"

Make sure you created the container:

```bash
az storage container create --name dj-monitor --account-name inflytemonitstg --auth-mode login
```

## Security Best Practices

1. **Never commit `.env` files** - Already in `.gitignore`
2. **Rotate access keys regularly** - Change them every 90 days
3. **Use SAS tokens** - For applications that don't need full account access
4. **Use RBAC and Managed Identity** - For Azure-hosted deployments
5. **Enable firewall rules** - Restrict access to specific IPs if needed
6. **Monitor access logs** - Enable Storage Analytics logging

## Next Steps

* ✅ Azure Storage configured
* 📧 Configure Mailgun (see [MAILGUN_SETUP.md](MAILGUN_SETUP.md))
* 🚀 Run the application: `cargo run --release`
* 📊 Monitor usage in [Azure Portal](https://portal.azure.com)

## Useful Azure CLI Commands

```bash
# List all storage accounts
az storage account list --output table

# Show container properties
az storage container show \
  --name dj-monitor \
  --account-name inflytemonitstg \
  --auth-mode login

# Download the DJ list blob
az storage blob download \
  --container-name dj-monitor \
  --name dj_list.json \
  --file dj_list_backup.json \
  --account-name inflytemonitstg \
  --auth-mode login

# View blob contents without downloading
az storage blob download \
  --container-name dj-monitor \
  --name dj_list.json \
  --account-name inflytemonitstg \
  --auth-mode login \
  --output json | jq -r '.content' | base64 -d

# Enable blob versioning (keep history of changes)
az storage account blob-service-properties update \
  --account-name inflytemonitstg \
  --enable-versioning true
```

## References

* [Azure Blob Storage Documentation](https://docs.microsoft.com/azure/storage/blobs/)
* [Azure CLI Reference](https://docs.microsoft.com/cli/azure/storage)
* [Azure Storage Pricing](https://azure.microsoft.com/pricing/details/storage/blobs/)
* [Azure RBAC Documentation](https://docs.microsoft.com/azure/role-based-access-control/)
