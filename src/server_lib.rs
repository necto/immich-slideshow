use actix_files::NamedFile;
use actix_web::{get, web, HttpRequest, HttpResponse, http::header};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fs;

pub struct AppState {
    pub counter: AtomicUsize,
    pub image_dir: String,
}

#[get("/image")]
async fn get_image(data: actix_web::web::Data<AppState>, req: HttpRequest) -> actix_web::Result<HttpResponse> {
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

// Extract the app setup into a separate function for testing
pub fn setup_app(cfg: &mut web::ServiceConfig) {
    cfg.service(get_image);
}
