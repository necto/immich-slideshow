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
        image_order_file: format!("{}/image_order.json", image_path),
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
        image_order_file: format!("{}/image_order.json", image_path),
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
        image_order_file: format!("{}/image_order.json", image_path),
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
        image_order_file: format!("{}/image_order.json", image_path),
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
        image_order_file: format!("{}/image_order.json", image_path),
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
        image_order_file: format!("{}/image_order.json", image_path),
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

#[actix_web::test]
async fn test_all_images_page() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create some test images
    for i in 1..=3 {
        let file_path = format!("{}/test{}.png", image_path, i);
        fs::write(&file_path, format!("Test image content {}", i))?;
    }
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Request /all-images
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    // Verify HTML contains expected elements
    assert!(content.contains("Image Gallery"), "Should have gallery title");
    assert!(content.contains("Next image to serve"), "Should show next indicator");
    assert!(content.contains("test1.png"), "Should contain first image filename");
    assert!(content.contains("test2.png"), "Should contain second image filename");
    assert!(content.contains("test3.png"), "Should contain third image filename");
    assert!(content.contains("image-grid"), "Should have image grid styling");
    assert!(content.contains("(out of 3)"), "Should show count of images");
    
    Ok(())
}

#[actix_web::test]
async fn test_all_images_next_indicator() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create some test images
    for i in 1..=4 {
        let file_path = format!("{}/image{}.png", image_path, i);
        fs::write(&file_path, format!("Test image {}", i))?;
    }
    
    // Create app state with counter at 2
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(2),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Request /all-images
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    // With counter at 2 and 4 images, next index should be 2 (0-based), which is "3" in display
    assert!(content.contains("Next image to serve:</strong> 3"), "Should indicate image 3 is next");
    assert!(content.contains("(out of 4)"), "Should show 4 images total");
    
    // Verify the next image card is rendered with special styling
    // The card for image3.png should have the "image-card next" class with yellow background
    assert!(content.contains("class='image-card next'"), "Next image card should have special styling class");
    assert!(content.contains("#fff9e6"), "Next image card should have yellow background color");
    assert!(content.contains("#ffc107"), "Next image card should have golden border");
    
    // Verify it's specifically image3.png with the next styling
    // Extract the content between <div class='image-card next'> and </div> to verify it contains image3.png
    if let Some(pos) = content.find("<div class='image-card next'>") {
        let next_card_section = &content[pos..];
        if let Some(end) = next_card_section.find("</div>") {
            let next_card_content = &next_card_section[..end];
            assert!(next_card_content.contains("image3.png"), 
                "The next styled card should contain image3.png, but got: {}", next_card_content);
        }
    } else {
        panic!("Could not find the next image card in HTML");
    }
    
    Ok(())
}

#[actix_web::test]
async fn test_all_images_empty_directory() -> std::io::Result<()> {
    // Create a temporary directory WITHOUT images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Request /all-images
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    // Verify message for no images
    assert!(content.contains("No images found"), "Should show no images message");
    
    Ok(())
}

#[actix_web::test]
async fn test_file_endpoint() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create some test images
    fs::write(format!("{}/test1.png", image_path), "Test image 1 content")?;
    fs::write(format!("{}/test2.png", image_path), "Test image 2 content")?;
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Request specific file
    let req = test::TestRequest::get().uri("/file/test1.png").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    // Verify correct file was served
    assert_eq!(content, "Test image 1 content");
    
    Ok(())
}

#[actix_web::test]
async fn test_file_endpoint_different_files() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create test images with different content
    fs::write(format!("{}/test1.png", image_path), "Content 1")?;
    fs::write(format!("{}/test2.png", image_path), "Content 2")?;
    fs::write(format!("{}/test3.png", image_path), "Content 3")?;
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Test multiple files
    for (filename, expected_content) in &[("test1.png", "Content 1"), ("test2.png", "Content 2"), ("test3.png", "Content 3")] {
        let req = test::TestRequest::get().uri(&format!("/file/{}", filename)).to_request();
        let resp = test::call_service(&app, req).await;
        
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let content = String::from_utf8_lossy(&body).to_string();
        
        assert_eq!(&content, *expected_content, "File {} should have correct content", filename);
    }
    
    Ok(())
}

#[actix_web::test]
async fn test_file_endpoint_not_found() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Request non-existent file
    let req = test::TestRequest::get().uri("/file/nonexistent.png").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), 404, "Should return 404 for non-existent file");
    
    Ok(())
}

#[actix_web::test]
async fn test_file_endpoint_directory_traversal() -> std::io::Result<()> {
    // Create a temporary directory structure
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Try directory traversal attacks
    for dangerous_path in &["../test.png", "../../etc/passwd", "/etc/passwd", "..\\..\\test"] {
        let uri = format!("/file/{}", dangerous_path);
        let req = test::TestRequest::get().uri(&uri).to_request();
        let resp = test::call_service(&app, req).await;
        
        assert!(!resp.status().is_success(), "Should reject directory traversal: {}", dangerous_path);
    }
    
    Ok(())
}

#[actix_web::test]
async fn test_all_images_uses_file_endpoint() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create some test images
    for i in 1..=3 {
        let file_path = format!("{}/image{}.png", image_path, i);
        fs::write(&file_path, format!("Test image {}", i))?;
    }
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Request /all-images
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    // Verify HTML contains /file/ URLs instead of /image
    assert!(content.contains("src='/file/"), "Should use /file/ endpoints for images");
    assert!(content.contains("/file/image1.png"), "Should reference image1.png");
    assert!(content.contains("/file/image2.png"), "Should reference image2.png");
    assert!(content.contains("/file/image3.png"), "Should reference image3.png");
    
    Ok(())
}

