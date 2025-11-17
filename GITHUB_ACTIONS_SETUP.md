# GitHub Actions CI/CD Setup

This repository includes a GitHub Actions workflow that automatically builds the Docker image and deploys it to Azure Container Instances whenever code is pushed to the `main` branch.

## Required GitHub Secrets

You need to configure the following secrets in your GitHub repository:

### Navigate to Settings → Secrets and variables → Actions → New repository secret

### Azure Credentials

#### 1. `AZURE_CREDENTIALS`

Create an Azure Service Principal with contributor access:

```bash
az ad sp create-for-rbac \
  --name "github-actions-inflyte-monitor" \
  --role contributor \
  --scopes /subscriptions/$(az account show --query id --output tsv)/resourceGroups/inflyte-monitor-rg \
  --sdk-auth
```

Copy the entire JSON output and save it as the `AZURE_CREDENTIALS` secret.

#### 2. `ACR_PASSWORD`

Get your Azure Container Registry password:

```bash
az acr credential show --name inflyteacr --query "passwords[0].value" --output tsv
```

### Azure Storage Configuration

#### 3. `AZURE_STORAGE_ACCOUNT`

Value: `inflytemonitstg`

#### 4. `AZURE_STORAGE_CONTAINER`

Value: `dj-monitor`

#### 5. `AZURE_BLOB_NAME`

Value: `dj_list.json`

#### 6. `AZURE_STORAGE_ACCESS_KEY`

Get your storage account access key:

```bash
az storage account keys list \
  --account-name inflytemonitstg \
  --resource-group inflyte-monitor-rg \
  --query '[0].value' \
  --output tsv
```

### Mailgun Configuration

#### 7. `MAILGUN_DOMAIN`

Value: `sandbox958cf91827134ddfa60ac99d46fa7b02.mailgun.org`

#### 8. `MAILGUN_API_KEY`

Value: Your Mailgun API key (from Mailgun dashboard)

#### 9. `RECIPIENT_EMAIL`

Value: `andrew@weisstech.guru` (or your email address)

#### 10. `FROM_EMAIL`

Value: `noreply@sandbox958cf91827134ddfa60ac99d46fa7b02.mailgun.org`

#### 11. `CHECK_INTERVAL_MINUTES`

Value: `60`

## Secrets Summary

Here's a quick reference of all required secrets:

| Secret Name | Description | Example Value |
|-------------|-------------|---------------|
| `AZURE_CREDENTIALS` | Service principal JSON | `{"clientId": "...", ...}` |
| `ACR_PASSWORD` | Container registry password | From `az acr credential show` |
| `AZURE_STORAGE_ACCOUNT` | Storage account name | `inflytemonitstg` |
| `AZURE_STORAGE_CONTAINER` | Blob container name | `dj-monitor` |
| `AZURE_BLOB_NAME` | Blob file name | `dj_list.json` |
| `AZURE_STORAGE_ACCESS_KEY` | Storage access key | From `az storage account keys list` |
| `MAILGUN_DOMAIN` | Mailgun domain | `sandbox958cf...mailgun.org` |
| `MAILGUN_API_KEY` | Mailgun API key | From Mailgun dashboard |
| `RECIPIENT_EMAIL` | Email recipient | `andrew@weisstech.guru` |
| `FROM_EMAIL` | Sender email | `noreply@sandbox...` |
| `CHECK_INTERVAL_MINUTES` | Check frequency | `60` |

## Workflow Triggers

The workflow runs:

* **Automatically:** On every push to the `main` branch
* **Manually:** Via the "Actions" tab → "Build and Deploy to Azure Container Instance" → "Run workflow"

## Workflow Steps

1. **Checkout code** - Gets the latest code from the repository
2. **Log in to ACR** - Authenticates with Azure Container Registry
3. **Build Docker image** - Builds the Docker image for linux/amd64 platform
4. **Push to ACR** - Pushes both `:latest` and `:${{ github.sha }}` tagged images
5. **Log in to Azure** - Authenticates with Azure using service principal
6. **Delete existing container** - Removes the old container instance
7. **Deploy new container** - Creates a new container instance with the latest image
8. **Verify deployment** - Shows the deployment status

