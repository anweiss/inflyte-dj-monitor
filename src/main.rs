use anyhow::{Context, Result};
use axum::{Router, extract::State, response::Json, routing::get};
use azure_storage::StorageCredentials;
use azure_storage_blobs::prelude::*;
use clap::Parser;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, warn};

#[derive(Parser, Debug)]
#[command(author, version, about = "Monitor inflyteapp.com URLs for DJ changes", long_about = None)]
struct Args {
    /// The inflyteapp.com URLs to monitor (comma-separated or multiple --url flags)
    #[arg(short, long, value_delimiter = ',', num_args = 0..)]
    url: Vec<String>,

    /// Path to a file containing URLs to monitor (one URL per line, # for comments)
    #[arg(short, long)]
    file: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
struct Campaign {
    url: String,
    name: String,
    track_title: Option<String>,
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
    http_port: u16,
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
                Campaign {
                    url,
                    name,
                    track_title: None,
                }
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
            http_port: env::var("HTTP_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("HTTP_PORT must be a valid number")?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
struct DjSupport {
    name: String,
    comment: Option<String>,
    stars: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DjStorage {
    djs: HashSet<DjSupport>,
}

/// Extract campaign name from URL (e.g., https://inflyteapp.com/r/pmqtne -> pmqtne)
fn extract_campaign_name(url: &str) -> String {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Read URLs from a file, ignoring comments and blank lines
fn read_urls_from_file(path: &PathBuf) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read URL file: {}", path.display()))?;

    let urls: Vec<String> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect();

    if urls.is_empty() {
        anyhow::bail!("No valid URLs found in file: {}", path.display());
    }

    Ok(urls)
}

/// Extract track artist and title from the webpage
async fn fetch_track_title(url: &str) -> Option<String> {
    use std::time::Duration;

    debug!(url = %url, "Fetching track title");

    // Create a client with timeout
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "Failed to create HTTP client");
            return None;
        }
    };

    // Fetch the page with timeout
    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            warn!(url = %url, error = %e, "Failed to fetch page");
            return None;
        }
    };

    let text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            warn!(error = %e, "Failed to read response text");
            return None;
        }
    };

    let document = Html::parse_document(&text);

    // Look for h1 tag which typically contains "Artist - Track Title"
    let h1_selector = match Selector::parse("h1") {
        Ok(s) => s,
        Err(e) => {
            warn!(error = ?e, "Failed to parse h1 selector");
            return None;
        }
    };

    for element in document.select(&h1_selector) {
        let text = element.text().collect::<String>();
        let trimmed = text.trim();

        // Skip if it's empty or looks like a navigation element
        if !trimmed.is_empty() && trimmed.contains('-') && !trimmed.contains("Inflyte") {
            debug!(title = %trimmed, "Found track title");
            return Some(trimmed.to_string());
        }
    }

    debug!("No track title found in h1 tags");
    None
}

/// Get blob name for a campaign
fn get_blob_name(config: &Config, campaign: &Campaign) -> String {
    format!("{}_{}.json", config.blob_name_prefix, campaign.name)
}

