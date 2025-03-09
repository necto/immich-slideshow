use image_server_lib::ImmichConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;
use reqwest;
use anyhow::Result;

mod mock_immich_server;

struct TestEnv {
    test_dir: TempDir,
    originals_dir: PathBuf,
    images_dir: PathBuf,
    style_dir: PathBuf,
    test_image_path: PathBuf,
    immich_url: String,
    api_key: String,
    album_id: String,
    asset_id: String,
}

impl ImmichConfig for TestEnv {
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

async fn setup_test_environment() -> Result<TestEnv> {
    // Create temporary directories
    let test_dir = TempDir::new()?;
    let originals_dir = test_dir.path().join("originals");
    let images_dir = test_dir.path().join("images");
    let style_dir = test_dir.path().join("style");
    
    fs::create_dir_all(&originals_dir)?;
    fs::create_dir_all(&images_dir)?;
    fs::create_dir_all(&style_dir)?;
    
    // Create a test image (a small colored square)
    let test_image_path = test_dir.path().join("test_image.txt");
    // AI! write "mock data" into the file by `test_image_path`

    // Use a simple style image
    let style_image_path = style_dir.join("style.jpg");
    fs::copy(&test_image_path, &style_image_path)?;
    
    // Test configuration
    let album_id = "test-album-123";
    let asset_id = "test-asset-456";
    let api_key = "test-api-key";
    
    Ok(TestEnv {
        test_dir,
        originals_dir,
        images_dir,
        style_dir,
        test_image_path,
        immich_url: String::new(), // Will be filled after server starts
        api_key: api_key.to_string(),
        album_id: album_id.to_string(),
        asset_id: asset_id.to_string(),
    })
}

#[tokio::test]
async fn test_end_to_end_flow() -> Result<()> {
    // Setup test environment
    let mut test_env = setup_test_environment().await?;
    
    // Start mock server
    let mock_server_addr = mock_immich_server::start_mock_server(
        &test_env.album_id,
        &test_env.asset_id,
        test_env.test_image_path.to_str().unwrap(),
        8000
    ).await?;
    
    test_env.immich_url = format!("http://{}", mock_server_addr);
    println!("Mock server started at: {}", test_env.immich_url);
    
    // Configure and run immich-fetcher
    let originals_dir_str = test_env.originals_dir.to_str().unwrap();
    
    // Run immich-fetcher with the mock server
    let immich_fetcher_status = Command::new("cargo")
        .args(&[
            "run", 
            "--bin", "immich-fetcher", 
            "--", 
            "--immich-url", &test_env.immich_url,
            "--api-key", &test_env.api_key,
            "--album-id", &test_env.album_id,
            "--originals-dir", originals_dir_str,
            "--max-images", "10"
        ])
        .status()?;
    
    assert!(immich_fetcher_status.success(), "immich-fetcher failed");
    
    // Verify the image was downloaded to originals directory
    let downloaded_files = fs::read_dir(&test_env.originals_dir)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    
    assert!(!downloaded_files.is_empty(), "No files were downloaded to originals directory");
    println!("Downloaded file: {:?}", downloaded_files[0]);
    
    // For simplicity, use dummy_convert_image.sh for the transformer
    let conversion_script = Path::new("conversion/dummy_convert_image.sh");
    
    // Run image-transformer manually (simplified version)
    for file in &downloaded_files {
        let output_file = test_env.images_dir.join(
            file.file_name().unwrap()
        );
        
        let transformer_status = Command::new("sh")
            .arg(conversion_script)
            .arg(file)
            .arg(&output_file)
            .status()?;
        
        assert!(transformer_status.success(), "image-transformer failed");
    }
    
    // Verify images were created in the images directory
    let transformed_files = fs::read_dir(&test_env.images_dir)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    
    assert!(!transformed_files.is_empty(), "No files were created in images directory");
    println!("Transformed file: {:?}", transformed_files[0]);
    
    // Start the image-server in the background
    let images_dir_str = test_env.images_dir.to_str().unwrap();
    let server_child = Command::new("cargo")
        .args(&[
            "run", 
            "--bin", "image-server", 
            "--", 
            "--image-dir", images_dir_str,
        ])
        .spawn()?;
    
    // Give the server time to start
    thread::sleep(Duration::from_secs(2));
    
    // Test that we can access the image from the server
    let client = reqwest::Client::new();
    let response = client.get("http://localhost:8080/image").send().await?;
    
    assert!(response.status().is_success(), "Failed to get image from server");
    let image_bytes = response.bytes().await?;
    assert!(!image_bytes.is_empty(), "Image response was empty");
    
    // Clean up the running server
    server_child.kill()?;
    
    // Test successful
    println!("End-to-end test completed successfully!");
    Ok(())
}
