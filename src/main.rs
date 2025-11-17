use anyhow::{Context, Result};
use azure_storage::StorageCredentials;
use azure_storage_blobs::prelude::*;
use clap::Parser;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::time::Duration;
use tokio::time;

#[derive(Parser, Debug)]
#[command(author, version, about = "Monitor inflyteapp.com URLs for DJ changes", long_about = None)]
struct Args {
    /// The inflyteapp.com URLs to monitor (comma-separated or multiple --url flags)
    #[arg(short, long, value_delimiter = ',', num_args = 1..)]
    url: Vec<String>,
}

#[derive(Debug, Clone)]
struct Campaign {
    url: String,
    name: String,
}

#[derive(Debug, Clone)]
struct Config {
    campaigns: Vec<Campaign>,
    storage_account: String,
    storage_container: String,
    blob_name_prefix: String,
    storage_credentials: StorageCredentials,
    mailgun_api_key: String,
    mailgun_domain: String,
    recipient_email: String,
    from_email: String,
    check_interval_minutes: u64,
}

impl Config {
    fn from_env(urls: Vec<String>) -> Result<Self> {
        dotenv::dotenv().ok();

        let storage_account = env::var("AZURE_STORAGE_ACCOUNT")
            .context("AZURE_STORAGE_ACCOUNT environment variable not set")?;

        let storage_credentials = if let Ok(access_key) = env::var("AZURE_STORAGE_ACCESS_KEY") {
            StorageCredentials::access_key(storage_account.clone(), access_key)
        } else if let Ok(sas_token) = env::var("AZURE_STORAGE_SAS_TOKEN") {
            StorageCredentials::sas_token(sas_token)?
        } else {
            anyhow::bail!("Either AZURE_STORAGE_ACCESS_KEY or AZURE_STORAGE_SAS_TOKEN must be set")
        };

        // Create campaign objects with extracted names
        let campaigns = urls
            .into_iter()
            .map(|url| {
                let name = extract_campaign_name(&url);
                Campaign { url, name }
            })
            .collect();

        Ok(Config {
            campaigns,
            storage_account,
            storage_container: env::var("AZURE_STORAGE_CONTAINER")
                .unwrap_or_else(|_| "inflyte-dj-monitor".to_string()),
            blob_name_prefix: env::var("AZURE_BLOB_NAME_PREFIX")
                .unwrap_or_else(|_| "dj_list".to_string()),
            storage_credentials,
            mailgun_api_key: env::var("MAILGUN_API_KEY")
                .context("MAILGUN_API_KEY environment variable not set")?,
            mailgun_domain: env::var("MAILGUN_DOMAIN")
                .context("MAILGUN_DOMAIN environment variable not set")?,
            recipient_email: env::var("RECIPIENT_EMAIL")
                .context("RECIPIENT_EMAIL environment variable not set")?,
            from_email: env::var("FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@inflyte.com".to_string()),
            check_interval_minutes: env::var("CHECK_INTERVAL_MINUTES")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("CHECK_INTERVAL_MINUTES must be a valid number")?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DjStorage {
    djs: HashSet<String>,
}

/// Extract campaign name from URL (e.g., https://inflyteapp.com/r/pmqtne -> pmqtne)
fn extract_campaign_name(url: &str) -> String {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Get blob name for a campaign
fn get_blob_name(config: &Config, campaign: &Campaign) -> String {
    format!("{}_{}.json", config.blob_name_prefix, campaign.name)
}

/// Fetch the webpage and extract DJ names from the Support section
async fn fetch_dj_list(url: &str) -> Result<HashSet<String>> {
    let response = reqwest::get(url)
        .await
        .context("Failed to fetch webpage")?
        .text()
        .await
        .context("Failed to read response text")?;

    let document = Html::parse_document(&response);

    let mut djs = HashSet::new();
    let mut in_support_section = false;

    for element in document.select(&Selector::parse("*").unwrap()) {
        // Check if we've hit a Support header
        if element.value().name() == "h3" {
            let text = element.text().collect::<String>();
            if text.trim() == "Support" {
                in_support_section = true;
                continue;
            } else if in_support_section {
                // We've hit another h3, so we're out of the Support section
                break;
            }
        }

        // If we're in the support section, extract DJ names
        if in_support_section && element.text().next().is_some() {
            let text = element.text().collect::<String>();
            if !text.trim().is_empty() && text.contains("Support from") {
                // Parse the DJ names (they're separated by commas and "and")
                let names_part = text.replace("Support from", "").replace(" and ", ", ");

                for name in names_part.split(',') {
                    let cleaned = name.trim();
                    if !cleaned.is_empty() && !cleaned.starts_with("Get Mad") {
                        djs.insert(cleaned.to_string());
                    }
                }
                break; // We found the support list
            }
        }
    }

    Ok(djs)
}

/// Load the previously saved DJ list from Azure Blob Storage
async fn load_previous_djs(config: &Config, campaign: &Campaign) -> Result<HashSet<String>> {
    let container_client = BlobServiceClient::new(
        config.storage_account.clone(),
        config.storage_credentials.clone(),
    )
    .container_client(&config.storage_container);

    let blob_name = get_blob_name(config, campaign);
    let blob_client = container_client.blob_client(&blob_name);

    match blob_client.get_content().await {
        Ok(content) => {
            let content_str =
                String::from_utf8(content).context("Failed to parse blob content as UTF-8")?;
            let storage: DjStorage =
                serde_json::from_str(&content_str).context("Failed to parse DJ storage JSON")?;
            Ok(storage.djs)
        }
        Err(_) => {
            // Blob doesn't exist yet (first run)
            Ok(HashSet::new())
        }
    }
}

/// Save the current DJ list to Azure Blob Storage
async fn save_djs(config: &Config, campaign: &Campaign, djs: &HashSet<String>) -> Result<()> {
    let storage = DjStorage { djs: djs.clone() };
    let json = serde_json::to_string_pretty(&storage).context("Failed to serialize DJ list")?;

    let container_client = BlobServiceClient::new(
        config.storage_account.clone(),
        config.storage_credentials.clone(),
    )
    .container_client(&config.storage_container);

    let blob_name = get_blob_name(config, campaign);
    let blob_client = container_client.blob_client(&blob_name);

    let bytes = json.into_bytes();
    blob_client
        .put_block_blob(bytes)
        .content_type("application/json")
        .await
        .context("Failed to upload DJ list to Azure Blob Storage")?;

    Ok(())
}

/// Send email notification via Mailgun API
async fn send_email_alert(config: &Config, campaign: &Campaign, new_djs: &[&String]) -> Result<()> {
    let dj_list = new_djs
        .iter()
        .map(|dj| format!("  • {}", dj))
        .collect::<Vec<_>>()
        .join("\n");

    let subject = format!(
        "🚨 {} New DJ{} Added to Inflyte Campaign '{}'",
        new_djs.len(),
        if new_djs.len() == 1 { "" } else { "s" },
        campaign.name
    );

    let html_body = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
        .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px; border-radius: 8px 8px 0 0; }}
        .content {{ background: #f9f9f9; padding: 20px; border-radius: 0 0 8px 8px; }}
        .dj-list {{ background: white; padding: 15px; border-left: 4px solid #667eea; margin: 15px 0; }}
        .dj-item {{ margin: 8px 0; }}
        .campaign {{ color: #667eea; font-weight: bold; }}
        .footer {{ text-align: center; margin-top: 20px; color: #666; font-size: 12px; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🎵 Inflyte DJ Monitor Alert</h1>
        </div>
        <div class="content">
            <p><strong>New DJs have been added to the Support section!</strong></p>
            <p class="campaign">Campaign: {}</p>
            <div class="dj-list">
                <h3>New Additions ({})</h3>
{}
            </div>
            <p>View the full list at: <a href="{}">{}</a></p>
        </div>
        <div class="footer">
            <p>This is an automated notification from your Inflyte DJ Monitor</p>
        </div>
    </div>
</body>
</html>"#,
        campaign.name,
        new_djs.len(),
        new_djs
            .iter()
            .map(|dj| format!("                <div class=\"dj-item\">✨ {}</div>", dj))
            .collect::<Vec<_>>()
            .join("\n"),
        &campaign.url,
        &campaign.url
    );

    let text_body = format!(
        "🚨 New DJs detected on Inflyte!\n\nCampaign: {}\n\n{}\n\nTotal new additions: {}\n\nView at: {}",
        campaign.name,
        dj_list,
        new_djs.len(),
        &campaign.url
    );

    let client = reqwest::Client::new();
    let mailgun_url = format!(
        "https://api.mailgun.net/v3/{}/messages",
        config.mailgun_domain
    );

    let form = reqwest::multipart::Form::new()
        .text("from", config.from_email.clone())
        .text("to", config.recipient_email.clone())
        .text("subject", subject)
        .text("text", text_body)
        .text("html", html_body);

    let response = client
        .post(&mailgun_url)
        .basic_auth("api", Some(&config.mailgun_api_key))
        .multipart(form)
        .send()
        .await
        .context("Failed to send email via Mailgun")?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Mailgun API error: {}", error_text)
    }
}

/// Check for new DJs and send alerts
async fn check_for_new_djs(config: &Config, campaign: &Campaign) -> Result<()> {
    println!("Checking {} for new DJs...", campaign.name);

    let current_djs = fetch_dj_list(&campaign.url).await?;
    let previous_djs = load_previous_djs(config, campaign).await?;

    if previous_djs.is_empty() {
        println!(
            "Initial run for {} - found {} DJs",
            campaign.name,
            current_djs.len()
        );
        println!("Current DJs: {:?}", current_djs);
        save_djs(config, campaign, &current_djs).await?;
        println!("✅ Saved initial DJ list for {}", campaign.name);
        return Ok(());
    } else {
        let new_djs: Vec<_> = current_djs.difference(&previous_djs).collect();

        if !new_djs.is_empty() {
            println!("\n🚨 ALERT: New DJs detected for {}!", campaign.name);
            println!("═══════════════════════════════");
            for dj in &new_djs {
                println!("  ✨ {}", dj);
            }
            println!("═══════════════════════════════\n");

            // Send email notification
            if let Err(e) = send_email_alert(config, campaign, &new_djs).await {
                eprintln!("Failed to send email alert: {}", e);
            } else {
                println!("✅ Email notification sent to {}", config.recipient_email);
            }
        } else {
            println!(
                "No new DJs found for {}. Total: {}",
                campaign.name,
                current_djs.len()
            );
        }

        save_djs(config, campaign, &current_djs).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    if args.url.is_empty() {
        anyhow::bail!("At least one URL must be provided via --url");
    }

    println!("🎵 Inflyte DJ Monitor Starting...");
    println!("Monitoring {} campaign(s):\n", args.url.len());

    // Load configuration from environment variables
    let config = Config::from_env(args.url)?;

    println!("Configuration:");
    println!("  Azure Storage Account: {}", config.storage_account);
    println!("  Azure Container: {}", config.storage_container);
    println!("  Blob Name Prefix: {}", config.blob_name_prefix);
    println!("  Email To: {}", config.recipient_email);
    println!("  Email From: {}", config.from_email);
    println!("  Mailgun Domain: {}", config.mailgun_domain);
    println!(
        "  Check Interval: {} minutes\n",
        config.check_interval_minutes
    );

    println!("Campaigns:");
    for campaign in &config.campaigns {
        println!("  • {} ({})", campaign.name, campaign.url);
    }
    println!();

    println!("Azure Blob Storage configured\n");

    // Run initial check for all campaigns
    for campaign in &config.campaigns {
        if let Err(e) = check_for_new_djs(&config, campaign).await {
            eprintln!("Error during check for {}: {}", campaign.name, e);
        }
    }

    // Set up periodic checks
    let mut interval = time::interval(Duration::from_secs(config.check_interval_minutes * 60));
    interval.tick().await; // First tick completes immediately

    loop {
        interval.tick().await;
        for campaign in &config.campaigns {
            if let Err(e) = check_for_new_djs(&config, campaign).await {
                eprintln!("Error during check for {}: {}", campaign.name, e);
            }
        }
    }
}