## Monitoring Deployments

### View workflow runs

Go to the **Actions** tab in your GitHub repository to see:

* Build status
* Deployment logs
* Error messages (if any)

### Check container status

After deployment completes, verify in Azure:

```bash
az container show \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --query "{Name:name, State:instanceView.state, RestartCount:containers[0].instanceView.restartCount}" \
  --output table
```

### View container logs

```bash
az container logs \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor
```

## Deployment History

Each deployment is tagged with the git commit SHA, allowing you to track which version is running:

```bash
# List all images in ACR
az acr repository show-tags \
  --name inflyteacr \
  --repository inflyte-monitor \
  --orderby time_desc \
  --output table
```

## Rollback to Previous Version

If you need to rollback to a previous version:

```bash
# Find the commit SHA you want to rollback to
az acr repository show-tags --name inflyteacr --repository inflyte-monitor --output table

# Delete the current container
az container delete --resource-group inflyte-monitor-rg --name inflyte-monitor --yes

# Deploy the specific version (replace COMMIT_SHA)
az container create \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --image inflyteacr.azurecr.io/inflyte-monitor:COMMIT_SHA \
  --registry-login-server inflyteacr.azurecr.io \
  --registry-username inflyteacr \
  --registry-password $(az acr credential show --name inflyteacr --query "passwords[0].value" --output tsv) \
  --cpu 1 \
  --memory 0.5 \
  --restart-policy Always \
  --os-type Linux \
  --environment-variables \
    AZURE_STORAGE_ACCOUNT=inflytemonitstg \
    AZURE_STORAGE_CONTAINER=dj-monitor \
    AZURE_BLOB_NAME=dj_list.json \
    MAILGUN_DOMAIN=sandbox958cf91827134ddfa60ac99d46fa7b02.mailgun.org \
    RECIPIENT_EMAIL=andrew@weisstech.guru \
    FROM_EMAIL=noreply@sandbox958cf91827134ddfa60ac99d46fa7b02.mailgun.org \
    CHECK_INTERVAL_MINUTES=60 \
  --secure-environment-variables \
    AZURE_STORAGE_ACCESS_KEY=$(az storage account keys list --account-name inflytemonitstg --resource-group inflyte-monitor-rg --query '[0].value' --output tsv) \
    MAILGUN_API_KEY=your-mailgun-api-key
```

## Troubleshooting

### "Error: Azure login action failed"

* Check that `AZURE_CREDENTIALS` secret is correctly formatted JSON
* Verify the service principal has contributor role on the resource group

### "Error: ACR authentication failed"

* Verify `ACR_PASSWORD` secret matches the output of `az acr credential show`

### "Container fails to start after deployment"

* Check the workflow logs for the deployment step
* Verify all secrets are set correctly
* Check container logs: `az container logs --resource-group inflyte-monitor-rg --name inflyte-monitor`

### "Deployment succeeds but container restarts"

* Missing or incorrect environment variables
* Check container instance view: `az container show --resource-group inflyte-monitor-rg --name inflyte-monitor --query "containers[0].instanceView"`

## Best Practices

* **Use git tags for releases:** Tag important commits with versions (e.g., `v1.0.0`)
* **Monitor workflow runs:** Enable email notifications for failed workflows
* **Test locally first:** Always test Docker builds locally before pushing
* **Review secrets regularly:** Rotate passwords and keys periodically
* **Use environments:** Consider using GitHub Environments for production deployments with approval gates

## Cost Considerations

GitHub Actions includes:

* **2,000 minutes/month free** for private repositories
* This workflow uses ~2-3 minutes per deployment
* Approximately 600-1,000 deployments per month within free tier

## Next Steps

1. Set up all required GitHub secrets
2. Push code to `main` branch to trigger the workflow
3. Monitor the Actions tab for deployment progress
4. Verify container is running in Azure
5. Set up email notifications for workflow failures
