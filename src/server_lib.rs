use actix_files::NamedFile;
use actix_web::{get, web, HttpRequest, HttpResponse, http::header};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fs;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

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

    // Get all files in the images directory
    let entries = fs::read_dir(&data.image_dir)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.is_file() {
                    Some(path.to_string_lossy().to_string())
                } else {
                    None
                }
            })
        })
        .collect::<Vec<String>>();

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
    let path: PathBuf = entries[index].clone().into();
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

// Extract the app setup into a separate function for testing
pub fn setup_app(cfg: &mut web::ServiceConfig) {
    cfg.service(get_image).service(get_control_panel);
}
