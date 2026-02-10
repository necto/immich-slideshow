use actix_web::{test, App};
use image_server_lib::server_lib::{setup_app, AppState};
use std::collections::HashSet;
use std::fs;
use std::sync::atomic::AtomicUsize;
use tempfile::tempdir;
use serde_json::Value;

// Helper function to create test images in a directory
fn create_test_images(image_path: &str, count: usize) -> std::io::Result<()> {
    for i in 1..=count {
        let file_path = format!("{}/img{}.png", image_path, i);
        fs::write(&file_path, format!("Image {}", i))?;
    }
    Ok(())
}

// Helper function to create test images with custom pattern
fn create_test_images_with_pattern(
    image_path: &str,
    pattern: &str,
    count: usize,
) -> std::io::Result<()> {
    for i in 1..=count {
        let file_path = format!("{}/{}{}.png", image_path, pattern, i);
        fs::write(&file_path, format!("Test image content {}", i))?;
    }
    Ok(())
}

// Helper function to create AppState
fn create_app_state(image_path: &str) -> actix_web::web::Data<AppState> {
    actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.to_string(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: format!("{}/image_order.json", image_path),
    })
}

#[actix_web::test]
async fn test_image_cycling() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create test images
    create_test_images_with_pattern(&image_path, "test", 5)?;
    
    // Create app state and service
    let app_state = create_app_state(&image_path);
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
    
    // Create a test image
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state and service
    let app_state = create_app_state(&image_path);
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
    
    // Create a test image
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state and service
    let app_state = create_app_state(&image_path);
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
    
    // Create a test image
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state
    let app_state = create_app_state(&image_path);
    
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
    
    // Create a test image
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state
    let app_state = create_app_state(&image_path);
    
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
    
    // Create a test image
    fs::write(format!("{}/test.png", image_path), "Test content")?;
    
    // Create app state
    let app_state = create_app_state(&image_path);
    
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
    
    // Create test images
    create_test_images_with_pattern(&image_path, "test", 3)?;
    
    // Create app state and service
    let app_state = create_app_state(&image_path);
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
    create_test_images_with_pattern(&image_path, "image", 4)?;
    
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
    let app_state = create_app_state(&image_path);
    
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
    let app_state = create_app_state(&image_path);
    
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
    let app_state = create_app_state(&image_path);
    
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
    let app_state = create_app_state(&image_path);
    
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
    create_test_images_with_pattern(&image_path, "image", 3)?;
    
    // Create app state
    let app_state = create_app_state(&image_path);
    
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
    let app_state = create_app_state(&image_path);
    
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
    create_test_images_with_pattern(&image_path, "image", 3)?;
    
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
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("image-name", "image2.png"), ("move-to", "0")])
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    // Verify the order changed by checking the /image endpoint
    // First call should now serve image2.png
    let req = test::TestRequest::get().uri("/image").to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    assert_eq!(content, "Test image content 2");
    
    Ok(())
}

#[actix_web::test]
async fn test_reorder_images_persistence() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    let order_file = format!("{}/image_order.json", image_path);
    
    // Create test images
    create_test_images(&image_path, 3)?;
    
    // Rename them to file*.png
    for i in 1..=3 {
        fs::rename(
            format!("{}/img{}.png", image_path, i),
            format!("{}/file{}.png", image_path, i)
        )?;
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
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("image-name", "file3.png"), ("move-to", "0")])
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
    
    create_test_images(&image_path, 4)?;
    
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
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("image-name", "img4.png"), ("move-to", "0")])
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Move img2 to position 1
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("image-name", "img2.png"), ("move-to", "1")])
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
    
    create_test_images(&image_path, 3)?;
    
    // Rename to image*.png pattern
    for i in 1..=3 {
        fs::rename(
            format!("{}/img{}.png", image_path, i),
            format!("{}/image{}.png", image_path, i)
        )?;
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
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("image-name", "nonexistent.png"), ("move-to", "0")])
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

