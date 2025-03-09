use anyhow::Result;
use image_server_lib::{TransformerConfig, process_existing_files};
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

    // AI! change this to check that every file in the output_dir corresponds to a file in the originals_dir and it has the same number of files
    // Verify that output files were created
    for filename in &test_files {
        let base_name = Path::new(filename).file_stem().unwrap();
        let output_filename = format!("{}.png", base_name.to_string_lossy());
        let output_path = output_dir.join(&output_filename);
        
        assert!(output_path.exists(), "Output file {} not created", output_filename);
    }
    
    Ok(())
}
