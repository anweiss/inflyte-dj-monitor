# Inflyte DJ Monitor

A Rust tool that periodically scrapes multiple Inflyte campaign URLs and monitors the Support section for new DJs, with cloud storage via Azure Blob Storage and email notifications via Mailgun.

## 🎵 Currently Monitored Campaigns

**Status:** 🔴 Not Yet Deployed  
**Total Campaigns:** 0  
**Check Interval:** N/A  
**Last Updated:** Awaiting first deployment

| Campaign | Track | DJs | Last Checked |
|----------|-------|-----|--------------|
| _No campaigns active yet_ | - | - | - |

> This section is automatically updated every 6 hours by querying the deployed application.

## Features

* 🔍 **Web Scraping** - Automatically scrapes multiple Inflyte campaign pages at configurable intervals
* 📊 **DJ Detection** - Extracts DJ names, comments, and star ratings from the Support section for each campaign
* ⭐ **Comment & Rating Tracking** - Monitors DJ feedback including text comments and star ratings
* 🎯 **Multi-Campaign Support** - Monitor multiple Inflyte campaigns simultaneously
* 📁 **Flexible URL Configuration** - Load URLs from command-line arguments or a flat file
* ☁️ **Cloud Storage** - Stores DJ lists per campaign in Azure Blob Storage for persistent, cloud-based tracking
* 📧 **Email Alerts** - Sends beautiful HTML email notifications via Mailgun with campaign details, comments, and ratings
* 🌐 **Live Status API** - HTTP endpoint exposing real-time campaign monitoring status
* 📝 **Auto-Updated README** - GitHub Actions automatically updates this README with current campaign stats every 6 hours

## Prerequisites