/// Fetch the webpage and extract DJ names, comments, and star ratings from the Support section
async fn fetch_dj_list(url: &str) -> Result<HashSet<DjSupport>> {
    let response = reqwest::get(url)
        .await
        .context("Failed to fetch webpage")?
        .text()
        .await
        .context("Failed to read response text")?;

    let document = Html::parse_document(&response);

    let mut djs = HashSet::new();

    // First pass: look for DJs with individual profile sections (name + comment + stars)
    // These appear as structured elements with img, name, and comment
    let h3_selector = Selector::parse("h3").unwrap();

    for h3 in document.select(&h3_selector) {
        let text = h3.text().collect::<String>();
        if text.trim() == "Support" {
            // Look at the next siblings after the Support h3
            if let Some(mut next_element) = h3.next_sibling() {
                loop {
                    // Check if we've hit another h3 or end of support section
                    if let Some(_element_ref) = next_element.value().as_element()
                        && _element_ref.name() == "h3"
                    {
                        break;
                    }

                    // Look for individual DJ profile sections
                    // These contain an img tag followed by name and comment text
                    if next_element.value().as_element().is_some() {
                        let elem = scraper::ElementRef::wrap(next_element).unwrap();

                        // Check if this element or its children contain an image (DJ profile pic)
                        let img_selector = Selector::parse("img").unwrap();
                        if elem.select(&img_selector).next().is_some() {
                            // Find all individual DJ profile entries within this container
                            // Each DJ entry should have its own image element
                            // Look for direct children or nested divs that each contain exactly one img
                            let div_selector = Selector::parse("div").unwrap();
                            let divs_with_img: Vec<_> = elem
                                .select(&div_selector)
                                .filter(|div| {
                                    // This div has an img and represents a single DJ entry
                                    div.select(&img_selector).next().is_some()
                                })
                                .collect();

                            // If we found individual DJ containers, process each one
                            if !divs_with_img.is_empty() {
                                // Find the innermost divs that contain exactly one img
                                // (to avoid processing parent containers that contain multiple profiles)
                                let innermost_divs: Vec<_> = divs_with_img
                                    .iter()
                                    .filter(|div| {
                                        // Count how many divs with images are nested inside this one
                                        let nested_count = div
                                            .select(&div_selector)
                                            .filter(|inner| {
                                                inner.select(&img_selector).next().is_some()
                                            })
                                            .count();
                                        // If no nested divs with images, this is an innermost DJ entry
                                        nested_count == 0
                                    })
                                    .collect();

                                for dj_div in innermost_divs {
                                    let full_text = dj_div.text().collect::<String>();
                                    let lines: Vec<&str> = full_text
                                        .lines()
                                        .map(|l| l.trim())
                                        .filter(|l| !l.is_empty())
                                        .collect();

                                    if lines.len() >= 1 {
                                        // Extract DJ name (first line before any emoji/stars)
                                        let name_line = lines[0];
                                        let name = name_line
                                            .split('⭐')
                                            .next()
                                            .unwrap_or(name_line)
                                            .trim()
                                            .to_string();

                                        // Extract comment (subsequent lines that aren't "Support from")
                                        let mut comment_parts = Vec::new();
                                        for line in lines.iter().skip(1) {
                                            if line.starts_with("Support from") {
                                                break;
                                            }
                                            comment_parts.push(*line);
                                        }
                                        let comment_text =
                                            comment_parts.join(" ").trim().to_string();

                                        // Count stars
                                        let stars = full_text.matches('⭐').count() as u8;

                                        if !name.is_empty() && name.len() < 100 {
                                            djs.insert(DjSupport {
                                                name,
                                                comment: if !comment_text.is_empty() {
                                                    Some(comment_text)
                                                } else {
                                                    None
                                                },
                                                stars: if stars > 0 { Some(stars) } else { None },
                                            });
                                        }
                                    }
                                }
                            } else {
                                // Fallback: no nested divs found, process as single entry
                                let full_text = elem.text().collect::<String>();
                                let lines: Vec<&str> = full_text
                                    .lines()
                                    .map(|l| l.trim())
                                    .filter(|l| !l.is_empty())
                                    .collect();

                                if lines.len() >= 2 {
                                    // Extract DJ name (first line before any emoji/stars)
                                    let name_line = lines[0];
                                    let name = name_line
                                        .split('⭐')
                                        .next()
                                        .unwrap_or(name_line)
                                        .trim()
                                        .to_string();

                                    // Extract comment (subsequent lines that aren't "Support from")
                                    let mut comment_parts = Vec::new();
                                    for line in &lines[1..] {
                                        if line.starts_with("Support from") {
                                            break;
                                        }
                                        comment_parts.push(*line);
                                    }
                                    let comment_text = comment_parts.join(" ").trim().to_string();

                                    // Count stars
                                    let stars = full_text.matches('⭐').count() as u8;

                                    if !name.is_empty() && name.len() < 100 {
                                        djs.insert(DjSupport {
                                            name,
                                            comment: if !comment_text.is_empty() {
                                                Some(comment_text)
                                            } else {
                                                None
                                            },
                                            stars: if stars > 0 { Some(stars) } else { None },
                                        });
                                    }
                                }
                            }
                        }

                        // Also check for "Support from" list in this element
                        let text = elem.text().collect::<String>();
                        if text.contains("Support from") {
                            // Extract the list of supporting DJs
                            let after_support = text.split("Support from").nth(1).unwrap_or("");

                            // Split by common delimiters
                            let normalized = after_support.replace(" and ", ", ");
                            let names: Vec<String> = normalized
                                .split(',')
                                .map(|s| s.trim())
                                .filter(|s| {
                                    !s.is_empty()
                                        && !s.starts_with("Get Mad")
                                        && !s.starts_with("Currently subscribed")
                                        && s.len() < 100
                                })
                                .map(|s| s.to_string())
                                .collect();

                            for name_str in names {
                                // Only add if it doesn't already exist (avoid duplicates)
                                if !djs.iter().any(|dj| dj.name == name_str) {
                                    djs.insert(DjSupport {
                                        name: name_str,
                                        comment: None,
                                        stars: None,
                                    });
                                }
                            }
                        }
                    }

                    // Move to next sibling
                    if let Some(next) = next_element.next_sibling() {
                        next_element = next;
                    } else {
                        break;
                    }
                }
            }
            break;
        }
    }

    Ok(djs)
}

