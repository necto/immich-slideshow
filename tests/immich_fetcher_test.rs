use std::fs;
use std::path::Path;
use mockito::Server;
use tempfile::tempdir;
use image_server_lib::{Asset, ImmichConfig, fetch_album_asset_list, download_asset};

#[tokio::test]
async fn test_download_asset() {
    // Create a temporary directory for test files
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Setup mock server
    let mut server = Server::new();
    let mock_server_url = server.url();
    
    // Mock the album endpoint
    let album_id = "test-album-id";
    let asset_id = "test-asset-id";
    let original_filename = "test-image.jpg";
    
    // Create mock album response
    let album_response = AlbumResponse {
        assets: vec![
            Asset {
                id: asset_id.to_string(),
                asset_type: "IMAGE".to_string(),
                checksum: "abc123".to_string(),
                original_file_name: original_filename.to_string(),
            }
        ],
    };
    
    // Setup album endpoint mock
    let album_mock = server.mock("GET", format!("/api/albums/{}?withoutAssets=false", album_id).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&album_response).unwrap())
        .create();
    
    // Setup asset download endpoint mock
    let test_image_content = b"fake image data";
    let asset_mock = server.mock("GET", format!("/api/assets/{}/original", asset_id).as_str())
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
        originals_dir: temp_path.clone(),
        max_images: 10,
    };
    
    // Fetch album assets
    let assets = fetch_album_asset_list(&client, &args, &args.album_id).await.expect("Failed to fetch album assets");
    
    // Verify we got the expected asset
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].id, asset_id);
    assert_eq!(assets[0].original_file_name, original_filename);
    
    // Download the asset
    let output_path = format!("{}/{}--_--{}", temp_path, asset_id, original_filename);
    download_asset(&client, &args, &asset_id, &output_path)
        .await
        .expect("Failed to download asset");
    
    // Verify the file was downloaded correctly
    assert!(Path::new(&output_path).exists());
    let downloaded_content = fs::read(&output_path).expect("Failed to read downloaded file");
    assert_eq!(downloaded_content, test_image_content);
}

// Helper struct to mimic the Args struct from the main code
struct TestArgs {
    immich_url: String,
    api_key: String,
    album_id: String,
    originals_dir: String,
    max_images: usize,
}

// Implement the ImmichConfig trait for TestArgs
impl ImmichConfig for TestArgs {
    fn immich_url(&self) -> &str {
        &self.immich_url
    }
    
    fn api_key(&self) -> &str {
        &self.api_key
    }
}
