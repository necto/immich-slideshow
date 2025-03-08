use std::fs;
use std::path::Path;
use mockito::{mock, server_url};
use tempfile::tempdir;
use serde_json::json;

// Import the Asset struct from the main code
// We need to recreate this here since we can't directly import from the binary
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Asset {
    pub id: String,
    #[serde(rename = "type")]
    pub asset_type: String,
    pub checksum: String,
    #[serde(rename = "originalFileName")]
    pub original_file_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AlbumResponse {
    pub assets: Vec<Asset>,
}

#[tokio::test]
async fn test_download_asset() {
    // Create a temporary directory for test files
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Setup mock server
    let mock_server_url = server_url();
    
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
    let _m1 = mock("GET", format!("/api/albums/{}?withoutAssets=false", album_id).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&album_response).unwrap())
        .create();
    
    // Setup asset download endpoint mock
    let test_image_content = b"fake image data";
    let _m2 = mock("GET", format!("/api/assets/{}/original", asset_id).as_str())
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
    let assets = fetch_album_asset_list(&client, &args).await.expect("Failed to fetch album assets");
    
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

// Copy of the fetch_album_asset_list function from the main code
async fn fetch_album_asset_list(client: &reqwest::Client, args: &TestArgs) -> anyhow::Result<Vec<Asset>> {
    let url = format!("{}/api/albums/{}?withoutAssets=false",
                      args.immich_url, args.album_id);
    
    let response = client.get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .header("x-api-key", &args.api_key)
        .send()
        .await?;
        
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("Failed to fetch album assets: HTTP {}: {}", status, text);
    }

    let resp: AlbumResponse = response.json().await?;
    Ok(resp.assets)
}

// Copy of the download_asset function from the main code
async fn download_asset(client: &reqwest::Client, args: &TestArgs, asset_id: &str, output_path: &str) -> anyhow::Result<()> {
    let url = format!("{}/api/assets/{}/original", args.immich_url, asset_id);
    
    let response = client.get(url)
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .header("x-api-key", &args.api_key)
        .send()
        .await?;
        
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("Failed to download asset: HTTP {}: {}", status, text);
    }
    
    let bytes = response.bytes().await?;
    fs::write(output_path, bytes)?;
    
    Ok(())
}
