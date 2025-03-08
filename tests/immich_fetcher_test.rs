use std::fs;
use serde_json::json;
use std::path::Path;
use mockito::Server;
use tempfile::tempdir;
use image_server_lib::{ImmichConfig, fetch_and_download_images};

#[tokio::test]
async fn test_download_asset() -> anyhow::Result<()> {
    // Create a temporary directory for test files
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Setup mock server
    let mut server = Server::new_async().await;
    let mock_server_url = server.url();
    
    // Mock the album endpoint
    let album_id = "test-album-id";
    let asset_id = "test-asset-id";

    let album_response = json!({
        "id": &album_id,
        "name": "Test Album",
        "description": "This is a test album",
        "createdAt": "2021-01-01T00:00:00Z",
        "updatedAt": "2021-01-01T00:00:00Z",
        "assets": [
            {
                "id": &asset_id,
                "type": "IMAGE",
                "checksum": "abc123",
                "originalFileName": "test-image.jpg"
            }
        ]
    });

    // Setup album endpoint mock
    let _album_mock = server.mock("GET", format!("/api/albums/{}?withoutAssets=false", album_id).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(album_response.to_string())
        .create();
    
    // Setup asset download endpoint mock
    let test_image_content = b"fake image data";
    let _asset_mock = server.mock("GET", format!("/api/assets/{}/original", asset_id).as_str())
        .with_status(200)
        .with_header("content-type", "application/octet-stream")
        .with_body(test_image_content)
        .create();
    
    // Create a reqwest client
    let client = reqwest::Client::new();
    
    // Create args struct with our test values
    let args = TestArgs {
        immich_url: mock_server_url,
        api_key: "test-api-key".to_string(),
        album_id: album_id.to_string(),
    };
    let max_images = 10;
    let originals_dir = temp_path.clone();

    fetch_and_download_images(
        &client,
        &args,
        &originals_dir,
        max_images
    ).await.expect("success");

    // Check that the directory contains exactly one file
    let entries = fs::read_dir(&temp_path)
        .expect("Failed to read temp directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to collect directory entries");
    
    assert_eq!(entries.len(), 1, "Directory should contain exactly one file");
    
    // Get the file path
    let file_path = entries[0].path();
    
    // Verify the file was downloaded correctly
    assert!(file_path.exists());
    let downloaded_content = fs::read(&file_path).expect("Failed to read downloaded file");
    assert_eq!(downloaded_content, test_image_content);
    
    Ok(())
}

// Helper struct to mimic the Args struct from the main code
struct TestArgs {
    immich_url: String,
    api_key: String,
    album_id: String,
}

// Implement the ImmichConfig trait for TestArgs
impl ImmichConfig for TestArgs {
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

#[tokio::test]
async fn test_remove_deleted_assets() -> anyhow::Result<()> {
    // Create a temporary directory for test files
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Setup mock server
    let mut server = Server::new_async().await;
    let mock_server_url = server.url();
    
    // Mock the album endpoint
    let album_id = "test-album-id";
    let asset_id = "test-asset-id";
    let removed_asset_id = "removed-asset-id";

    // Create a test file that should be removed (simulating a file from a previous fetch)
    let removed_file_path = format!("{}/{}--_--removed-image.jpg", temp_path, removed_asset_id);
    fs::write(&removed_file_path, b"old image data").expect("Failed to write test file");
    
    // Verify the file was created
    assert!(Path::new(&removed_file_path).exists());

    // Create album response with only one asset (the other one is "removed")
    let album_response = json!({
        "id": &album_id,
        "name": "Test Album",
        "description": "This is a test album",
        "createdAt": "2021-01-01T00:00:00Z",
        "updatedAt": "2021-01-01T00:00:00Z",
        "assets": [
            {
                "id": &asset_id,
                "type": "IMAGE",
                "checksum": "abc123",
                "originalFileName": "test-image.jpg"
            }
            // removed_asset_id is intentionally not included
        ]
    });
    
    // Setup album endpoint mock
    let _album_mock = server.mock("GET", format!("/api/albums/{}?withoutAssets=false", album_id).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(album_response.to_string())
        .create();
    
    // Setup asset download endpoint mock
    let test_image_content = b"fake image data";
    let _asset_mock = server.mock("GET", format!("/api/assets/{}/original", asset_id).as_str())
        .with_status(200)
        .with_header("content-type", "application/octet-stream")
        .with_body(test_image_content)
        .create();
    
    // Create a reqwest client
    let client = reqwest::Client::new();
    
    // Create args struct with our test values
    let args = TestArgs {
        immich_url: mock_server_url,
        api_key: "test-api-key".to_string(),
        album_id: album_id.to_string(),
    };
    let max_images = 10;
    let originals_dir = temp_path.clone();

    // Run fetch_and_download_images which should download the new asset and remove the old one
    fetch_and_download_images(
        &client,
        &args,
        &originals_dir,
        max_images
    ).await.expect("Failed to fetch and download images");

    // Check that the directory contains exactly one file (the new one)
    let entries = fs::read_dir(&temp_path)
        .expect("Failed to read temp directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to collect directory entries");
    
    assert_eq!(entries.len(), 1, "Directory should contain exactly one file");
    
    // Verify the removed file no longer exists
    assert!(!Path::new(&removed_file_path).exists(), "Removed asset file should not exist");
    
    // Get the file path of the remaining file
    let file_path = entries[0].path();
    
    // Verify the file name contains the correct asset ID
    let file_name = file_path.file_name().unwrap().to_string_lossy();
    assert!(file_name.starts_with(asset_id), "File name should start with the asset ID");
    
    // Verify the file was downloaded correctly
    let downloaded_content = fs::read(&file_path).expect("Failed to read downloaded file");
    assert_eq!(downloaded_content, test_image_content);
    
    Ok(())
}
