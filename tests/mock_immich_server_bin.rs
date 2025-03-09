use std::env;

mod mock_immich_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let album_id = env::var("ALBUM_ID").unwrap_or_else(|_| "test-album-123".to_string());
    let asset_id = env::var("ASSET_ID").unwrap_or_else(|_| "test-asset-456".to_string());
    let test_image_path = env::var("TEST_IMAGE_PATH").unwrap_or_else(|_| "tests/test_image.jpg".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8000".to_string()).parse().unwrap();

    println!("Starting mock Immich server with:");
    println!("Album ID: {}", album_id);
    println!("Asset ID: {}", asset_id);
    println!("Test Image: {}", test_image_path);

    let addr = mock_immich_server::start_mock_server(
        &album_id, 
        &asset_id, 
        &test_image_path, 
        port
    ).await?;
    
    println!("Mock server started on {}", addr);
    
    // Keep the server running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down mock server");
    
    Ok(())
}