#[actix_web::test]
async fn test_image_and_all_images_same_order() -> std::io::Result<()> {
    // Create a temporary directory with test images in non-alphabetical order
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create images with names that would sort differently than creation order
    // Create in order: z.png, a.png, m.png
    // Alphabetically sorted would be: a.png, m.png, z.png
    fs::write(format!("{}/z.png", image_path), "Z image")?;
    fs::write(format!("{}/a.png", image_path), "A image")?;
    fs::write(format!("{}/m.png", image_path), "M image")?;
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Get the order from /all-images
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let all_images_content = String::from_utf8_lossy(&body).to_string();
    
    // Extract filenames in order from /all-images HTML
    // They appear in <div class='image-name'>filename</div> elements
    let mut all_images_order = Vec::new();
    
    let marker = "<div class='image-name'>";
    let mut idx = 0;
    while let Some(pos) = all_images_content[idx..].find(marker) {
        idx = idx + pos + marker.len();
        if let Some(end) = all_images_content[idx..].find("</div>") {
            let filename = all_images_content[idx..idx + end].to_string();
            all_images_order.push(filename);
            idx = idx + end;
        }
    }
    
    println!("Order from /all-images: {:?}", all_images_order);
    
    // Now get the order from multiple /image calls
    let mut image_order = Vec::new();
    for i in 0..3 {
        let req = test::TestRequest::get().uri("/image").to_request();
        let resp = test::call_service(&app, req).await;
        let body = test::read_body(resp).await;
        let content = String::from_utf8_lossy(&body).to_string();
        
        // The content is the actual file content, but we can check which one by content
        let filename = if content.contains("Z") {
            "z.png"
        } else if content.contains("A") {
            "a.png"
        } else {
            "m.png"
        };
        
        image_order.push(filename.to_string());
        println!("Request {}: served {}", i, filename);
    }
    
    println!("Order from /image: {:?}", image_order);
    
    // The orders should match
    assert_eq!(all_images_order, image_order, 
        "/all-images order should match /image serving order");
    
    Ok(())
}

#[actix_web::test]
async fn test_reorder_images_move_to_position() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    let order_file = format!("{}/image_order.json", image_path);
    
    // Create test images
    for i in 1..=3 {
        let file_path = format!("{}/image{}.png", image_path, i);
        fs::write(&file_path, format!("Test image {}", i))?;
    }
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: order_file.clone(),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // First request to initialize order
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Request to move image2.png to position 0
    let req = test::TestRequest::get()
        .uri("/all-images?image-name=image2.png&move-to=0")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    // Verify the order changed by checking the /image endpoint
    // First call should now serve image2.png
    let req = test::TestRequest::get().uri("/image").to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    assert_eq!(content, "Test image 2");
    
    Ok(())
}

#[actix_web::test]
async fn test_reorder_images_persistence() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    let order_file = format!("{}/image_order.json", image_path);
    
    // Create test images
    for i in 1..=3 {
        fs::write(format!("{}/file{}.png", image_path, i), format!("Image {}", i))?;
    }
    
    // Create app state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: order_file.clone(),
    });
    
    // Set up the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Initialize order
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Reorder: move file3.png to position 0
    let req = test::TestRequest::get()
        .uri("/all-images?image-name=file3.png&move-to=0")
        .to_request();
    let _ = test::call_service(&app, req).await;
    
    // Check the saved order file
    let order_content = fs::read_to_string(&order_file)?;
    let parsed: Value = serde_json::from_str(&order_content).unwrap();
    
    // file3.png should be at index 0
    assert_eq!(parsed[0], "file3.png");
    
    Ok(())
}

#[actix_web::test]
async fn test_reorder_multiple_times() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    for i in 1..=4 {
        fs::write(format!("{}/img{}.png", image_path, i), format!("Image {}", i))?;
    }
    
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Initialize
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Move img4 to position 0
    let req = test::TestRequest::get()
        .uri("/all-images?image-name=img4.png&move-to=0")
        .to_request();
    let _ = test::call_service(&app, req).await;
    
    // Move img2 to position 1
    let req = test::TestRequest::get()
        .uri("/all-images?image-name=img2.png&move-to=1")
        .to_request();
    let _ = test::call_service(&app, req).await;
    
    // Verify /image serves in correct order
    let mut served_images = Vec::new();
    for _ in 0..4 {
        let req = test::TestRequest::get().uri("/image").to_request();
        let resp = test::call_service(&app, req).await;
        let body = test::read_body(resp).await;
        let content = String::from_utf8_lossy(&body).to_string();
        served_images.push(content);
    }
    
    assert_eq!(served_images[0], "Image 4"); // img4.png first
    assert_eq!(served_images[1], "Image 2"); // img2.png second
    
    Ok(())
}

#[actix_web::test]
async fn test_reorder_nonexistent_image_returns_error() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    for i in 1..=3 {
        fs::write(format!("{}/image{}.png", image_path, i), format!("Image {}", i))?;
    }
    
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    });
    
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Initialize the order
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Try to move a non-existent image
    let req = test::TestRequest::get()
        .uri("/all-images?image-name=nonexistent.png&move-to=0")
        .to_request();
    let resp = test::call_service(&app, req).await;
    
    // Should return 400 Bad Request
    assert_eq!(resp.status(), 400);
    
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    // Should contain error message about image not found
    assert!(content.contains("not found"), "Error message should mention image not found");
    assert!(content.contains("nonexistent.png"), "Error should mention the image name");
    
    Ok(())
}