#[actix_web::test]
async fn test_new_images_inserted_after_current_position() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create initial images
    create_test_images(&image_path, 3)?;
    
    let app_state = create_app_state(&image_path);
    
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Initialize by requesting all-images to create the order
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Check the initial order
    let order_file = format!("{}/image_order.json", image_path);
    let initial_order_content = fs::read_to_string(&order_file)?;
    let initial_order: Value = serde_json::from_str(&initial_order_content).unwrap();
    
    println!("Initial order: {:?}", initial_order);
    
    // Move counter to position 1 by making a request
    // This moves counter from 0 to 1
    let req = test::TestRequest::get().uri("/image").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Current position is now 1, next will be at index 2 % 3
    // Add a new image
    fs::write(format!("{}/img4.png", image_path), "Image 4")?;
    
    // Make another request which will reload entries and insert the new image
    // at position next_index + 1 where next_index = 1 % 3 = 1
    // so it inserts at position 2
    let req = test::TestRequest::get().uri("/image").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Check the order after adding new image
    let new_order_content = fs::read_to_string(&order_file)?;
    let new_order: Value = serde_json::from_str(&new_order_content).unwrap();
    
    println!("Order after adding img4: {:?}", new_order);
    
    // The new image should be at position 2 (right after position 1)
    assert_eq!(new_order[2], "img4.png", 
        "New image should be inserted right after current position");
    
    // Verify the array has 4 elements
    assert_eq!(new_order.as_array().unwrap().len(), 4);
    
    Ok(())
}

#[actix_web::test]
async fn test_multiple_new_images_near_end_of_list() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    let order_file = format!("{}/image_order.json", image_path);
    
    // Create initial images
    create_test_images(&image_path, 5)?;
    
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: order_file.clone(),
    });
    
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Initialize order
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Get the initial order
    let initial_order_content = fs::read_to_string(&order_file)?;
    let initial_order: Value = serde_json::from_str(&initial_order_content).unwrap();
    println!("Initial order: {:?}", initial_order);
    
    // Make 4 requests to move counter near the end
    // After 4 requests, counter will be at 4, next index will be 4 % 5 = 4 (near the end)
    for _ in 0..4 {
        let req = test::TestRequest::get().uri("/image").to_request();
        let _ = test::call_service(&app, req).await;
    }
    
    // Now counter is at 4, next will be at index 4 % 5 = 4
    // Add two new images
    fs::write(format!("{}/new_a.png", image_path), "New A")?;
    fs::write(format!("{}/new_b.png", image_path), "New B")?;
    
    // Make another request to trigger the new images insertion
    let req = test::TestRequest::get().uri("/image").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Check the order after adding new images
    let new_order_content = fs::read_to_string(&order_file)?;
    let new_order: Value = serde_json::from_str(&new_order_content).unwrap();
    
    println!("Order after adding 2 images: {:?}", new_order);
    
    // Verify the array has 7 elements (5 original + 2 new)
    assert_eq!(new_order.as_array().unwrap().len(), 7, "Should have 7 images total");
    
    // The new images should be inserted at positions 5 and 6 (right after position 4)
    // They should be together at those positions, in some order
    let new_a_pos = new_order.as_array().unwrap()
        .iter()
        .position(|v| v.as_str() == Some("new_a.png"));
    let new_b_pos = new_order.as_array().unwrap()
        .iter()
        .position(|v| v.as_str() == Some("new_b.png"));
    
    assert!(new_a_pos.is_some(), "new_a.png should be in the list");
    assert!(new_b_pos.is_some(), "new_b.png should be in the list");
    
    let new_a_idx = new_a_pos.unwrap();
    let new_b_idx = new_b_pos.unwrap();
    
    // Both should be at positions 5 or 6
    assert!(new_a_idx >= 5 && new_a_idx <= 6, "new_a.png should be at position 5 or 6");
    assert!(new_b_idx >= 5 && new_b_idx <= 6, "new_b.png should be at position 5 or 6");
    assert_ne!(new_a_idx, new_b_idx, "new_a and new_b should be at different positions");
    
    // Verify that the order of the original images is preserved (except the ones we didn't see)
    // Position 4 should contain one of the original images
    let pos_4_value = new_order[4].as_str().unwrap();
    assert!(pos_4_value.starts_with("img"), "Position 4 should have an original image");
    
    Ok(())
}

#[actix_web::test]
async fn test_new_images_with_counter_at_list_end() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    let order_file = format!("{}/image_order.json", image_path);
    
    // Create initial images
    create_test_images(&image_path, 3)?;
    
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: image_path.clone(),
        params_file: format!("{}/params.json", image_path),
        image_order_file: order_file.clone(),
    });
    
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Initialize order
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let _ = test::call_service(&app, req).await;
    
    let initial_order_content = fs::read_to_string(&order_file)?;
    let initial_order: Value = serde_json::from_str(&initial_order_content).unwrap();
    println!("Initial order: {:?}", initial_order);
    
    // Make requests to position counter at the very end
    // With 3 images, after 3 requests counter will be at 3
    // Next index will be 3 % 3 = 0, so we'll insert after position 0
    for _ in 0..3 {
        let req = test::TestRequest::get().uri("/image").to_request();
        let _ = test::call_service(&app, req).await;
    }
    
    // Add a new image when counter is at the end (will wrap to position 0)
    fs::write(format!("{}/new_late.png", image_path), "New Late")?;
    
    // Request to trigger insertion
    let req = test::TestRequest::get().uri("/image").to_request();
    let _ = test::call_service(&app, req).await;
    
    let new_order_content = fs::read_to_string(&order_file)?;
    let new_order: Value = serde_json::from_str(&new_order_content).unwrap();
    
    println!("Order after counter wrapped: {:?}", new_order);
    
    // Should have 4 images
    assert_eq!(new_order.as_array().unwrap().len(), 4);
    
    // The new image should be inserted at position 1 (after position 0)
    // because counter % 3 = 0, and we insert after 0, so at position 1
    assert_eq!(new_order[1], "new_late.png", 
        "New image should be inserted at position 1 (after position 0)");
    
    Ok(())
}

