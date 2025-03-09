use std::env;
use anyhow;

mod mock_immich_server;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // Get configuration from environment variables
    let album_id = env::var("ALBUM_ID").unwrap_or_else(|_| "test-album-123".to_string());
    let asset_id = env::var("ASSET_ID").unwrap_or_else(|_| "test-asset-456".to_string());
    let test_image_path = env::var("TEST_IMAGE_PATH").unwrap_or_else(|_| "tests/test_image.jpg".to_string());

    println!("Starting mock Immich server with:");
    println!("Album ID: {}", album_id);
    println!("Asset ID: {}", asset_id);
    println!("Test Image: {}", test_image_path);

    // Start the server
    let addr = mock_immich_server::start_mock_server(
        &album_id, 
        &asset_id, 
        &test_image_path
    ).await?;
    
    println!("Mock server started on {}", addr);
    
    // Keep the server running until we receive a ctrl+c signal
    tokio::signal::ctrl_c().await?;
    println!("Shutting down mock server");
    
    Ok(())
}
