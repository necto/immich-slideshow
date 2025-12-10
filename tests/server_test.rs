use actix_web::{test, App};
use image_server_lib::server_lib::{setup_app, AppState};
use std::collections::HashSet;
use std::fs;
use std::sync::atomic::AtomicUsize;
use tempfile::tempdir;
use serde_json::Value;

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
    let temp_params_file = format!("{}/params.json", image_path);
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: temp_params_file.clone(),
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
        let resp = test::call_service(&app, req).await;
        
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

#[actix_web::test]
async fn test_parameter_storage_and_retrieval() -> std::io::Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    let params_file = format!("{}/params.json", image_path);
    
    // Create a test image
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: params_file.clone(),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Request /image with query parameters
    let req = test::TestRequest::get()
        .uri("/image?param1=18&alfa=x&status=active")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    // Now check /control-panel endpoint
    let req = test::TestRequest::get()
        .uri("/control-panel")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    // Parse the JSON response
    let parsed: Value = serde_json::from_str(&content).unwrap();
    
    // Verify all parameters are stored with their values
    assert_eq!(parsed["param1"]["value"], "18");
    assert_eq!(parsed["alfa"]["value"], "x");
    assert_eq!(parsed["status"]["value"], "active");
    
    // Verify timestamps are present and are numbers
    assert!(parsed["param1"]["timestamp"].is_number());
    assert!(parsed["alfa"]["timestamp"].is_number());
    assert!(parsed["status"]["timestamp"].is_number());
    
    Ok(())
}

#[actix_web::test]
async fn test_parameter_overwrite() -> std::io::Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    let params_file = format!("{}/params.json", image_path);
    
    // Create a test image
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: params_file.clone(),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // First request with param1=value1
    let req = test::TestRequest::get()
        .uri("/image?param1=value1")
        .to_request();
    let _ = test::call_service(&app, req).await;
    
    // Second request with param1=value2 (should overwrite)
    let req = test::TestRequest::get()
        .uri("/image?param1=value2")
        .to_request();
    let _ = test::call_service(&app, req).await;
    
    // Check /control-panel
    let req = test::TestRequest::get()
        .uri("/control-panel")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    let parsed: Value = serde_json::from_str(&content).unwrap();
    
    // Verify the parameter was overwritten with the latest value
    assert_eq!(parsed["param1"]["value"], "value2");
    
    Ok(())
}

#[actix_web::test]
async fn test_control_panel_empty() -> std::io::Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    let params_file = format!("{}/params.json", image_path);
    
    // Create a test image
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state (without creating the params file)
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: params_file.clone(),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Check /control-panel when no params file exists
    let req = test::TestRequest::get()
        .uri("/control-panel")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    // Should return empty JSON object
    assert_eq!(content.trim(), "{}");
    
    Ok(())
}

#[actix_web::test]
async fn test_url_encoded_parameters() -> std::io::Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    let params_file = format!("{}/params.json", image_path);
    
    // Create a test image
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: params_file.clone(),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Request with URL-encoded parameter (space as %20)
    let req = test::TestRequest::get()
        .uri("/image?message=hello%20world&email=test%40example.com")
        .to_request();
    let _ = test::call_service(&app, req).await;
    
    // Check /control-panel
    let req = test::TestRequest::get()
        .uri("/control-panel")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    let parsed: Value = serde_json::from_str(&content).unwrap();
    
    // Verify URL decoding worked
    assert_eq!(parsed["message"]["value"], "hello world");
    assert_eq!(parsed["email"]["value"], "test@example.com");
    
    Ok(())
}

#[actix_web::test]
async fn test_selective_parameter_overwrite() -> std::io::Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    let params_file = format!("{}/params.json", image_path);
    
    // Create a test image
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: params_file.clone(),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // First request with param1 and param2
    let req = test::TestRequest::get()
        .uri("/image?param1=value1&param2=value2")
        .to_request();
    let _ = test::call_service(&app, req).await;
    
    // Verify both parameters are stored
    let req = test::TestRequest::get()
        .uri("/control-panel")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    let parsed: Value = serde_json::from_str(&content).unwrap();
    
    assert_eq!(parsed["param1"]["value"], "value1");
    assert_eq!(parsed["param2"]["value"], "value2");
    
    // Second request only modifies param1
    let req = test::TestRequest::get()
        .uri("/image?param1=updated_value1")
        .to_request();
    let _ = test::call_service(&app, req).await;
    
    // Check /control-panel again
    let req = test::TestRequest::get()
        .uri("/control-panel")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    let parsed: Value = serde_json::from_str(&content).unwrap();
    
    // param1 should be updated, param2 should remain unchanged
    assert_eq!(parsed["param1"]["value"], "updated_value1", "param1 should be overwritten");
    assert_eq!(parsed["param2"]["value"], "value2", "param2 should preserve its value");
    
    Ok(())
}
