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
    pub image_order_file: String,
}

#[get("/image")]
async fn get_image(data: actix_web::web::Data<AppState>, req: HttpRequest) -> actix_web::Result<HttpResponse> {
    // Store GET parameters if any
    let query_string = req.query_string();
    if !query_string.is_empty() {
        let _ = store_parameters(&data.params_file, query_string);
    }

    // Get current counter before loading entries (so new images are inserted after current position)
    let counter = data.counter.load(Ordering::SeqCst);
    
    // Get all image files in the images directory (in order)
    let entries = get_image_entries(&data.image_dir, &data.image_order_file, counter)?;

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

/// Get all image files from the image directory in the order specified in the order file
/// New images are inserted right after the current position (next image to serve)
fn get_image_entries(image_dir: &str, image_order_file: &str, current_counter: usize) -> actix_web::Result<Vec<PathBuf>> {
    // Get all available files from the directory (excluding the order file itself and params file)
    let order_filename = Path::new(image_order_file)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image_order.json");
    
    let available_files: Vec<String> = fs::read_dir(image_dir)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.is_file() {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .and_then(|s| {
                            // Exclude metadata files
                            if s == order_filename || s == "params.json" {
                                None
                            } else {
                                Some(s.to_string())
                            }
                        })
                } else {
                    None
                }
            })
        })
        .collect();

    // Load or initialize the order list
    let mut order_list: Vec<String> = if Path::new(image_order_file).exists() {
        match fs::read_to_string(image_order_file) {
            Ok(content) => {
                serde_json::from_str(&content)
                    .unwrap_or_else(|_| available_files.clone())
            }
            Err(_) => available_files.clone(),
        }
    } else {
        available_files.clone()
    };

    // Remove files that no longer exist, keep order of remaining files
    order_list.retain(|f| available_files.contains(f));

    // Add any new files that appeared in the directory
    // Insert them right after the current position instead of at the end
    let new_files: Vec<String> = available_files
        .iter()
        .filter(|f| !order_list.contains(f))
        .cloned()
        .collect();

    if !new_files.is_empty() {
        // Calculate the insertion point: right after the current/next image
        let insert_position = if order_list.is_empty() {
            0
        } else {
            let next_index = current_counter % order_list.len();
            next_index + 1
        };

        // Insert new files at the calculated position
        for (i, file) in new_files.into_iter().enumerate() {
            order_list.insert(insert_position + i, file);
        }
    }

    // Save the updated order
    let _ = fs::write(image_order_file, serde_json::to_string_pretty(&order_list).unwrap_or_default());

    // Convert to PathBuf
    let entries = order_list
        .into_iter()
        .map(|filename| Path::new(image_dir).join(filename))
        .collect();

    Ok(entries)
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Reorder images by moving an image to a specific position in the order list
/// Returns an error if the image is not found in the order list
fn reorder_images(image_order_file: &str, image_name: &str, target_position: usize) -> Result<(), String> {
    // Load the current order list
    let mut order_list: Vec<String> = if Path::new(image_order_file).exists() {
        let content = fs::read_to_string(image_order_file)
            .map_err(|e| format!("Failed to read order file: {}", e))?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        return Err("Order file not found".to_string());
    };

    // Find and remove the image from its current position
    if let Some(current_pos) = order_list.iter().position(|f| f == image_name) {
        order_list.remove(current_pos);
    } else {
        return Err(format!("Image '{}' not found in order list", image_name));
    }

    // Insert at the target position
    let insert_pos = std::cmp::min(target_position, order_list.len());
    order_list.insert(insert_pos, image_name.to_string());

    // Save the updated order
    fs::write(image_order_file, serde_json::to_string_pretty(&order_list)
              .map_err(|e| format!("Failed to serialize order list: {}", e))?)
        .map_err(|e| format!("Failed to write order file: {}", e))?;
    Ok(())
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
    
    let response = file.into_response(&req);
    
    Ok(response)
}

