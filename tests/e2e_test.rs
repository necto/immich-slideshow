use image_server_lib::ImmichConfig;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Child};
use tempfile::TempDir;
use reqwest;
use anyhow::Result;

mod mock_immich_server;

struct ManagedChild {
    cmdline: String,
    child: Option<Child>, // Use Option to ensure we can drop the process safely
}

impl ManagedChild {
    fn new(command: &str, args: &[&str]) -> Result<Self> {
        let cmdline = format!("{} {}", command, args.join(" "));
        let child = Command::new(command).args(args).spawn()?;
        Ok(ManagedChild { cmdline, child: Some(child) })
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            println!("Terminating child process '{}'", self.cmdline);
            if let Err(e) = child.kill() {
                eprintln!("Failed to kill child process: {}", e);
            }
            let _ = child.wait(); // Clean up resources
        }
    }
}

struct TestEnv {
    // Keep the temporary directory alive for the duration of the test
    #[allow(dead_code)]
    test_dir: TempDir,
    originals_dir: PathBuf,
    images_dir: PathBuf,
    test_image_path: PathBuf,
    test_image_data: Vec<u8>,
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

    fs::create_dir_all(&originals_dir)?;
    fs::create_dir_all(&images_dir)?;

    // Create a test image file with mock data
    let test_image_data = "mock image data for testing purposes";
    let test_image_path = test_dir.path().join("test_image.txt");
    fs::write(&test_image_path, test_image_data)?;

    // Test configuration
    let album_id = "test-album-123";
    let asset_id = "test-asset-456";
    let api_key = "test-api-key";
    
    Ok(TestEnv {
        test_dir,
        originals_dir,
        images_dir,
        test_image_path,
        test_image_data: test_image_data.as_bytes().to_vec(),
        immich_url: String::new(), // Will be filled after server starts
        api_key: api_key.to_string(),
        album_id: album_id.to_string(),
        asset_id: asset_id.to_string(),
    })
}

#[actix_web::test]
async fn test_end_to_end_flow() -> Result<()> {
    // Setup test environment
    let mut test_env = setup_test_environment().await?;

    // Start mock server
    let mock_server_addr = mock_immich_server::start_mock_server(
        &test_env.album_id,
        &test_env.asset_id,
        test_env.test_image_path.to_str().unwrap()
    ).await?;

    // Wait for the server to start.
    // For some reason if I press on immedeately, the server just stalls.
    actix_rt::time::sleep(std::time::Duration::from_secs(1)).await;
    
    test_env.immich_url = format!("http://{}", mock_server_addr);
    println!("Mock server started at: {}", test_env.immich_url);
    
    // Configure and run immich-fetcher
    let originals_dir_str = test_env.originals_dir.to_str().unwrap();

    // Make sure the binaries are built to avoid delays when I need to run them
    let build_fetcher = Command::new("cargo").args(&["build", "--bin", "immich-fetcher"]).output()?;
    assert!(build_fetcher.status.success(), "Failed to build immich-fetcher");
    let build_transformer = Command::new("cargo").args(&["build", "--bin", "image-transformer"]).output()?;
    assert!(build_transformer.status.success(), "Failed to build image-transformer");
    let build_server = Command::new("cargo").args(&["build", "--bin", "image-server"]).output()?;
    assert!(build_server.status.success(), "Failed to build image-server");

    // Run all three services in parallel in the background
    let _immich_fetcher = ManagedChild::new("cargo", &[
        "run",
        "--bin", "immich-fetcher",
        "--",
        "--immich-url", test_env.immich_url.as_str(),
        "--api-key", test_env.api_key.as_str(),
        "--album-id", test_env.album_id.as_str(),
        "--originals-dir", originals_dir_str,
        "--max-images", "10"
    ])?;

    let _transformer = ManagedChild::new("cargo", &[
        "run",
        "--bin", "image-transformer",
        "--",
        "--originals-dir", originals_dir_str,
        "--output-dir", test_env.images_dir.to_str().unwrap(),
        "--conversion-script", "conversion/dummy_convert_image.sh",
    ])?;

    let _image_server = ManagedChild::new("cargo", &[
        "run",
        "--bin", "image-server",
        "--",
        "--image-dir", test_env.images_dir.to_str().unwrap(),
    ])?;
    // Let them all start
    actix_rt::time::sleep(std::time::Duration::from_secs(1)).await;

    // Verify the image was downloaded to originals directory
    let downloaded_files = fs::read_dir(&test_env.originals_dir)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    
    assert!(!downloaded_files.is_empty(), "No files were downloaded to originals directory");
    println!("Downloaded file: {:?}", downloaded_files[0]);

    // Verify images were created in the images directory
    let transformed_files = fs::read_dir(&test_env.images_dir)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;

    assert!(!transformed_files.is_empty(), "No files were created in images directory");
    println!("Transformed file: {:?}", transformed_files[0]);

    // Test that we can access the image from the server
    let client = reqwest::Client::new();
    let response = client.get("http://localhost:8080/image").send().await?;

    assert!(response.status().is_success(), "Failed to get image from server");
    let image_bytes = response.bytes().await?;
    assert!(!image_bytes.is_empty(), "Image response was empty");
    assert_eq!(image_bytes, test_env.test_image_data, "Image content mismatch");

    // Test successful
    println!("End-to-end test completed successfully!");
    Ok(())
}
