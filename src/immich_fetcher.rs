use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Client;
use std::fs;
use std::path::Path;
use std::time::Duration;
use std::thread;
use dotenv::dotenv;
use image_server_lib::{
    ImmichConfig,
    fetch_and_download_images,
};

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

impl ImmichConfig for Args {
    fn immich_url(&self) -> &str {
        &self.immich_url
    }

    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn album_id(&self) -> &str {
        &self.album_id
    }
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
        match fetch_and_download_images(&client, &args, &args.originals_dir, args.max_images).await {
            Ok(_) => println!("Fetch cycle completed successfully"),
            Err(e) => eprintln!("Error during fetch cycle: {}", e),
        }
        
        // Wait for 1 minute before the next fetch
        println!("Waiting 60 seconds before next fetch...");
        thread::sleep(Duration::from_secs(60));
    }
}
