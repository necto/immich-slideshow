use anyhow::{Context, Result};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

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

// Trait to abstract the API configuration
pub trait ImmichConfig {
    fn immich_url(&self) -> &str;
    fn api_key(&self) -> &str;
    fn album_id(&self) -> &str;
}

pub async fn fetch_album_asset_list<T: ImmichConfig>(client: &Client, config: &T) -> Result<Vec<Asset>> {
    let url = format!("{}/api/albums/{}?withoutAssets=false",
                      config.immich_url(), config.album_id());
    
    let response = client.get(url)
        .header(header::ACCEPT, "application/json")
        .header("x-api-key", config.api_key())
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

pub async fn download_asset<T: ImmichConfig>(client: &Client, config: &T, asset_id: &str, output_path: &str) -> Result<()> {
    let url = format!("{}/api/assets/{}/original", config.immich_url(), asset_id);
    
    let response = client.get(url)
        .header(header::ACCEPT, "application/octet-stream")
        .header("x-api-key", config.api_key())
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

pub async fn fetch_and_download_images<T: ImmichConfig>(
    client: &Client,
    args: &T,
    originals_dir: &str,
    max_images: usize
) -> Result<()> {
    // Fetch assets from album
    let assets = fetch_album_asset_list(client, args).await?;
    println!("Found {} assets in album", assets.len());

    // Create a set of current asset IDs for quick lookup
    let current_asset_ids: std::collections::HashSet<String> = assets
        .iter()
        .take(max_images)
        .map(|asset| asset.id.clone())
        .collect();

    // Check for files to remove (files that are no longer in the album)
    let removed_count = remove_deleted_assets(originals_dir, &current_asset_ids)?;
    if removed_count > 0 {
        println!("Removed {} assets that are no longer in the album", removed_count);
    }

    // Download assets
    let mut downloaded_count = 0;
    for (i, asset) in assets.iter().enumerate() {
        if i >= max_images {
            break;
        }

        let original_path = format!("{}/{}--_--{}",
                                  originals_dir,
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
    println!("Originals saved to: {}", originals_dir);

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