/// Load the previously saved DJ list from Azure Blob Storage
async fn load_previous_djs(config: &Config, campaign: &Campaign) -> Result<HashSet<DjSupport>> {
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

            // Try to parse as new format first
            if let Ok(storage) = serde_json::from_str::<DjStorage>(&content_str) {
                Ok(storage.djs)
            } else {
                // Try to migrate from old format (HashSet<String>)
                #[derive(Deserialize)]
                struct OldDjStorage {
                    djs: HashSet<String>,
                }

                if let Ok(old_storage) = serde_json::from_str::<OldDjStorage>(&content_str) {
                    info!(
                        "Migrating old DJ storage format to new format with comment/rating support"
                    );
                    let migrated: HashSet<DjSupport> = old_storage
                        .djs
                        .into_iter()
                        .map(|name| DjSupport {
                            name,
                            comment: None,
                            stars: None,
                        })
                        .collect();
                    Ok(migrated)
                } else {
                    anyhow::bail!("Failed to parse DJ storage JSON in either old or new format")
                }
            }
        }
        Err(_) => {
            // Blob doesn't exist yet (first run)
            Ok(HashSet::new())
        }
    }
}

/// Save the current DJ list to Azure Blob Storage
async fn save_djs(config: &Config, campaign: &Campaign, djs: &HashSet<DjSupport>) -> Result<()> {
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
async fn send_email_alert(
    config: &Config,
    campaign: &Campaign,
    new_djs: &[&DjSupport],
) -> Result<()> {
    let dj_list = new_djs
        .iter()
        .map(|dj| {
            let mut line = format!("  • {}", dj.name);
            if let Some(stars) = dj.stars {
                line.push_str(&format!(" ({}⭐)", "⭐".repeat(stars as usize)));
            }
            if let Some(comment) = &dj.comment {
                line.push_str(&format!(" - \"{}\"", comment));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n");

    let campaign_display = campaign.track_title.as_ref().unwrap_or(&campaign.name);

    let subject = format!(
        "🚨 {} New DJ{} {} for {}",
        new_djs.len(),
        if new_djs.len() == 1 { "" } else { "s" },
        if new_djs
            .iter()
            .any(|dj| dj.comment.is_some() || dj.stars.is_some())
        {
            "Support/Comment"
        } else {
            "Added"
        },
        campaign_display
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
            <p class="campaign">Track: {}</p>
            <div class="dj-list">
                <h3>New Support ({})</h3>
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
        campaign_display,
        new_djs.len(),
        new_djs
            .iter()
            .map(|dj| {
                let mut entry = format!(
                    "                <div class=\"dj-item\"><strong>✨ {}</strong>",
                    dj.name
                );
                if let Some(stars) = dj.stars {
                    entry.push_str(&format!(
                        " <span style=\"color: #FFD700;\">{}</span>",
                        "⭐".repeat(stars as usize)
                    ));
                }
                if let Some(comment) = &dj.comment {
                    entry.push_str(&format!(
                        "<br/><em style=\"color: #666; margin-left: 20px;\">\"{}\"{}</em>",
                        comment, "</div>"
                    ));
                } else {
                    entry.push_str("</div>");
                }
                entry
            })
            .collect::<Vec<_>>()
            .join("\n"),
        &campaign.url,
        &campaign.url
    );

    let text_body = format!(
        "🚨 New DJ support detected on Inflyte!\n\nTrack: {}\n\n{}\n\nTotal new additions: {}\n\nView at: {}",
        campaign_display,
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

/// Shared application state for HTTP server
#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    campaign_stats: Arc<RwLock<Vec<CampaignStats>>>,
}

/// Campaign statistics for HTTP endpoint
#[derive(Debug, Clone, Serialize)]
struct CampaignStats {
    name: String,
    url: String,
    track_title: Option<String>,
    dj_count: usize,
    last_checked: Option<String>,
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Get current campaigns being monitored
async fn get_campaigns(State(state): State<AppState>) -> Json<serde_json::Value> {
    let stats = state.campaign_stats.read().await;
    Json(serde_json::json!({
        "status": "active",
        "campaigns": stats.clone(),
        "total_campaigns": stats.len(),
        "check_interval_minutes": state.config.check_interval_minutes,
    }))
}

/// Update campaign statistics
async fn update_campaign_stats(
    state: &AppState,
    campaign: &Campaign,
    dj_count: usize,
) -> Result<()> {
    let mut stats = state.campaign_stats.write().await;

    // Find existing stat or create new one
    if let Some(stat) = stats.iter_mut().find(|s| s.name == campaign.name) {
        stat.dj_count = dj_count;
        stat.last_checked = Some(chrono::Utc::now().to_rfc3339());
    } else {
        stats.push(CampaignStats {
            name: campaign.name.clone(),
            url: campaign.url.clone(),
            track_title: campaign.track_title.clone(),
            dj_count,
            last_checked: Some(chrono::Utc::now().to_rfc3339()),
        });
    }

    Ok(())
}

/// Start HTTP server
async fn start_http_server(state: AppState, port: u16) {
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/campaigns", get(get_campaigns))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind HTTP server");

    info!(address = %addr, "HTTP server listening");
    info!("Health endpoint: http://{}/health", addr);
    info!("Campaigns endpoint: http://{}/campaigns", addr);

    axum::serve(listener, app)
        .await
        .expect("HTTP server failed");
}

/// Check for new DJs and send alerts
async fn check_for_new_djs(
    config: &Config,
    campaign: &Campaign,
    state: Option<&AppState>,
) -> Result<()> {
    info!(campaign = %campaign.name, "Checking for new DJs");

    let current_djs = fetch_dj_list(&campaign.url).await?;
    let previous_djs = load_previous_djs(config, campaign).await?;

    if previous_djs.is_empty() {
        info!(
            campaign = %campaign.name,
            count = current_djs.len(),
            "Initial run - found DJs"
        );
        debug!(djs = ?current_djs, "Current DJs");
        save_djs(config, campaign, &current_djs).await?;
        info!(campaign = %campaign.name, "Saved initial DJ list");

        // Update campaign stats
        if let Some(state) = state {
            update_campaign_stats(state, campaign, current_djs.len()).await?;
        }

        return Ok(());
    } else {
        let new_djs: Vec<_> = current_djs.difference(&previous_djs).collect();

        if !new_djs.is_empty() {
            info!(
                campaign = %campaign.name,
                count = new_djs.len(),
                "🚨 ALERT: New DJ support detected!"
            );
            for dj in &new_djs {
                let mut line = format!("✨ {}", dj.name);
                if let Some(stars) = dj.stars {
                    line.push_str(&format!(" {}", "⭐".repeat(stars as usize)));
                }
                if let Some(comment) = &dj.comment {
                    line.push_str(&format!(" - \"{}\"", comment));
                }
                info!("{}", line);
            }

            // Send email notification
            if let Err(e) = send_email_alert(config, campaign, &new_djs).await {
                error!(error = %e, "Failed to send email alert");
            } else {
                info!(recipient = %config.recipient_email, "Email notification sent");
            }
        } else {
            info!(
                campaign = %campaign.name,
                total = current_djs.len(),
                "No new DJs found"
            );

            // Debug: Show a few examples of what we're tracking
            if !current_djs.is_empty() {
                debug!("Sample of tracked DJs:");
                for (i, dj) in current_djs.iter().take(5).enumerate() {
                    let mut line = format!("{}. {}", i + 1, dj.name);
                    if let Some(stars) = dj.stars {
                        line.push_str(&format!(" ({}⭐)", stars));
                    }
                    if let Some(comment) = &dj.comment {
                        line.push_str(&format!(" - \"{}\"", comment));
                    }
                    debug!("{}", line);
                }
            }
        }

        save_djs(config, campaign, &current_djs).await?;

        // Update campaign stats
        if let Some(state) = state {
            update_campaign_stats(state, campaign, current_djs.len()).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Parse command-line arguments
    let args = Args::parse();

    debug!("Parsed arguments");

    // Collect URLs from both command-line args and file
    let mut urls = args.url.clone();

    if let Some(file_path) = &args.file {
        debug!(path = ?file_path, "Reading URLs from file");
        let file_urls = read_urls_from_file(file_path)?;
        urls.extend(file_urls);
    }

    // If no URLs provided via args, try INFLYTE_URLS environment variable
    if urls.is_empty()
        && let Ok(env_urls) = env::var("INFLYTE_URLS")
    {
        debug!(env_urls = %env_urls, "Reading URLs from INFLYTE_URLS environment variable");
        let env_url_list: Vec<String> = env_urls
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        urls.extend(env_url_list);
    }

    // Remove duplicates while preserving order
    let mut seen = HashSet::new();
    urls.retain(|url| seen.insert(url.clone()));

    debug!(count = urls.len(), "Total URLs collected");

    if urls.is_empty() {
        anyhow::bail!("At least one URL must be provided via --url or --file");
    }

    info!("🎵 Inflyte DJ Monitor Starting");
    info!(count = urls.len(), "Monitoring campaigns");

    debug!("Loading configuration from environment");

    // Load configuration from environment variables
    let mut config = Config::from_env(urls)?;

    debug!("Configuration loaded successfully");

    info!("Configuration:");
    info!("  Azure Storage Account: {}", config.storage_account);
    info!("  Azure Container: {}", config.storage_container);
    info!("  Blob Name Prefix: {}", config.blob_name_prefix);
    info!("  Email To: {}", config.recipient_email);
    info!("  Email From: {}", config.from_email);
    info!("  Mailgun Domain: {}", config.mailgun_domain);
    info!(
        "  Check Interval: {} minutes",
        config.check_interval_minutes
    );

    debug!(
        campaigns = config.campaigns.len(),
        "Fetching track information"
    );

    // Fetch track titles for all campaigns
    info!("Fetching track information");
    for campaign in &mut config.campaigns {
        debug!(url = %campaign.url, "Fetching title");
        if let Some(title) = fetch_track_title(&campaign.url).await {
            campaign.track_title = Some(title);
        }
    }

    debug!("Track information fetched");

    info!("Campaigns:");
    for campaign in &config.campaigns {
        if let Some(title) = &campaign.track_title {
            info!("  • {} ({})", title, campaign.url);
        } else {
            info!("  • {} ({})", campaign.name, campaign.url);
        }
    }

    info!("Azure Blob Storage configured");

    debug!("Creating application state");

    // Create shared application state
    let app_state = AppState {
        config: Arc::new(config.clone()),
        campaign_stats: Arc::new(RwLock::new(Vec::new())),
    };

    debug!(port = config.http_port, "Starting HTTP server");

    // Start HTTP server in background
    let http_port = config.http_port;
    let server_state = app_state.clone();
    tokio::spawn(async move {
        start_http_server(server_state, http_port).await;
    });

    debug!("HTTP server spawned");

    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    debug!(campaigns = config.campaigns.len(), "Running initial checks");

    // Run initial check for all campaigns
    for campaign in &config.campaigns {
        debug!(campaign = %campaign.name, "Checking campaign");
        if let Err(e) = check_for_new_djs(&config, campaign, Some(&app_state)).await {
            error!(campaign = %campaign.name, error = %e, "Error during check");
        }
    }

    debug!("Initial checks complete, starting periodic loop");

    // Set up periodic checks
    let mut interval = time::interval(Duration::from_secs(config.check_interval_minutes * 60));
    interval.tick().await; // First tick completes immediately

    info!("Entering main monitoring loop");

    loop {
        interval.tick().await;
        debug!("Running periodic check");
        for campaign in &config.campaigns {
            if let Err(e) = check_for_new_djs(&config, campaign, Some(&app_state)).await {
                error!(campaign = %campaign.name, error = %e, "Error during check");
            }
        }
    }
}
