use anyhow::{Context, Result};
use clap::Parser;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
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

    /// Directory to save converted images to
    #[arg(long, default_value = "images")]
    output_dir: String,

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
    
    if !Path::new(&args.output_dir).exists() {
        fs::create_dir_all(&args.output_dir)
            .context("Failed to create output directory")?;
    }
    
    // Initialize HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    println!("Args: {:?}", args);
    // Fetch assets from album
    let assets = fetch_album_asset_list(&client, &args).await?;
    println!("Found {} assets in album", assets.len());

    // Download assets
    for (i, asset) in assets.iter().enumerate() {
        if i >= args.max_images {
            break;
        }

        let original_path = format!("{}/{}-{}",
                                  args.originals_dir,
                                  asset.id,
                                  asset.original_file_name);

        download_asset(&client, &args, &asset.id, &original_path).await
            .with_context(|| format!("Failed to download asset {}", asset.id))?;

        println!("Downloaded asset {} to {}", asset.id, original_path);
        
        // Generate output filename (PNG)
        let output_filename = format!("{}.png", asset.id);
        let output_path = format!("{}/{}", args.output_dir, output_filename);
        
        // Convert the image to grayscale PNG
        convert_to_grayscale(&original_path, &output_path)
            .with_context(|| format!("Failed to convert asset {} to grayscale", asset.id))?;
            
        println!("Converted to grayscale: {}", output_path);
    }

    println!("Successfully processed {} images",
        assets.len().min(args.max_images));
    println!("Originals saved to: {}", args.originals_dir);
    println!("Converted images saved to: {}", args.output_dir);

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
/// Convert an image to grayscale PNG using ImageMagick
fn convert_to_grayscale(input_path: &str, output_path: &str) -> Result<()> {
    let status = Command::new("convert")
        .arg(input_path)
        .arg("-colorspace")
        .arg("Gray")
        .arg("-depth")
        .arg("8")
        .arg("-resize")
        .arg("1072x1448^")
        .arg("-gravity")
        .arg("center")
        .arg("-crop")
        .arg("1072x1448+0+0")
        .arg("+repage")
        .arg(output_path)
        .status()
        .context("Failed to execute convert command. Is ImageMagick installed?")?;
        
    if !status.success() {
        anyhow::bail!("Convert command failed with exit code: {}", status);
    }
    
    Ok(())
}
