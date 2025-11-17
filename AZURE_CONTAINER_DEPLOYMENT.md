# Deploying to Azure Container Instances

This guide walks you through deploying the Inflyte DJ Monitor as an Azure Container Instance.

## Prerequisites

* Completed [AZURE_SETUP.md](AZURE_SETUP.md) (storage account configured)
* Docker installed locally ([installation guide](https://docs.docker.com/get-docker/))
* Azure CLI installed and logged in

## Step 1: Create Azure Container Registry

Azure Container Registry (ACR) stores your Docker images.

```bash
# Create the container registry
az acr create \
  --resource-group inflyte-monitor-rg \
  --name inflyteacr \
  --sku Basic \
  --location eastus

# Enable admin access (for simple authentication)
az acr update \
  --name inflyteacr \
  --admin-enabled true
```

**Note:** ACR names must be globally unique and contain only lowercase letters and numbers (5-50 characters).

## Step 2: Build and Push Docker Image

### Build the Docker image locally

**Important:** Azure Container Instances requires `linux/amd64` architecture. If you're on Apple Silicon (M1/M2/M3), you must specify the platform:

```bash
# Navigate to your project directory
cd /path/to/inflyte

# Build the Docker image for linux/amd64
docker build --platform linux/amd64 -t inflyte-monitor:latest .
```

**Note:** Cross-platform builds on Apple Silicon will be slower due to emulation.

### Log in to Azure Container Registry

```bash
# Get ACR login server
ACR_LOGIN_SERVER=$(az acr show --name inflyteacr --query loginServer --output tsv)

# Log in to ACR
az acr login --name inflyteacr
```

### Tag and push the image

```bash
# Tag the image for ACR
docker tag inflyte-monitor:latest ${ACR_LOGIN_SERVER}/inflyte-monitor:latest

# Push to ACR
docker push ${ACR_LOGIN_SERVER}/inflyte-monitor:latest
```

Verify the image was pushed:

```bash
az acr repository list --name inflyteacr --output table
```

## Step 3: Get ACR Credentials

```bash
# Get ACR password
ACR_PASSWORD=$(az acr credential show --name inflyteacr --query "passwords[0].value" --output tsv)

# Store for later use
echo "ACR Login Server: ${ACR_LOGIN_SERVER}"
echo "ACR Username: inflyteacr"
echo "ACR Password: ${ACR_PASSWORD}"
```

## Step 4: Get Azure Storage Access Key

```bash
# Get the storage account access key
STORAGE_KEY=$(az storage account keys list \
  --account-name inflytemonitstg \
  --resource-group inflyte-monitor-rg \
  --query '[0].value' \
  --output tsv)

echo "Storage Key: ${STORAGE_KEY}"
```

## Step 5: Create Azure Container Instance

Create a container instance with environment variables for configuration:

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
    AZURE_STORAGE_ACCOUNT=inflytemonitstg \
    AZURE_STORAGE_CONTAINER=dj-monitor \
    AZURE_BLOB_NAME=dj_list.json \
    MAILGUN_DOMAIN=your-domain.mailgun.org \
    RECIPIENT_EMAIL=you@example.com \
    FROM_EMAIL=noreply@inflyte.com \
    CHECK_INTERVAL_MINUTES=60 \
  --secure-environment-variables \
    AZURE_STORAGE_ACCESS_KEY=${STORAGE_KEY} \
    MAILGUN_API_KEY=your-mailgun-api-key
```

**Important:** Replace `your-domain.mailgun.org` , `you@example.com` , and `your-mailgun-api-key` with your actual values.

## Step 6: Verify Deployment

Check the container status:

```bash
az container show \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --query "{Status:instanceView.state, IP:ipAddress.ip}" \
  --output table
```

View container logs:

```bash
az container logs \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --follow
```

You should see output like:

```
🎵 Inflyte DJ Monitor Starting...
Monitoring: https://inflyteapp.com/r/pmqtne

Configuration:
  Azure Storage Account: inflytemonitstg
  Azure Container: dj-monitor
  Blob Name: dj_list.json
  ...

Azure Blob Storage configured
Checking for new DJs...
```

## Updating the Deployment

When you make code changes, rebuild and redeploy:

```bash
# Rebuild the Docker image for linux/amd64
docker build --platform linux/amd64 -t inflyte-monitor:latest .

# Tag and push to ACR
docker tag inflyte-monitor:latest ${ACR_LOGIN_SERVER}/inflyte-monitor:latest
docker push ${ACR_LOGIN_SERVER}/inflyte-monitor:latest

# Delete the old container instance
az container delete \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --yes

# Recreate with the same command from Step 5
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
    AZURE_STORAGE_ACCOUNT=inflytemonitstg \
    AZURE_STORAGE_CONTAINER=dj-monitor \
    AZURE_BLOB_NAME=dj_list.json \
    MAILGUN_DOMAIN=your-domain.mailgun.org \
    RECIPIENT_EMAIL=you@example.com \
    FROM_EMAIL=noreply@inflyte.com \
    CHECK_INTERVAL_MINUTES=60 \
  --secure-environment-variables \
    AZURE_STORAGE_ACCESS_KEY=${STORAGE_KEY} \
    MAILGUN_API_KEY=your-mailgun-api-key
```

## Alternative: Using Managed Identity (More Secure)

Instead of using storage access keys, you can use managed identity:

### 1. Create container instance with managed identity

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
  --assign-identity \
  --environment-variables \
    AZURE_STORAGE_ACCOUNT=inflytemonitstg \
    AZURE_STORAGE_CONTAINER=dj-monitor \
    AZURE_BLOB_NAME=dj_list.json \
    MAILGUN_DOMAIN=your-domain.mailgun.org \
    RECIPIENT_EMAIL=you@example.com \
    FROM_EMAIL=noreply@inflyte.com \
    CHECK_INTERVAL_MINUTES=60 \
  --secure-environment-variables \
    MAILGUN_API_KEY=your-mailgun-api-key
```

### 2. Assign Storage Blob Data Contributor role to the managed identity

```bash
# Get the managed identity principal ID
PRINCIPAL_ID=$(az container show \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --query identity.principalId \
  --output tsv)

# Assign the role
az role assignment create \
  --role "Storage Blob Data Contributor" \
  --assignee ${PRINCIPAL_ID} \
  --scope "/subscriptions/$(az account show --query id --output tsv)/resourceGroups/inflyte-monitor-rg/providers/Microsoft.Storage/storageAccounts/inflytemonitstg"
```

**Note:** The application code would need to be modified to use `DefaultAzureCredential` instead of access key authentication for this to work.

## Monitoring and Management

### View logs

```bash
# View logs
az container logs \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor

# Follow logs in real-time (may fail with InternalServerError if logs aren't ready yet)
az container logs \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --follow
```

**Note:** If you get an `InternalServerError` or `AttributeError` when viewing logs, it's likely because:

* The container just started and logs aren't available yet
* Azure's logging service is initializing

To verify the container is working without logs:

```bash
# Check if the DJ list blob was created
az storage blob list \
  --container-name dj-monitor \
  --account-name inflytemonitstg \
  --query "[].{Name:name, Created:properties.creationTime, Size:properties.contentLength}" \
  --output table
```

If `dj_list.json` exists, the application is running successfully.

### Restart the container

```bash
az container restart \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor
```

### Stop the container

```bash
az container stop \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor
```

### Start the container

```bash
az container start \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor
```

### Delete the container

```bash
az container delete \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --yes
```

## Cost Estimates

### Azure Container Instances

Based on 1 vCPU, 0.5 GB RAM, running 24/7:

* **Compute:** ~$0.0000012/vCPU/second = ~$3.11/month
* **Memory:** ~$0.00000014/GB/second = ~$0.18/month

**Total ACI Cost:** ~$3.30/month

### Azure Container Registry (Basic)

* **Storage:** $0.10/day = ~$3.00/month
* **Included:** 10 GB storage, unlimited pulls

**Total ACR Cost:** ~$3.00/month

### Combined Monthly Cost

* Azure Container Instances: $3.30
* Azure Container Registry: $3.00
* Azure Blob Storage: $0.01
* Mailgun: Free (under 5, 000 emails/month)

**Total: ~$6.31/month**

## Troubleshooting

### "Failed to pull image"

Check ACR credentials:

```bash
az acr credential show --name inflyteacr
```

### Container keeps restarting

View logs to see the error:

```bash
az container logs --resource-group inflyte-monitor-rg --name inflyte-monitor
```

### "AZURE_STORAGE_ACCESS_KEY environment variable not set"

Ensure you included `--secure-environment-variables` in the create command with the correct storage key.

### Check container events

```bash
az container show \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --query instanceView.events \
  --output table
```

## Clean Up

To delete all resources:

```bash
# Delete container instance
az container delete --resource-group inflyte-monitor-rg --name inflyte-monitor --yes

# Delete container registry
az acr delete --name inflyteacr --yes

# Delete entire resource group (including storage account)
az group delete --name inflyte-monitor-rg --yes
```

## Next Steps

* Set up Azure Monitor alerts for container failures
* Configure Application Insights for detailed monitoring
* Set up Azure DevOps or GitHub Actions for CI/CD
* Consider Azure Container Apps for more advanced orchestration

## References

* [Azure Container Instances Documentation](https://docs.microsoft.com/azure/container-instances/)
* [Azure Container Registry Documentation](https://docs.microsoft.com/azure/container-registry/)
* [Azure Container Instances Pricing](https://azure.microsoft.com/pricing/details/container-instances/)
