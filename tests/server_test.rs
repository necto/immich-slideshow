use actix_web::{test, App, HttpResponse};
use image_server::{setup_app, AppState};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use tempfile::tempdir;

#[actix_web::test]
async fn test_image_cycling() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create some test images with different content
    for i in 1..=5 {
        let file_path = format!("{}/test{}.png", image_path, i);
        fs::write(&file_path, format!("Test image content {}", i))?;
    }
    
    // Create app state with our temporary image directory
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Make enough requests to cycle through all images
    let mut responses = HashSet::new();
    for _ in 0..10 {  // More than the number of images to ensure cycling
        let req = test::TestRequest::get().uri("/image").to_request();
        let resp: HttpResponse = test::call_and_read_body_json(&app, req).await;
        
        // Extract the image content
        let body = test::read_body(resp).await;
        let content = String::from_utf8_lossy(&body).to_string();
        
        // Add to our set of seen responses
        responses.insert(content);
    }
    
    // Check that we've seen all 5 different images
    assert_eq!(responses.len(), 5, "Should have received all 5 different images");
    
    Ok(())
}