#[get("/all-images")]
async fn get_all_images(data: actix_web::web::Data<AppState>, req: HttpRequest) -> actix_web::Result<HttpResponse> {
    // Handle reordering if parameters are provided
    let query_string = req.query_string();
    if !query_string.is_empty() {
        let mut move_to: Option<usize> = None;
        let mut image_name: Option<String> = None;
        let mut next_index: Option<usize> = None;

        for pair in query_string.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                match key {
                    "move-to" => {
                        move_to = value.parse().ok();
                    }
                    "image-name" => {
                        image_name = urlencoding::decode(value)
                            .ok()
                            .map(|s| s.to_string());
                    }
                    "next-index" => {
                        next_index = value.parse().ok();
                    }
                    _ => {}
                }
            }
        }

        if let (Some(target_pos), Some(name)) = (move_to, image_name) {
            match reorder_images(&data.image_order_file, &name, target_pos) {
                Err(err) => {
                    return Ok(HttpResponse::BadRequest()
                        .content_type("text/html; charset=utf-8")
                        .body(format!(
                            "<html><body><h1>Error</h1><p>{}</p><p><a href='/all-images'>Back</a></p></body></html>",
                            html_escape(&err)
                        )))
                }
                Ok(_) => {}
            }
        }

        if let Some(idx) = next_index {
            data.counter.store(idx, Ordering::SeqCst);
        }
    }

    // Get all image files in the images directory (in order)
    // Pass counter so new images are inserted after current position
    let counter = data.counter.load(Ordering::SeqCst);
    let entries = get_image_entries(&data.image_dir, &data.image_order_file, counter)?;

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
            .image-actions { margin-top: 10px; display: flex; gap: 8px; flex-wrap: wrap; }\
            .set-next-btn { background-color: #4CAF50; color: white; padding: 8px 12px; border: none; border-radius: 4px; cursor: pointer; font-size: 12px; text-decoration: none; display: inline-block; }\
            .set-next-btn:hover { background-color: #45a049; }\
            .image-card.next .set-next-btn { background-color: #ffc107; color: #333; }\
            .image-card.next .set-next-btn:hover { background-color: #ffb300; }\
            .move-btn { background-color: #2196F3; color: white; padding: 6px 10px; border: none; border-radius: 4px; cursor: pointer; font-size: 11px; text-decoration: none; display: inline-block; }\
            .move-btn:hover { background-color: #0b7dda; }\
            .move-btn:disabled, .move-btn[disabled] { background-color: #ccc; cursor: not-allowed; }\
            .move-btn.disabled { pointer-events: none; background-color: #ccc; }\
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

        // Build move buttons
        let mut move_buttons = String::new();
        
        // Move left button (only if not first)
        if index > 0 {
            move_buttons.push_str(&format!(
                "<a href='/all-images?image-name={}&move-to={}' class='move-btn' title='Left'>←</a>",
                urlencoding::encode(filename),
                index - 1
            ));
        } else {
            move_buttons.push_str("<span class='move-btn disabled' title='Left'>←</span>");
        }
        
        // Move right button (only if not last)
        if index < entries.len() - 1 {
            move_buttons.push_str(&format!(
                "<a href='/all-images?image-name={}&move-to={}' class='move-btn' title='Right'>→</a>",
                urlencoding::encode(filename),
                index + 1
            ));
        } else {
            move_buttons.push_str("<span class='move-btn disabled' title='Right'>→</span>");
        }
        
        // Move to after current image button (only if not already after current)
        if index != next_index + 1 && index != next_index {
            // If moving an image from before current to after current, we need to adjust the current index
            // to keep the same image highlighted (decrease by 1 because removal shifts indices)
            let new_next_index = if index < next_index { next_index - 1 } else { next_index };
            let after_current_pos = if index < next_index { next_index  } else { next_index + 1 };
            move_buttons.push_str(&format!(
                "<a href='/all-images?image-name={}&move-to={}&next-index={}' class='move-btn' title='After Current'>↯</a>",
                urlencoding::encode(filename),
                after_current_pos,
                new_next_index
            ));
        }
        
        // Move to begin button (only if not already at begin)
        if index > 0 {
            // If moving an image from after current to before current, we need to adjust the current index
            // (the image we removed shifts indices, so increment by 1)
            let new_next_index = if index <= next_index { next_index + 1 } else { next_index };
            move_buttons.push_str(&format!(
                "<a href='/all-images?image-name={}&move-to={}&next-index={}' class='move-btn' title='To Begin'>⤒</a>",
                urlencoding::encode(filename),
                0,
                new_next_index
            ));
        }
        
        // Move to end button (only if not already at end)
        if index < entries.len() - 1 {
            // If moving an image from before current to after current, we need to adjust the current index
            // (the image we removed shifts indices, so decrement by 1)
            let new_next_index = if index < next_index { next_index - 1 } else { next_index };
            move_buttons.push_str(&format!(
                "<a href='/all-images?image-name={}&move-to={}&next-index={}' class='move-btn' title='To End'>⤓</a>",
                urlencoding::encode(filename),
                entries.len() - 1,
                new_next_index
            ));
        }

        html.push_str(&format!(
            "<div class='{}'>\
                <img src='/file/{}' alt='{}'>\
                <div class='image-info'>\
                    <div class='image-name'>{}</div>\
                    <div class='image-date'>{}</div>\
                </div>\
                <div class='image-actions'>\
                    <a href='/all-images?next-index={}' class='set-next-btn'>Set as Next</a>\
                    {}\
                </div>\
            </div>",
            card_class,
            urlencoding::encode(filename),
            filename,
            filename,
            date_str,
            index,
            move_buttons
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
