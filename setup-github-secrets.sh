#!/bin/bash

# GitHub Actions Secrets Setup Script
# This script helps you set up all required GitHub secrets for the CI/CD pipeline

set -e

echo "🔐 GitHub Actions Secrets Setup for Inflyte DJ Monitor"
echo "========================================================"
echo ""

# Check if GitHub CLI is installed
if ! command -v gh &> /dev/null; then
    echo "❌ GitHub CLI (gh) is not installed."
    echo "   Install it from: https://cli.github.com/"
    echo "   Or use: brew install gh"
    exit 1
fi

# Check if Azure CLI is installed
if ! command -v az &> /dev/null; then
    echo "❌ Azure CLI (az) is not installed."
    echo "   Install it from: https://docs.microsoft.com/cli/azure/install-azure-cli"
    echo "   Or use: brew install azure-cli"
    exit 1
fi

echo "✅ GitHub CLI and Azure CLI are installed"
echo ""

# Check if user is logged in to GitHub
if ! gh auth status &> /dev/null; then
    echo "❌ Not logged in to GitHub CLI"
    echo "   Run: gh auth login"
    exit 1
fi

echo "✅ Logged in to GitHub CLI"
echo ""

# Get repository info
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || echo "")
if [ -z "$REPO" ]; then
    echo "❌ Not in a GitHub repository directory"
    exit 1
fi

echo "📦 Repository: $REPO"
echo ""

# Set Azure variables
RESOURCE_GROUP="inflyte-monitor-rg"
STORAGE_ACCOUNT="inflytemonitstg"
ACR_NAME="inflyteacr"

echo "🔍 Fetching Azure credentials..."
echo ""

# Get repository information for federated credential
REPO_OWNER=$(gh repo view --json owner -q .owner.login)
REPO_NAME=$(gh repo view --json name -q .name)

echo "📦 Repository: $REPO_OWNER/$REPO_NAME"
echo ""

# 1. Setup OIDC Authentication
echo "1️⃣  Setting up Azure App Registration with OIDC..."

# Check if app already exists
APP_ID=$(az ad app list --display-name "github-actions-inflyte-monitor" --query "[0].appId" -o tsv 2>/dev/null || echo "")

if [ -z "$APP_ID" ]; then
    echo "Creating new app registration..."
    az ad app create --display-name "github-actions-inflyte-monitor" > /dev/null
    APP_ID=$(az ad app list --display-name "github-actions-inflyte-monitor" --query "[0].appId" -o tsv)
    echo "✅ App registration created: $APP_ID"
    
    # Create service principal
    echo "Creating service principal..."
    az ad sp create --id $APP_ID > /dev/null
    
    # Wait a bit for the SP to be created
    sleep 5
    
    # Get SP object ID and assign role
    SP_OBJECT_ID=$(az ad sp show --id $APP_ID --query id -o tsv)
    echo "Assigning contributor role..."
    az role assignment create \
      --role contributor \
      --assignee-object-id $SP_OBJECT_ID \
      --assignee-principal-type ServicePrincipal \
      --scope /subscriptions/$(az account show --query id --output tsv)/resourceGroups/${RESOURCE_GROUP} > /dev/null
    echo "✅ Service principal configured with contributor role"
else
    echo "✅ Using existing app registration: $APP_ID"
fi

# Configure federated credential
echo "Configuring federated credential for GitHub Actions..."
CRED_NAME="github-actions-main"

# Check if credential already exists
EXISTING_CRED=$(az ad app federated-credential list --id $APP_ID --query "[?name=='$CRED_NAME'].name" -o tsv 2>/dev/null || echo "")

if [ -z "$EXISTING_CRED" ]; then
    az ad app federated-credential create \
      --id $APP_ID \
      --parameters "{
        \"name\": \"$CRED_NAME\",
        \"issuer\": \"https://token.actions.githubusercontent.com\",
        \"subject\": \"repo:${REPO_OWNER}/${REPO_NAME}:ref:refs/heads/main\",
        \"audiences\": [\"api://AzureADTokenExchange\"]
      }" > /dev/null
    echo "✅ Federated credential created for main branch"
else
    echo "✅ Federated credential already exists"
fi

# Set OIDC secrets
TENANT_ID=$(az account show --query tenantId -o tsv)
SUBSCRIPTION_ID=$(az account show --query id -o tsv)

gh secret set AZURE_CLIENT_ID --body "$APP_ID" --repo "$REPO"
echo "✅ Secret AZURE_CLIENT_ID set"

gh secret set AZURE_TENANT_ID --body "$TENANT_ID" --repo "$REPO"
echo "✅ Secret AZURE_TENANT_ID set"

gh secret set AZURE_SUBSCRIPTION_ID --body "$SUBSCRIPTION_ID" --repo "$REPO"
echo "✅ Secret AZURE_SUBSCRIPTION_ID set"
echo ""

