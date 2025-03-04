use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use dotenv::dotenv;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory containing original images
    #[arg(long, default_value = "originals")]
    originals_dir: String,

    /// Directory to save converted images to
    #[arg(long, default_value = "images")]
    output_dir: String,
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
    
    // Get list of files to process
    let entries = fs::read_dir(&args.originals_dir)
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
    
    println!("Found {} files to process", files.len());
    
    // Process each file
    for file_path in &files {
        let file_name = file_path.file_name()
            .context("Invalid file path")?
            .to_string_lossy();
            
        // Generate output filename with same name but PNG extension
        let file_stem = Path::new(&*file_name).file_stem()
            .context("Failed to get file stem")?
            .to_string_lossy();
            
        let output_filename = format!("{}.png", file_stem);
        let output_path = format!("{}/{}", args.output_dir, output_filename);
        
        // Convert the image to grayscale PNG
        convert_to_grayscale(file_path.to_string_lossy().as_ref(), &output_path)
            .with_context(|| format!("Failed to convert asset {} to grayscale", file_stem))?;
            
        println!("Converted to grayscale: {}", output_path);
    }
    
    println!("Successfully processed {} images", files.len());
    println!("Converted images saved to: {}", args.output_dir);
    
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
