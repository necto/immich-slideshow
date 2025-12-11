use actix_files::NamedFile;
use actix_web::{get, web, HttpRequest, HttpResponse, http::header};
use std::path::{PathBuf, Path};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fs;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Local};

pub struct AppState {
    pub counter: AtomicUsize,
    pub image_dir: String,
    pub params_file: String,
}

#[get("/image")]
async fn get_image(data: actix_web::web::Data<AppState>, req: HttpRequest) -> actix_web::Result<HttpResponse> {
    // Store GET parameters if any
    let query_string = req.query_string();
    if !query_string.is_empty() {
        let _ = store_parameters(&data.params_file, query_string);
    }

    // Get all image files in the images directory
    let entries = get_image_entries(&data.image_dir)?;

    if entries.is_empty() {
        return Err(actix_web::error::ErrorInternalServerError("No files found in static directory"));
    }

    let counter = data.counter.fetch_add(1, Ordering::SeqCst);
    if entries.len() - 1 <= counter {
        data.counter.store(0, Ordering::SeqCst);
    }
    // Increment counter and get current value
    let index = counter % entries.len();

    // Choose image based on count
    let path = &entries[index];
    println!("Serving image #{}: {}", index, path.display());

    // Open the file
    let file = NamedFile::open(path)?;

    let mut response = file.into_response(&req);

    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    );
    response.headers_mut().insert(
        header::PRAGMA,
        header::HeaderValue::from_static("no-cache"),
    );
    response.headers_mut().insert(
        header::EXPIRES,
        header::HeaderValue::from_static("0"),
    );

    Ok(response)
}

#[get("/control-panel")]
async fn get_control_panel(data: actix_web::web::Data<AppState>) -> actix_web::Result<HttpResponse> {
    // Read the parameters file
    match fs::read_to_string(&data.params_file) {
        Ok(content) => {
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(content))
        }
        Err(_) => {
            // If file doesn't exist, return an empty JSON object
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body("{}"))
        }
    }
}

/// Get all image files from the image directory, sorted by path
fn get_image_entries(image_dir: &str) -> actix_web::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(image_dir)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.is_file() {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .collect::<Vec<PathBuf>>();

    entries.sort();
    Ok(entries)
}

fn store_parameters(params_file: &str, query_string: &str) -> std::io::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Load existing parameters
    let mut params: Value = if std::path::Path::new(params_file).exists() {
        let content = fs::read_to_string(params_file)?;
        serde_json::from_str(&content).unwrap_or(json!({}))
    } else {
        json!({})
    };

    // Parse query string and add parameters
    for pair in query_string.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let decoded_value = urlencoding::decode(value)
                .unwrap_or_else(|_| std::borrow::Cow::Borrowed(value))
                .to_string();

            // Store the parameter with its timestamp
            if let Some(obj) = params.as_object_mut() {
                obj.insert(
                    key.to_string(),
                    json!({
                        "value": decoded_value,
                        "timestamp": now
                    }),
                );
            }
        }
    }

    // Write back to file
    fs::write(params_file, serde_json::to_string_pretty(&params)?)?;
    Ok(())
}

#[get("/file/{filename}")]
async fn get_file(data: actix_web::web::Data<AppState>, filename: web::Path<String>, req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let filename = filename.into_inner();
    
    // Prevent directory traversal attacks - reject paths with ".." or starting with "/"
    if filename.contains("..") || filename.starts_with('/') {
        return Err(actix_web::error::ErrorBadRequest("Invalid filename"));
    }
    
    // Build the full path
    let file_path = Path::new(&data.image_dir).join(&filename);
    
    // Verify the resolved path is still within the image directory
    let canonicalized_image_dir = fs::canonicalize(&data.image_dir)
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to resolve image directory"))?;
    let canonicalized_file_path = fs::canonicalize(&file_path)
        .map_err(|_| actix_web::error::ErrorNotFound("File not found"))?;
    
    if !canonicalized_file_path.starts_with(&canonicalized_image_dir) {
        return Err(actix_web::error::ErrorForbidden("Access denied"));
    }
    
    // Verify the file exists and is a file (not a directory)
    if !canonicalized_file_path.is_file() {
        return Err(actix_web::error::ErrorNotFound("File not found"));
    }
    
    println!("Serving file: {}", canonicalized_file_path.display());
    
    // Open and serve the file
    let file = NamedFile::open(&canonicalized_file_path)?;
    
    let mut response = file.into_response(&req);
    
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    );
    response.headers_mut().insert(
        header::PRAGMA,
        header::HeaderValue::from_static("no-cache"),
    );
    response.headers_mut().insert(
        header::EXPIRES,
        header::HeaderValue::from_static("0"),
    );
    
    Ok(response)
}

#[get("/all-images")]
async fn get_all_images(data: actix_web::web::Data<AppState>) -> actix_web::Result<HttpResponse> {
    // Get all image files in the images directory
    let entries = get_image_entries(&data.image_dir)?;

    if entries.is_empty() {
        return Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body("<html><body><h1>No images found</h1></body></html>"));
    }

    // Get current counter to determine next image
    let counter = data.counter.load(Ordering::SeqCst);
    let next_index = counter % entries.len();

    // Build HTML
    let mut html = String::from(
        "<!DOCTYPE html><html><head><meta charset='utf-8'>\
        <title>All Images</title>\
        <style>\
            body { font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }\
            h1 { color: #333; }\
            .next-indicator { background-color: #ffffcc; padding: 10px; margin: 10px 0; border-left: 4px solid #ffc107; }\
            .image-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 20px; }\
            .image-card { background: white; padding: 15px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }\
            .image-card.next { background-color: #fff9e6; border: 2px solid #ffc107; }\
            .image-card img { width: 100%; height: auto; border-radius: 4px; }\
            .image-info { margin-top: 10px; font-size: 14px; }\
            .image-name { font-weight: bold; word-break: break-word; margin: 5px 0; }\
            .image-date { color: #666; font-size: 13px; }\
        </style>\
        </head><body>\
        <h1>Image Gallery</h1>"
    );

    // Add next indicator
    html.push_str(&format!(
        "<div class='next-indicator'><strong>Next image to serve:</strong> {} (out of {})</div>",
        next_index + 1,
        entries.len()
    ));

    html.push_str("<div class='image-grid'>");

    // Add images
    for (index, path) in entries.iter().enumerate() {
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");

        // Get file metadata for modification time
        let modification_time = fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .ok();

        let date_str = modification_time
            .and_then(|time| {
                let datetime: DateTime<Local> = time.into();
                Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
            })
            .unwrap_or_else(|| "Unknown".to_string());

        let card_class = if index == next_index { "image-card next" } else { "image-card" };

        html.push_str(&format!(
            "<div class='{}'>\
                <img src='/file/{}' alt='{}'>\
                <div class='image-info'>\
                    <div class='image-name'>{}</div>\
                    <div class='image-date'>{}</div>\
                </div>\
            </div>",
            card_class,
            urlencoding::encode(filename),
            filename,
            filename,
            date_str
        ));
    }

    html.push_str("</div></body></html>");

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

// Extract the app setup into a separate function for testing
pub fn setup_app(cfg: &mut web::ServiceConfig) {
    cfg.service(get_image).service(get_control_panel).service(get_file).service(get_all_images);
}
