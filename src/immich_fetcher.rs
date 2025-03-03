use anyhow::{Context, Result};
use clap::Parser;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;
use dotenv::dotenv;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Immich API URL
    #[arg(long, env = "IMMICH_URL")]
    immich_url: String,

    /// Immich API key
    #[arg(long, env = "IMMICH_API_KEY")]
    api_key: String,

    /// Album ID to fetch images from
    #[arg(long, env = "IMMICH_ALBUM_ID")]
    album_id: String,

    /// Directory to save images to
    #[arg(long, default_value = "images")]
    output_dir: String,

    /// Maximum number of images to fetch
    #[arg(long, default_value = "100")]
    max_images: usize,
}

#[derive(Debug, Deserialize)]
struct Asset {
    id: String,
    #[serde(rename = "originalPath")]
    original_path: String,
}

#[derive(Debug, Serialize)]
struct AlbumAssetsRequest {
    #[serde(rename = "albumId")]
    album_id: String,
    skip: usize,
    take: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenv().ok();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Create output directory if it doesn't exist
    if !Path::new(&args.output_dir).exists() {
        fs::create_dir_all(&args.output_dir)
            .context("Failed to create output directory")?;
    }
    
    // Initialize HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    
    // Fetch assets from album
    let assets = fetch_album_assets(&client, &args).await?;
    println!("Found {} assets in album", assets.len());
    
    // Download assets
    for (i, asset) in assets.iter().enumerate() {
        if i >= args.max_images {
            break;
        }
        
        let extension = Path::new(&asset.original_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg");
            
        let output_path = format!("{}/{}.{}", args.output_dir, asset.id, extension);
        
        download_asset(&client, &args, &asset.id, &output_path).await
            .with_context(|| format!("Failed to download asset {}", asset.id))?;
            
        println!("Downloaded asset {} to {}", asset.id, output_path);
    }
    
    println!("Successfully downloaded {} images to {}", 
        assets.len().min(args.max_images), 
        args.output_dir);
    
    Ok(())
}

async fn fetch_album_assets(client: &Client, args: &Args) -> Result<Vec<Asset>> {
    let url = format!("{}/api/album/assets", args.immich_url);
    
    let request_body = AlbumAssetsRequest {
        album_id: args.album_id.clone(),
        skip: 0,
        take: args.max_images,
    };
    
    let response = client.post(url)
        .header(header::ACCEPT, "application/json")
        .header("x-api-key", &args.api_key)
        .json(&request_body)
        .send()
        .await?;
        
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("Failed to fetch album assets: HTTP {}: {}", status, text);
    }
    
    let assets: Vec<Asset> = response.json().await?;
    Ok(assets)
}

async fn download_asset(client: &Client, args: &Args, asset_id: &str, output_path: &str) -> Result<()> {
    let url = format!("{}/api/asset/file/{}", args.immich_url, asset_id);
    
    let response = client.get(url)
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
