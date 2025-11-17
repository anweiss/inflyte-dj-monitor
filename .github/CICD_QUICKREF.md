# CI/CD Quick Reference

## Automated Deployment Workflow

Every push to `main` triggers:

```
Push to main
    ↓
Build Docker Image (linux/amd64)
    ↓
Push to ACR (tagged with commit SHA)
    ↓
Delete Old Container
    ↓
Deploy New Container
    ↓
Verify Deployment
```

## One-Time Setup

```bash
# 1. Install GitHub CLI (if not already installed)
brew install gh

# 2. Login to GitHub CLI
gh auth login

# 3. Run the setup script
./setup-github-secrets.sh
```

## Manual Deployment Trigger

```bash
# Trigger deployment without pushing code
gh workflow run deploy.yml
```

Or via GitHub web UI:
1. Go to Actions tab
2. Select "Build and Deploy to Azure Container Instance"
3. Click "Run workflow"

## Monitoring

### View latest workflow run

```bash
gh run list --limit 5
```

### View workflow details

```bash
gh run view
```

### Follow workflow logs

```bash
gh run watch
```

## Verify Deployment

```bash
# Check container status
az container show \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor \
  --query "{State:instanceView.state, Image:containers[0].image}"

# View container logs
az container logs \
  --resource-group inflyte-monitor-rg \
  --name inflyte-monitor

# Check blob storage
az storage blob list \
  --container-name dj-monitor \
  --account-name inflytemonitstg \
  --output table
```

## Rollback

```bash
# List available image tags
az acr repository show-tags \
  --name inflyteacr \
  --repository inflyte-monitor \
  --orderby time_desc

# Update workflow to deploy specific SHA or manually deploy
```

## Update Secrets

```bash
# Update a single secret
gh secret set MAILGUN_API_KEY --body 'new-api-key'

# List all secrets
gh secret list

# Delete a secret
gh secret delete SECRET_NAME
```

## Troubleshooting

### Workflow fails at "Build Docker image"

* Check Dockerfile syntax
* Verify Cargo.toml dependencies
* Test locally: `docker build --platform linux/amd64 -t test .`

### Workflow fails at "Push to ACR"

* Verify `ACR_PASSWORD` secret is correct
* Check ACR admin is enabled: `az acr update --name inflyteacr --admin-enabled true`

### Workflow fails at "Deploy to Azure Container Instance"

* Verify `AZURE_CREDENTIALS` secret is valid
* Check service principal has contributor role
* Verify all environment variable secrets are set

### Container deployed but not running

* Check container logs in Azure
* Verify all required secrets match .env file
* Check for missing environment variables

## GitHub Actions Costs

* **Free tier:** 2, 000 minutes/month
* **This workflow:** ~2-3 minutes per run
* **Deployments/month:** 600-1, 000 within free tier

## Useful Commands

```bash
# Cancel running workflow
gh run cancel <run-id>

# Re-run failed workflow
gh run rerun <run-id>

# Download workflow logs
gh run download <run-id>

# View workflow YAML
gh workflow view deploy.yml

# List all workflows
gh workflow list
```
