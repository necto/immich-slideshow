use std::net::SocketAddr;
use std::sync::Arc;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde_json::json;
use std::path::Path;
use tokio::fs;
use std::io;

// Configuration for our mock server
pub struct MockServerConfig {
    album_id: String,
    asset_id: String,
    test_image_path: String,
}

// App state to hold our configuration
struct AppState {
    config: Arc<MockServerConfig>,
}

// Handler for album requests
async fn album_handler(data: web::Data<AppState>) -> impl Responder {
    println!("Mock server received album request");
    
    let album_json = json!({
        "id": data.config.album_id,
        "assets": [
            {
                "id": data.config.asset_id,
                "originalPath": "test_image.jpg",
                "deviceAssetId": "test_image",
                "ownerId": "test_user",
                "thumbhash": "",
                "type": "IMAGE"
            }
        ]
    });
    
    HttpResponse::Ok()
        .content_type("application/json")
        .body(album_json.to_string())
}

// Handler for asset original image requests
async fn asset_original_handler(data: web::Data<AppState>) -> impl Responder {
    println!("Mock server received asset original request");
    
    match fs::read(&data.config.test_image_path).await {
        Ok(image_data) => {
            HttpResponse::Ok()
                .content_type("image/jpeg")
                .body(image_data)
        },
        Err(_) => {
            HttpResponse::NotFound().body("Image not found")
        }
    }
}

// Default 404 handler
async fn not_found() -> impl Responder {
    HttpResponse::NotFound().body("Not found")
}

// Start the mock server
pub async fn start_mock_server(
    album_id: &str,
    asset_id: &str,
    test_image_path: &str,
    port: u16,
) -> Result<SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
    // Create the configuration
    let config = Arc::new(MockServerConfig {
        album_id: album_id.to_string(),
        asset_id: asset_id.to_string(),
        test_image_path: test_image_path.to_string(),
    });
    
    // Check if the test image path exists
    if !Path::new(test_image_path).exists() {
        println!("Warning: Test image file does not exist at path: {}", test_image_path);
    }
    
    // Build address
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let addr_str = format!("{}:{}", addr.ip(), addr.port());
    
    // Start the server
    println!("Starting mock Immich server on http://{}", addr);
    
    // Create the app with our handlers
    let config_clone = config.clone();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState { 
                config: config_clone.clone() 
            }))
            .route(
                &format!("/api/albums/{}", config_clone.album_id), 
                web::get().to(album_handler)
            )
            .route(
                &format!("/api/assets/{}/original", config_clone.asset_id), 
                web::get().to(asset_original_handler)
            )
            .default_service(web::route().to(not_found))
    })
    .bind(&addr_str)?
    .run();
    
    // Start server in the background
    let server_handle = server.handle();
    tokio::spawn(async move {
        if let Err(e) = server.await {
            eprintln!("Mock server error: {}", e);
        }
    });
    
    Ok(addr)
}