# 2. ACR_PASSWORD
echo "2️⃣  Fetching Azure Container Registry password..."
ACR_PASSWORD=$(az acr credential show --name ${ACR_NAME} --query "passwords[0].value" --output tsv)
gh secret set ACR_PASSWORD --body "$ACR_PASSWORD" --repo "$REPO"
echo "✅ Secret ACR_PASSWORD set"
echo ""

# 3-6. Azure Storage
echo "3️⃣  Setting Azure Storage secrets..."
gh secret set AZURE_STORAGE_ACCOUNT --body "inflytemonitstg" --repo "$REPO"
echo "✅ Secret AZURE_STORAGE_ACCOUNT set"

gh secret set AZURE_STORAGE_CONTAINER --body "dj-monitor" --repo "$REPO"
echo "✅ Secret AZURE_STORAGE_CONTAINER set"

gh secret set AZURE_BLOB_NAME_PREFIX --body "dj_list" --repo "$REPO"
echo "✅ Secret AZURE_BLOB_NAME_PREFIX set"

STORAGE_KEY=$(az storage account keys list \
  --account-name ${STORAGE_ACCOUNT} \
  --resource-group ${RESOURCE_GROUP} \
  --query '[0].value' \
  --output tsv)
gh secret set AZURE_STORAGE_ACCESS_KEY --body "$STORAGE_KEY" --repo "$REPO"
echo "✅ Secret AZURE_STORAGE_ACCESS_KEY set"
echo ""

# 7-12. Mailgun and app configuration
echo "4️⃣  Setting Mailgun and application secrets..."
echo ""

# Load from .env if it exists
if [ -f .env ]; then
    echo "📄 Loading values from .env file..."
    source .env
    
    # Prompt for campaign URLs
    echo ""
    echo "Enter Inflyte campaign URLs (comma-separated):"
    echo "Example: https://inflyteapp.com/r/pmqtne,https://inflyteapp.com/r/campaign2"
    read -p "URLs: " INFLYTE_URLS_INPUT
    
    if [ -z "$INFLYTE_URLS_INPUT" ]; then
        echo "⚠️  No URLs provided, using default from .env or example"
        INFLYTE_URLS="${INFLYTE_URL:-https://inflyteapp.com/r/pmqtne}"
    else
        INFLYTE_URLS="$INFLYTE_URLS_INPUT"
    fi
    
    gh secret set INFLYTE_URLS --body "$INFLYTE_URLS" --repo "$REPO"
    echo "✅ Secret INFLYTE_URLS set"
    
    gh secret set MAILGUN_DOMAIN --body "$MAILGUN_DOMAIN" --repo "$REPO"
    echo "✅ Secret MAILGUN_DOMAIN set"
    
    gh secret set MAILGUN_API_KEY --body "$MAILGUN_API_KEY" --repo "$REPO"
    echo "✅ Secret MAILGUN_API_KEY set"
    
    gh secret set RECIPIENT_EMAIL --body "$RECIPIENT_EMAIL" --repo "$REPO"
    echo "✅ Secret RECIPIENT_EMAIL set"
    
    gh secret set FROM_EMAIL --body "$FROM_EMAIL" --repo "$REPO"
    echo "✅ Secret FROM_EMAIL set"
    
    gh secret set CHECK_INTERVAL_MINUTES --body "${CHECK_INTERVAL_MINUTES:-60}" --repo "$REPO"
    echo "✅ Secret CHECK_INTERVAL_MINUTES set"
else
    echo "⚠️  .env file not found. Please set Mailgun secrets manually:"
    echo ""
    echo "   gh secret set INFLYTE_URLS --body 'https://inflyteapp.com/r/campaign1,https://inflyteapp.com/r/campaign2'"
    echo "   gh secret set MAILGUN_DOMAIN --body 'your-domain.mailgun.org'"
    echo "   gh secret set MAILGUN_API_KEY --body 'your-api-key'"
    echo "   gh secret set RECIPIENT_EMAIL --body 'your-email@example.com'"
    echo "   gh secret set FROM_EMAIL --body 'noreply@your-domain.mailgun.org'"
    echo "   gh secret set CHECK_INTERVAL_MINUTES --body '60'"
fi

echo ""
echo "🎉 GitHub Actions secrets setup complete!"
echo ""
echo "🔐 Authentication: Using OpenID Connect (OIDC) - no long-lived secrets!"
echo ""
echo "📋 Summary of secrets:"
gh secret list --repo "$REPO"
echo ""
echo "🚀 Next steps:"
echo "   1. Verify all secrets are set correctly in GitHub Settings → Secrets"
echo "   2. Ensure the workflow has 'permissions: id-token: write' (already configured)"
echo "   3. Push code to 'main' branch to trigger the first deployment"
echo "   4. Monitor the deployment in the Actions tab"
echo ""
echo "📖 For more information, see GITHUB_ACTIONS_SETUP.md"
