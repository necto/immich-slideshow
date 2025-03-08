use anyhow::{Context, Result};
use clap::Parser;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, event::RemoveKind};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;
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
    watch_for_new_files(rx, args)?;
    
    Ok(())
}

fn process_existing_files(args: &Args) -> Result<()> {
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
    
    println!("Found {} existing files to process", files.len());
    
    // Process each file
    for file_path in &files {
        process_file(file_path, args)?;
    }
    
    println!("Successfully processed {} existing images", files.len());
    
    Ok(())
}

fn watch_for_new_files(rx: Receiver<Result<Event, notify::Error>>, args: Args) -> Result<()> {
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

fn process_file(file_path: &Path, args: &Args) -> Result<()> {
    let file_name = file_path.file_name()
        .context("Invalid file path")?
        .to_string_lossy();
        
    // Generate output filename with same name but PNG extension
    let file_stem = Path::new(&*file_name).file_stem()
        .context("Failed to get file stem")?
        .to_string_lossy();
        
    let output_filename = format!("{}.png", file_stem);
    let output_path = format!("{}/{}", args.output_dir, output_filename);
    
    // Check if output file already exists
    if Path::new(&output_path).exists() {
        println!("Output file already exists, skipping: {}", output_path);
        return Ok(());
    }
    
    // Convert the image to grayscale PNG
    convert_to_grayscale(file_path.to_string_lossy().as_ref(), &output_path)
        .with_context(|| format!("Failed to convert asset {} to grayscale", file_stem))?;
        
    println!("Converted to grayscale: {}", output_path);
    
    Ok(())
}

/// Convert an image to grayscale PNG using a bash script that invokes ImageMagick
fn convert_to_grayscale(input_path: &str, output_path: &str) -> Result<()> {
    let status = Command::new("bash")
        .arg("convert_image.sh")
        .arg(input_path)
        .arg(output_path)
        .status()
        .context("Failed to execute conversion script. Is the script available and executable?")?;
        
    if !status.success() {
        anyhow::bail!("Conversion script failed with exit code: {}", status);
    }
    
    Ok(())
}

/// Handle a file that has been removed from the originals directory
fn handle_removed_file(file_path: &Path, args: &Args) -> Result<()> {
    // Factor out file_path -> output_path logic AI!
    let file_name = file_path.file_name()
        .context("Invalid file path")?
        .to_string_lossy();

    // Generate the corresponding output filename
    let file_stem = Path::new(&*file_name).file_stem()
        .context("Failed to get file stem")?
        .to_string_lossy();
        
    let output_filename = format!("{}.png", file_stem);
    let output_path = format!("{}/{}", args.output_dir, output_filename);
    
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
