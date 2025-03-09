use anyhow::Context;
use notify::{Event, EventKind, event::RemoveKind, Config, RecommendedWatcher, Watcher, RecursiveMode};
use std::cmp::min;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Receiver;
use std::time::Duration;

pub trait TransformerConfig {
    fn originals_dir(&self) -> &str;
    fn transformed_dir(&self) -> &str;
    fn conversion_script(&self) -> &str;
}

pub fn process_existing_files<T: TransformerConfig>(args: &T) -> anyhow::Result<()> {
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

fn handle_file_system_events<T: TransformerConfig>(
    rx: Receiver<anyhow::Result<Event, notify::Error>>,
    args: &T,
    timeout_ms: Option<u64>
) -> anyhow::Result<()> {
    let start_time = std::time::Instant::now();

    // Process events from the watcher
    loop {
        // Check if we've exceeded the timeout
        let wait_time_remaining = if let Some(timeout_ms) = timeout_ms {
            let elapsed = start_time.elapsed().as_millis() as u64;
            if timeout_ms <= elapsed {
                println!("Timeout reached, exiting watcher");
                break;
            }
            min(1000, timeout_ms - elapsed)
        } else {
            1000
        };

        // Try to receive an event, but with a short timeout to let us check the overall timeout
        match rx.recv_timeout(Duration::from_millis(wait_time_remaining)) {
            Ok(Ok(event)) => {
                match event.kind {
                    // Handle file creation or modification events
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            if path.is_file() {
                                println!("New file detected: {:?}", path);
                                match process_file(&path, args) {
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
                            match handle_removed_file(&path, args) {
                                Ok(_) => println!("Successfully handled removed file"),
                                Err(e) => eprintln!("Error handling removed file: {}", e),
                            }
                        }
                    },
                    _ => {} // Ignore other event types
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {:?}", e),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Just a timeout on the recv, continue the loop
                continue;
            },
            Err(e) => {
                eprintln!("Channel error: {:?}", e);
                return Err(e.into());
            }
        }
    }
    Ok(())
}

/// Get the output path for a given input file path
fn get_output_path(file_path: &Path, output_dir: &str) -> anyhow::Result<String> {
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

fn process_file<T: TransformerConfig>(file_path: &Path, args: &T) -> anyhow::Result<()> {
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
fn convert_image(input_path: &str, output_path: &str, script_path: &str) -> anyhow::Result<()> {
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
fn handle_removed_file<T: TransformerConfig>(file_path: &Path, args: &T) -> anyhow::Result<()> {
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

/// Sets up a file watcher with a timeout for testing
pub fn run_file_watcher_with_timeout<T: TransformerConfig>(
    args: &T,
    timeout_ms: Option<u64>
) -> anyhow::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .context("Failed to create file watcher")?;
    
    // Start watching the directory
    watcher.watch(Path::new(args.originals_dir()), RecursiveMode::NonRecursive)
        .context("Failed to watch directory")?;

    if let Some(tout) = timeout_ms {
        println!("Watching for new files with timeout of {}ms...", tout);
    } else {
        println!("Watching for new files...");
    }

    handle_file_system_events(rx, args, timeout_ms)
}
