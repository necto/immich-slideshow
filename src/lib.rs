use anyhow::{Context, Result};
use reqwest::{Client, header};

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

// Import from image_transformer
use std::sync::mpsc::Receiver;
use notify::{Event, EventKind, event::RemoveKind};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
struct AlbumResponse {
    pub assets: Vec<Asset>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Asset {
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

pub trait TransformerConfig {
    fn originals_dir(&self) -> &str;
    fn transformed_dir(&self) -> &str;
    fn conversion_script(&self) -> &str;
}

async fn fetch_album_asset_list<T: ImmichConfig>(client: &Client, config: &T) -> Result<Vec<Asset>> {
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

async fn download_asset<T: ImmichConfig>(client: &Client, config: &T, asset_id: &str, output_path: &str) -> Result<()> {
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

pub fn process_existing_files<T: TransformerConfig>(args: &T) -> Result<()> {
    // Get list of files to process
    let entries = fs::read_dir(&args.originals_dir())
        .context("Failed to read originals directory")?;

    let files = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<PathBuf>>();

    println!("Found {} existing files to process", files.len());

    // Process each file
    for file_path in &files {
        process_file(file_path, args)?;
    }

    println!("Successfully processed {} existing images", files.len());

    Ok(())
}

pub fn handle_file_system_events<T: TransformerConfig>(rx: Receiver<Result<Event, notify::Error>>, args: T) -> Result<()> {
    // Process events from the watcher
    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                match event.kind {
                    // Handle file creation or modification events
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            if path.is_file() {
                                println!("New file detected: {:?}", path);
                                match process_file(&path, &args) {
                                    Ok(_) => println!("Successfully processed new file"),
                                    Err(e) => eprintln!("Error processing file: {}", e),
                                }
                            }
                        }
                    },
                    // Handle file removal events
                    EventKind::Remove(RemoveKind::File) => {
                        for path in event.paths {
                            println!("File removed: {:?}", path);
                            match handle_removed_file(&path, &args) {
                                Ok(_) => println!("Successfully handled removed file"),
                                Err(e) => eprintln!("Error handling removed file: {}", e),
                            }
                        }
                    },
                    _ => {} // Ignore other event types
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {:?}", e),
            Err(e) => {
                eprintln!("Channel error: {:?}", e);
                // Sleep to avoid tight loop in case of errors
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

/// Get the output path for a given input file path
fn get_output_path(file_path: &Path, output_dir: &str) -> Result<String> {
    let file_name = file_path.file_name()
        .context("Invalid file path")?
        .to_string_lossy();

    // Generate output filename with same name but PNG extension
    let file_stem = Path::new(&*file_name).file_stem()
        .context("Failed to get file stem")?
        .to_string_lossy();

    let output_filename = format!("{}.png", file_stem);
    Ok(format!("{}/{}", output_dir, output_filename))
}

fn process_file<T: TransformerConfig>(file_path: &Path, args: &T) -> Result<()> {
    let output_path = get_output_path(file_path, &args.transformed_dir())?;

    // Check if output file already exists
    if Path::new(&output_path).exists() {
        println!("Output file already exists, skipping: {}", output_path);
        return Ok(());
    }

    // Convert the image to grayscale PNG
    convert_image(
        file_path.to_string_lossy().as_ref(),
        &output_path,
        &args.conversion_script()
    )
    .with_context(|| format!("Failed to convert asset {} to grayscale",
                             file_path.to_string_lossy()))?;

    println!("Converted to grayscale: {}", output_path);

    Ok(())
}

/// Convert an image to grayscale PNG using a bash script that invokes ImageMagick
fn convert_image(input_path: &str, output_path: &str, script_path: &str) -> Result<()> {
    let status = Command::new("bash")
        .arg(script_path)
        .arg(input_path)
        .arg(output_path)
        .status()
        .with_context(|| format!("Failed to execute conversion script '{}'. Is the script available and executable?", script_path))?;

    if !status.success() {
        anyhow::bail!("Conversion script failed with exit code: {}", status);
    }

    Ok(())
}

/// Handle a file that has been removed from the originals directory
fn handle_removed_file<T: TransformerConfig>(file_path: &Path, args: &T) -> Result<()> {
    let output_path = get_output_path(file_path, &args.transformed_dir())?;

    // Check if the output file exists
    if Path::new(&output_path).exists() {
        println!("Removing corresponding output file: {}", output_path);
        fs::remove_file(&output_path)
            .with_context(|| format!("Failed to remove output file: {}", output_path))?;
    } else {
        println!("No corresponding output file found for: {:?}", file_path);
    }

    Ok(())
}