#[actix_web::test]
async fn test_set_next_index_parameter() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create test images
    create_test_images(&image_path, 5)?;
    
    let app_state = create_app_state(&image_path);
    
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Initialize order
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let _gallery_html = String::from_utf8_lossy(&body).to_string();
    
    // Extract the order from the gallery HTML or read the order file
    let order_file = format!("{}/image_order.json", image_path);
    let order_content = fs::read_to_string(&order_file)?;
    let order: Value = serde_json::from_str(&order_content).unwrap();
    
    let order_list = order.as_array().unwrap();
    let image_at_index_3 = order_list[3].as_str().unwrap();
    
    println!("Order: {:?}", order_list);
    println!("Image at index 3: {}", image_at_index_3);
    
    // Set next index to 3 via /all-images
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("next-index", "3")])
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    // Next request should serve the image at index 3
    let req = test::TestRequest::get().uri("/image").to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let served_content = String::from_utf8_lossy(&body).to_string();
    
    // Verify we got the correct image from the order
    let expected_content = format!("Image {}", &image_at_index_3[3..image_at_index_3.len()-4]);
    assert_eq!(served_content, expected_content, 
        "Should serve the image at index 3, which is {}", image_at_index_3);
    
    Ok(())
}

#[actix_web::test]
async fn test_next_index_zero() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create test images
    create_test_images(&image_path, 3)?;
    
    let app_state = create_app_state(&image_path);
    
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Initialize order
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Make some requests to advance counter
    for _ in 0..2 {
        let req = test::TestRequest::get().uri("/image").to_request();
        let _ = test::call_service(&app, req).await;
    }
    
    // Set next index to 0
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("next-index", "0")])
        .to_request();
    let _ = test::call_service(&app, req).await;
    
    // Next request should serve image at index 0
    let req = test::TestRequest::get().uri("/image").to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    
    // All images start with "Image ", verify we got one
    assert!(content.starts_with("Image"), "Should get an image");
    
    Ok(())
}

#[actix_web::test]
async fn test_next_index_with_reorder() -> std::io::Result<()> {
    // Create a temporary directory with test images
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create test images
    create_test_images(&image_path, 4)?;
    
    let app_state = create_app_state(&image_path);
    
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Initialize order
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Reorder and set next index in one request
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("image-name", "img4.png"), ("move-to", "0"), ("next-index", "0")])
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    // Next request should serve img4.png (which we just moved to position 0)
    let req = test::TestRequest::get().uri("/image").to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let content = String::from_utf8_lossy(&body).to_string();
    assert_eq!(content, "Image 4");
    
    Ok(())
}

#[actix_web::test]
async fn test_next_index_sequence() -> std::io::Result<()> {
    // Test setting different indices in sequence
    let temp_dir = tempdir()?;
    let image_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create test images
    create_test_images(&image_path, 5)?;
    
    let app_state = create_app_state(&image_path);
    
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .configure(setup_app)
    ).await;
    
    // Initialize order
    let req = test::TestRequest::get().uri("/all-images").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Test jumping to index 0
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("next-index", "0")])
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Get image at index 0
    let req = test::TestRequest::get().uri("/image").to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let img_0 = String::from_utf8_lossy(&body).to_string();

    // Jump to index 2
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("next-index", "2")])
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Get image at index 2
    let req = test::TestRequest::get().uri("/image").to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let img_2 = String::from_utf8_lossy(&body).to_string();

    // Jump to index 4
    let req = test::TestRequest::post()
        .uri("/all-images")
        .set_form(&[("next-index", "4")])
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Get image at index 4
    let req = test::TestRequest::get().uri("/image").to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let img_4 = String::from_utf8_lossy(&body).to_string();
    
    // All should be different
    assert_ne!(img_0, img_2);
    assert_ne!(img_2, img_4);
    assert_ne!(img_0, img_4);
    
    Ok(())
}
