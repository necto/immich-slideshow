use anyhow::{Context, Result};
use clap::Parser;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;
use std::thread;
use dotenv::dotenv;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Immich API URL
    #[arg(long, env("IMMICH_URL"))]
    immich_url: String,

    /// Immich API key
    #[arg(long, env("IMMICH_API_KEY"))]
    api_key: String,

    /// Album ID to fetch images from
    #[arg(long, env("IMMICH_ALBUM_ID"))]
    album_id: String,

    /// Directory to save original images to
    #[arg(long, default_value = "originals")]
    originals_dir: String,

    /// Maximum number of images to fetch
    #[arg(long, default_value = "100")]
    max_images: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenv().ok();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Create directories if they don't exist
    if !Path::new(&args.originals_dir).exists() {
        fs::create_dir_all(&args.originals_dir)
            .context("Failed to create originals directory")?;
    }
    
    // Initialize HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    println!("Starting continuous fetcher service");
    println!("Args: {:?}", args);
    println!("Will check for new images every minute");
    
    // Run continuously
    loop {
        match fetch_and_download_images(&client, &args).await {
            Ok(_) => println!("Fetch cycle completed successfully"),
            Err(e) => eprintln!("Error during fetch cycle: {}", e),
        }
        
        // Wait for 1 minute before the next fetch
        println!("Waiting 60 seconds before next fetch...");
        thread::sleep(Duration::from_secs(60));
    }
}

async fn fetch_and_download_images(client: &Client, args: &Args) -> Result<()> {
    // Fetch assets from album
    let assets = fetch_album_asset_list(client, args).await?;
    println!("Found {} assets in album", assets.len());

    // Create a set of current asset IDs for quick lookup
    let current_asset_ids: std::collections::HashSet<String> = assets
        .iter()
        .take(args.max_images)
        .map(|asset| asset.id.clone())
        .collect();

    // Check for files to remove (files that are no longer in the album)
    let removed_count = remove_deleted_assets(&args.originals_dir, &current_asset_ids)?;
    if removed_count > 0 {
        println!("Removed {} assets that are no longer in the album", removed_count);
    }

    // Download assets
    let mut downloaded_count = 0;
    for (i, asset) in assets.iter().enumerate() {
        if i >= args.max_images {
            break;
        }

        let original_path = format!("{}/{}--_--{}",
                                  args.originals_dir,
                                  asset.id,
                                  asset.original_file_name);
        
        // Skip if file already exists
        if Path::new(&original_path).exists() {
            println!("Asset {} already exists, skipping", asset.id);
            continue;
        }

        download_asset(client, args, &asset.id, &original_path).await
            .with_context(|| format!("Failed to download asset {}", asset.id))?;

        println!("Downloaded asset {} to {}", asset.id, original_path);
        downloaded_count += 1;
    }

    if downloaded_count > 0 {
        println!("Successfully downloaded {} new images", downloaded_count);
    } else {
        println!("No new images to download");
    }
    println!("Originals saved to: {}", args.originals_dir);

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlbumResponse {
    pub assets: Vec<Asset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    #[serde(rename = "type")]
    pub asset_type: String,
    pub checksum: String,
    #[serde(rename = "originalFileName")]
    pub original_file_name: String,
}

async fn fetch_album_asset_list(client: &Client, args: &Args) -> Result<Vec<Asset>> {
    let url = format!("{}/api/albums/{}?withoutAssets=false",
                      args.immich_url, args.album_id);
    
    let response = client.get(url)
        .header(header::ACCEPT, "application/json")
        .header("x-api-key", &args.api_key)
        .send()
        .await?;
        
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("Failed to fetch album assets: HTTP {}: {}", status, text);
    }

    let resp: AlbumResponse = response.json().await?;
    Ok(resp.assets)
}

async fn download_asset(client: &Client, args: &Args, asset_id: &str, output_path: &str) -> Result<()> {
    let url = format!("{}/api/assets/{}/original", args.immich_url, asset_id);
    
    let response = client.get(url)
        .header(header::ACCEPT, "application/octet-stream")
        .header("x-api-key", &args.api_key)
        .send()
        .await?;
        
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("Failed to download asset: HTTP {}: {}", status, text);
    }
    
    let bytes = response.bytes().await?;
    fs::write(output_path, bytes)?;
    
    Ok(())
}

/// Removes files from the originals directory that are no longer in the album
fn remove_deleted_assets(originals_dir: &str, current_asset_ids: &std::collections::HashSet<String>) -> Result<usize> {
    let entries = fs::read_dir(originals_dir)
        .context("Failed to read originals directory")?;
    
    let mut removed_count = 0;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if !path.is_file() {
            continue;
        }
        
        // Extract asset ID from filename (format is "{asset_id}--_--{original_filename}")
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            if let Some(separator_pos) = filename.find("--_--") {
                let asset_id = &filename[0..separator_pos];
                
                // If this asset is no longer in the album, remove it
                if !current_asset_ids.contains(asset_id) {
                    println!("Removing asset {} as it's no longer in the album", asset_id);
                    fs::remove_file(&path)
                        .with_context(|| format!("Failed to remove file: {:?}", path))?;
                    removed_count += 1;
                }
            }
        }
    }
    
    Ok(removed_count)
}
