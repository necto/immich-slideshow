use anyhow::Result;
use image_server_lib::image_transformer_lib::{TransformerConfig, process_existing_files, run_file_watcher_with_timeout};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

struct TransformerArgs {
    originals_dir: String,
    transformed_dir: String,
    conversion_script: String,
}

impl TransformerConfig for TransformerArgs {
    fn originals_dir(&self) -> &str {
        &self.originals_dir
    }

    fn transformed_dir(&self) -> &str {
        &self.transformed_dir
    }

    fn conversion_script(&self) -> &str {
        &self.conversion_script
    }
}

#[test]
fn test_process_existing_files() -> Result<()> {
    // Create temporary directories for the test
    let temp_dir = tempdir()?;
    let originals_dir = temp_dir.path().join("originals");
    let output_dir = temp_dir.path().join("output");
    
    fs::create_dir_all(&originals_dir)?;
    fs::create_dir_all(&output_dir)?;
    
    // Create some test image files
    let test_files = vec!["test1.jpg", "test2.jpg", "test3.jpg"];
    for filename in &test_files {
        let file_path = originals_dir.join(filename);
        let mut file = File::create(&file_path)?;
        write!(file, "Test image content")?;
    }
    
    // Set up arguments using the dummy conversion script
    let args = TransformerArgs {
        originals_dir: originals_dir.to_string_lossy().to_string(),
        transformed_dir: output_dir.to_string_lossy().to_string(),
        conversion_script: "conversion/dummy_convert_image.sh".to_string(),
    };
    
    // Run the function being tested
    process_existing_files(&args)?;

    // Verify that output files were created correctly
    // First, read all files from both directories
    let output_entries = fs::read_dir(&output_dir)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    
    // Count the number of files in the output directory
    let output_file_count = output_entries.len();
    
    // Check that we have the correct number of output files
    assert_eq!(output_file_count, test_files.len(), 
               "Number of output files ({}) does not match number of input files ({})",
               output_file_count, test_files.len());

    // Verify each output file corresponds to an original file
    for output_path in output_entries {
        if let Some(output_filename) = output_path.file_name() {
            if let Some(output_stem) = output_path.file_stem() {
                let output_stem_str = output_stem.to_string_lossy();
                let found_match = test_files.iter().any(|orig_name| {
                    let orig_stem = Path::new(orig_name).file_stem().unwrap().to_string_lossy();
                    orig_stem == output_stem_str
                });
                
                assert!(found_match, "Output file {:?} doesn't correspond to any input file", output_filename);
            }
        }
    }

    Ok(())
}

#[test]
fn test_run_file_watcher_removes_files() -> Result<()> {
    // Create temporary directories for the test
    let temp_dir = tempdir()?;
    let originals_dir = temp_dir.path().join("originals");
    let output_dir = temp_dir.path().join("output");
    
    fs::create_dir_all(&originals_dir)?;
    fs::create_dir_all(&output_dir)?;
    
    // Set up arguments using the dummy conversion script
    let args = TransformerArgs {
        originals_dir: originals_dir.to_string_lossy().to_string(),
        transformed_dir: output_dir.to_string_lossy().to_string(),
        conversion_script: "conversion/dummy_convert_image.sh".to_string(),
    };
    
    // Start the file watcher in a separate thread with a longer timeout
    let originals_dir_clone = originals_dir.clone();
    let watcher_handle = std::thread::spawn(move || {
        run_file_watcher_with_timeout(&args, Some(2000)).unwrap(); // 2 second timeout
    });
    
    // Sleep briefly to let the watcher initialize
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    // Create a new test file to trigger the watcher
    let test_file_path = originals_dir_clone.join("test_remove.jpg");
    {
        let mut file = File::create(&test_file_path)?;
        write!(file, "Test image content")?;
    }
    
    // Give it some time to process the creation
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Check that the output file was created
    let expected_output = output_dir.join("test_remove.png");
    assert!(expected_output.exists(), "Output file was not created by watcher");
    
    // Remove the original file
    fs::remove_file(&test_file_path)?;
    
    // Give it some time to process the deletion
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Check that the output file was also removed
    assert!(!expected_output.exists(), "Output file was not removed when original was deleted");
    
    // Wait for watcher thread to finish
    watcher_handle.join().expect("Watcher thread panicked");
    
    Ok(())
}

#[test]
fn test_run_file_watcher() -> Result<()> {
    // Create temporary directories for the test
    let temp_dir = tempdir()?;
    let originals_dir = temp_dir.path().join("originals");
    let output_dir = temp_dir.path().join("output");
    
    fs::create_dir_all(&originals_dir)?;
    fs::create_dir_all(&output_dir)?;
    
    // Set up arguments using the dummy conversion script
    let args = TransformerArgs {
        originals_dir: originals_dir.to_string_lossy().to_string(),
        transformed_dir: output_dir.to_string_lossy().to_string(),
        conversion_script: "conversion/dummy_convert_image.sh".to_string(),
    };
    
    // Start the file watcher in a separate thread with a short timeout
    let originals_dir_clone = originals_dir.clone();
    let watcher_handle = std::thread::spawn(move || {
        run_file_watcher_with_timeout(&args, Some(1000)).unwrap(); // 1 second timeout
    });
    
    // Sleep briefly to let the watcher initialize
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    // Create a new test file to trigger the watcher
    let test_file = originals_dir_clone.join("test_watch.jpg");
    let mut file = File::create(&test_file)?;
    write!(file, "Test image content")?;
    
    // Give it some time to process
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Check that the output file was created
    let expected_output = output_dir.join("test_watch.png");
    assert!(expected_output.exists(), "Output file was not created by watcher");
    
    // Wait for watcher thread to finish
    watcher_handle.join().expect("Watcher thread panicked");
    
    Ok(())
}