* **Rust** (latest stable version)
* **Azure Account** with Storage access
* **Mailgun Account** (free tier available at <https://mailgun.com>)
* **Azure Storage credentials** configured

## Quick Start

### 1. Clone and Build

```bash
git clone <your-repo>
cd inflyte
cargo build --release
```

### 2. Azure Storage Setup

Create a storage account and container:

```bash
# Set variables
export RESOURCE_GROUP="inflyte-rg"
export STORAGE_ACCOUNT="inflytedjmonitor$(date +%s | tail -c 6)"
export LOCATION="eastus"

# Create resource group (if needed)
az group create --name $RESOURCE_GROUP --location $LOCATION

# Create storage account
az storage account create \
  --name $STORAGE_ACCOUNT \
  --resource-group $RESOURCE_GROUP \
  --location $LOCATION \
  --sku Standard_LRS

# Create container
az storage container create \
  --name inflyte-dj-monitor \
  --account-name $STORAGE_ACCOUNT

# Get access key
az storage account keys list \
  --account-name $STORAGE_ACCOUNT \
  --resource-group $RESOURCE_GROUP \
  --query '[0].value' -o tsv
```

See [DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md) for complete deployment instructions (Azure Storage, Container Registry, GitHub Actions).

### 3. Mailgun Setup

1. Sign up at <https://mailgun.com> (free tier: 5, 000 emails/month)
2. Verify your domain OR use the provided sandbox domain
3. Get your API key from: Settings → API Keys
4. For sandbox domains: Add authorized recipients in Sending → Sending domains → Authorized Recipients

### 4. Configure Environment Variables

```bash
cp .env.example .env
```

Edit `.env` :

```bash
# Azure Storage Configuration
AZURE_STORAGE_ACCOUNT=inflytedjmonitor123456
AZURE_STORAGE_CONTAINER=inflyte-dj-monitor
AZURE_BLOB_NAME_PREFIX=dj_list
AZURE_STORAGE_ACCESS_KEY=your-storage-access-key-here

# Mailgun Configuration
MAILGUN_API_KEY=your-mailgun-api-key-here
MAILGUN_DOMAIN=sandboxXXX.mailgun.org
RECIPIENT_EMAIL=your-email@example.com
FROM_EMAIL=noreply@sandboxXXX.mailgun.org

# App Configuration
CHECK_INTERVAL_MINUTES=60
```

**Azure Credentials:** Use either:

1. Storage Access Key (recommended for development)
2. SAS Token (for limited-time access)

### 5. Run the Monitor

#### Option 1: Using Command-Line URLs

```bash
# Load environment and run with one or more URLs
cargo run --release -- --url https://inflyteapp.com/r/pmqtne

# Monitor multiple campaigns (comma-separated)
cargo run --release -- --url https://inflyteapp.com/r/campaign1,https://inflyteapp.com/r/campaign2

# Or use multiple --url flags
cargo run --release -- --url https://inflyteapp.com/r/campaign1 --url https://inflyteapp.com/r/campaign2
```

#### Option 2: Using a URL File (Recommended for Multiple Campaigns)

Create a `urls.txt` file with one URL per line:

```bash
# Copy the example file
cp urls.txt.example urls.txt

# Edit urls.txt to add your campaign URLs
# Lines starting with # are comments and will be ignored
```

Example `urls.txt` :

```text
# My Inflyte Campaigns
https://inflyteapp.com/r/pmqtne
https://inflyteapp.com/r/campaign2
https://inflyteapp.com/r/campaign3
```

Then run:

```bash
cargo run --release -- --file urls.txt
```

#### Option 3: Combine Both Methods

You can use both command-line URLs and a file simultaneously:

```bash
cargo run --release -- --url https://inflyteapp.com/r/extra --file urls.txt
```

## How It Works

```text
┌─────────────────┐
│  Initial Run    │
│  - Fetch DJs    │
│  - Save to Blob │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Wait Interval  │
│  (60 minutes)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Check for New  │
│  DJs on Page    │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
 New DJs?   No change
    │         │
    ▼         ▼
┌────────┐  Continue
│ Email! │  monitoring
│ Alert  │
└────────┘
```

1. **Scrape** - Fetches the Inflyte page and extracts Support section
2. **Compare** - Loads previous DJ list from Azure Blob Storage and compares
3. **Alert** - If new DJs found, sends email via Mailgun
4. **Store** - Updates the list in Azure Blob Storage
5. **Repeat** - Waits for configured interval and repeats

## Output Examples

### Initial Run

```text
🎵 Inflyte DJ Monitor Starting...
Monitoring 2 campaign(s):

Configuration:
  Azure Storage Account: inflytemonitstg
  Azure Container: dj-monitor
  Blob Name Prefix: dj_list
  Email To: you@example.com
  Email From: noreply@sandbox123.mailgun.org
  Mailgun Domain: sandbox123.mailgun.org
  Check Interval: 60 minutes

Campaigns:
  • pmqtne (https://inflyteapp.com/r/pmqtne)
  • campaign2 (https://inflyteapp.com/r/campaign2)

Azure Blob Storage configured

Checking pmqtne for new DJs...
Initial run for pmqtne - found 27 DJs

Checking campaign2 for new DJs...
Initial run for campaign2 - found 15 DJs
```

### When New DJs Are Detected

```text
Checking pmqtne for new DJs...

🚨 ALERT: New DJ support detected for pmqtne!
═══════════════════════════════
  ✨ Vitor Saguanza ⭐⭐⭐⭐⭐
     💬 "Beautiful vibe!"
  ✨ New DJ Name 2
═══════════════════════════════

✅ Email notification sent to you@example.com
```

### No Changes

```text
Checking pmqtne for new DJs...
No new DJs found for pmqtne. Total: 27

Checking campaign2 for new DJs...
No new DJs found for campaign2. Total: 15
```

## Email Notification Example

When new DJs are detected, you'll receive a beautifully formatted HTML email with comments and star ratings:

**Subject:** 🚨 2 New DJ Support/Comment to Inflyte Campaign 'pmqtne'

**Body:**

```text
🎵 Inflyte DJ Monitor Alert
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

New DJs have been added to the Support section!

Campaign: pmqtne

New Support (2)
  ✨ Vitor Saguanza ⭐⭐⭐⭐⭐
     "Beautiful vibe!"
  ✨ New DJ Name 2

View the full list at: https://inflyteapp.com/r/pmqtne
```

## Configuration Options

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `AZURE_STORAGE_ACCOUNT` | ✅ Yes | - | Azure Storage account name |
| `AZURE_STORAGE_CONTAINER` | ✅ Yes | - | Azure Blob container name |
| `AZURE_BLOB_NAME_PREFIX` | No | `dj_list` | Prefix for blob names (campaign name will be appended) |
| `AZURE_STORAGE_ACCESS_KEY` | ✅ Yes* | - | Azure Storage access key (or use SAS token) |
| `AZURE_STORAGE_SAS_TOKEN` | ✅ Yes* | - | Azure Storage SAS token (alternative to access key) |
| `MAILGUN_API_KEY` | ✅ Yes | - | Your Mailgun API key |
| `MAILGUN_DOMAIN` | ✅ Yes | - | Your Mailgun domain |
| `RECIPIENT_EMAIL` | ✅ Yes | - | Email address to receive alerts |
| `FROM_EMAIL` | No | `noreply@inflyte.com` | Sender email address |
| `CHECK_INTERVAL_MINUTES` | No | `60` | Minutes between checks |
| `HTTP_PORT` | No | `8080` | Port for status API endpoint |

## API Endpoints

The application exposes HTTP endpoints for monitoring status:

### `GET /health`

Health check endpoint that returns `OK` if the service is running.

**Example:**

```bash
curl http://localhost:8080/health
```

### `GET /campaigns`

Returns current campaign monitoring status in JSON format.

**Example Response:**

```json
{
  "status": "active",
  "total_campaigns": 2,
  "check_interval_minutes": 60,
  "campaigns": [
    {
      "name": "pmqtne",
      "url": "https://inflyteapp.com/r/pmqtne",
      "track_title": "Artist - Track Name",
      "dj_count": 27,
      "last_checked": "2025-11-18T15:30:00Z"
    }
  ]
}
```

**Example:**

```bash
curl http://localhost:8080/campaigns
```

When deployed to Azure Container Instances, these endpoints are publicly accessible via the container's IP address on port 8080.

## Deployment Options

### Running Locally

```bash
# Single campaign via command-line
cargo run --release -- --url https://inflyteapp.com/r/pmqtne

# Multiple campaigns via command-line
cargo run --release -- --url https://inflyteapp.com/r/campaign1,https://inflyteapp.com/r/campaign2

# Using a URL file (recommended)
cargo run --release -- --file urls.txt
```

### Running as a Background Service (Linux)

Create a systemd service file at `/etc/systemd/system/inflyte-monitor.service` :

```ini
[Unit]
Description=Inflyte DJ Monitor
After=network.target

[Service]
Type=simple
User=youruser
WorkingDirectory=/path/to/inflyte
EnvironmentFile=/path/to/inflyte/.env
ExecStart=/path/to/inflyte/target/release/inflyte --file /path/to/inflyte/urls.txt
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

### Running on Azure Container Instances

For a fully managed, always-on deployment with automatic CI/CD, see:

* **[AZURE_CONTAINER_DEPLOYMENT.md](AZURE_CONTAINER_DEPLOYMENT.md)** - Manual Docker deployment
* **[GITHUB_ACTIONS_SETUP.md](GITHUB_ACTIONS_SETUP.md)** - Automated CI/CD with GitHub Actions

Quick summary:

1. Build Docker image
2. Push to Azure Container Registry
3. Create container instance with environment variables
4. Set up GitHub Actions for automatic deployments
5. Monitor via Azure CLI or portal

**Cost:** ~$6.31/month for 24/7 operation

**Deployment:** Automatic on every push to `main` branch (via GitHub Actions)

### Running on Azure Functions

For less frequent checks (e.g., once per day), you can run this as an Azure Function triggered by a timer.

## Troubleshooting

### "AZURE_STORAGE_ACCOUNT environment variable not set"

* Make sure you've created a `.env` file
* Verify `.env` is in the same directory as `Cargo.toml`
* Check that all Azure Storage variables are set in `.env`

### "Failed to upload DJ list to Azure Blob Storage"

* Verify your Azure Storage access key or SAS token is correct
* Check storage account permissions
* Ensure the container exists: `az storage container show --name your-container --account-name your-account`

### "Failed to send email via Mailgun"

* Verify your Mailgun API key is correct
* For sandbox domains, ensure recipient email is authorized
* Check Mailgun domain is correct (include `.mailgun.org` for sandbox)
* Review Mailgun logs at <https://app.mailgun.com/logs>

### "Unable to authenticate with Azure Storage"

Configure Azure credentials using one of:

```bash
# Option 1: Access Key (in .env file)
AZURE_STORAGE_ACCESS_KEY=your-access-key

# Option 2: SAS Token (in .env file)
AZURE_STORAGE_SAS_TOKEN=your-sas-token

# Option 3: Managed Identity (recommended for Azure deployments)
# No credentials needed - automatically handled by Azure SDK
```

## Development

### Running in Development Mode

```bash
cargo run
```

### Running Tests

```bash
cargo test
```

### Checking for Updates

```bash
cargo update
cargo build
```

## Cost Estimates

### Azure Blob Storage

* Storage: < $0.01/month (one small JSON file, Hot tier)
* Requests: < $0.01/month (24-48 operations/day)

### Mailgun

* Free Tier: 5, 000 emails/month (more than enough for DJ alerts)
* Assuming 1-2 alerts/day = ~60 emails/month = **FREE**

### Total Estimated Cost

**$0.02 - $0.05/month** (essentially free!)

## License

MIT

## Contributing

Pull requests welcome! For major changes, please open an issue first.

## Support

For issues or questions:

1. Check the [Troubleshooting](#troubleshooting) section
2. Review [DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md) for deployment configuration
3. Open an issue on GitHub
