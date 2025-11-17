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

# 1. AZURE_CREDENTIALS
echo "1️⃣  Creating Azure Service Principal for GitHub Actions..."
AZURE_CREDENTIALS=$(az ad sp create-for-rbac \
  --name "github-actions-inflyte-monitor" \
  --role contributor \
  --scopes /subscriptions/$(az account show --query id --output tsv)/resourceGroups/${RESOURCE_GROUP} \
  --sdk-auth 2>/dev/null || echo "")

if [ -z "$AZURE_CREDENTIALS" ]; then
    echo "⚠️  Service principal might already exist. Trying to get existing credentials..."
    echo "   If this fails, manually create the service principal or delete the existing one."
else
    echo "✅ Service principal created"
    gh secret set AZURE_CREDENTIALS --body "$AZURE_CREDENTIALS" --repo "$REPO"
    echo "✅ Secret AZURE_CREDENTIALS set"
fi
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

gh secret set AZURE_BLOB_NAME --body "dj_list.json" --repo "$REPO"
echo "✅ Secret AZURE_BLOB_NAME set"

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
    
    gh secret set INFLYTE_URL --body "${INFLYTE_URL:-https://inflyteapp.com/r/pmqtne}" --repo "$REPO"
    echo "✅ Secret INFLYTE_URL set"
    
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
    echo "   gh secret set INFLYTE_URL --body 'https://inflyteapp.com/r/YOUR_EVENT'"
    echo "   gh secret set MAILGUN_DOMAIN --body 'your-domain.mailgun.org'"
    echo "   gh secret set MAILGUN_API_KEY --body 'your-api-key'"
    echo "   gh secret set RECIPIENT_EMAIL --body 'your-email@example.com'"
    echo "   gh secret set FROM_EMAIL --body 'noreply@your-domain.mailgun.org'"
    echo "   gh secret set CHECK_INTERVAL_MINUTES --body '60'"
fi

echo ""
echo "🎉 GitHub Actions secrets setup complete!"
echo ""
echo "📋 Summary of secrets:"
gh secret list --repo "$REPO"
echo ""
echo "🚀 Next steps:"
echo "   1. Verify all secrets are set correctly in GitHub Settings → Secrets"
echo "   2. Push code to 'main' branch to trigger the first deployment"
echo "   3. Monitor the deployment in the Actions tab"
echo ""
echo "📖 For more information, see GITHUB_ACTIONS_SETUP.md"
