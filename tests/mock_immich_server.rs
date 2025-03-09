use std::net::SocketAddr;
use std::sync::Arc;
use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs;
use serde_json::json;

struct MockServerConfig {
    album_id: String,
    asset_id: String,
    test_image_path: String,
}

async fn handle_request(
    req: Request<Body>,
    config: Arc<MockServerConfig>,
) -> Result<Response<Body>, Infallible> {
    let uri = req.uri().path();
    
    println!("Mock server received request: {}", uri);
    
    // Handle album request
    if uri == format!("/api/albums/{}", config.album_id) {
        let album_json = json!({
            "id": config.album_id,
            "assets": [
                {
                    "id": config.asset_id,
                    "originalPath": "test_image.jpg",
                    "deviceAssetId": "test_image",
                    "ownerId": "test_user",
                    "thumbhash": "",
                    "type": "IMAGE"
                }
            ]
        });
        
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(album_json.to_string()))
            .unwrap());
    }
    
    // Handle asset original image request
    if uri == format!("/api/assets/{}/original", config.asset_id) {
        match fs::read(&config.test_image_path).await {
            Ok(image_data) => {
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "image/jpeg")
                    .body(Body::from(image_data))
                    .unwrap());
            },
            Err(_) => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Image not found"))
                    .unwrap());
            }
        }
    }
    
    // Default 404 response
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap())
}

pub async fn start_mock_server(
    album_id: &str,
    asset_id: &str,
    test_image_path: &str,
    port: u16,
) -> Result<SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    let config = Arc::new(MockServerConfig {
        album_id: album_id.to_string(),
        asset_id: asset_id.to_string(),
        test_image_path: test_image_path.to_string(),
    });
    
    let make_svc = make_service_fn(move |_conn| {
        let config = Arc::clone(&config);
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req, Arc::clone(&config))
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    println!("Mock Immich server listening on http://{}", addr);

    // We don't await the server, as it would block forever
    tokio::spawn(async move {
        if let Err(e) = server.await {
            eprintln!("Mock server error: {}", e);
        }
    });

    Ok(addr)
}
