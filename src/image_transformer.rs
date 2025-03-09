use anyhow::{Context, Result};
use clap::Parser;
use notify::{Config, RecommendedWatcher, Watcher, RecursiveMode};
use std::fs;
use std::path::Path;
use std::sync::mpsc::channel;
use dotenv::dotenv;
use image_server_lib::{TransformerConfig, process_existing_files, handle_file_system_events};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Directory containing original images
    #[arg(long, default_value = "originals")]
    originals_dir: String,

    /// Directory to save converted images to
    #[arg(long, default_value = "images")]
    output_dir: String,
    
    /// Path to the conversion script
    #[arg(long, env = "CONVERSION_SCRIPT", default_value = "convert_image.sh")]
    conversion_script: String,
}

impl TransformerConfig for Args {
    fn originals_dir(&self) -> &str {
        &self.originals_dir
    }

    fn transformed_dir(&self) -> &str {
        &self.output_dir
    }

    fn conversion_script(&self) -> &str {
        &self.conversion_script
    }
}

fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenv().ok();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Create output directory if it doesn't exist
    if !Path::new(&args.output_dir).exists() {
        fs::create_dir_all(&args.output_dir)
            .context("Failed to create output directory")?;
    }
    
    // Create originals directory if it doesn't exist
    if !Path::new(&args.originals_dir).exists() {
        fs::create_dir_all(&args.originals_dir)
            .context("Failed to create originals directory")?;
    }
    
    println!("Starting continuous transformer service");
    println!("Watching for new files in: {}", args.originals_dir);
    println!("Converting images to: {}", args.output_dir);
    
    // Process existing files first
    process_existing_files(&args)?;
    
    // Set up file watcher
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .context("Failed to create file watcher")?;
    
    // Start watching the originals directory
    watcher.watch(Path::new(&args.originals_dir), RecursiveMode::NonRecursive)
        .context("Failed to watch directory")?;
    
    println!("Watching for new files...");
    
    // Process events
    handle_file_system_events(rx, args)?;
    
    Ok(())
}
