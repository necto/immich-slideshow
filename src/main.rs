use actix_files::NamedFile;
use actix_web::{get, App, HttpServer, Result, HttpRequest, HttpResponse, http::header};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fs;

struct AppState {
    counter: AtomicUsize,
}

const IMAGE_DIR : &str = "images";

#[get("/image")]
async fn get_image(data: actix_web::web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse> {
    // Get all files in the static directory
    let entries = fs::read_dir(IMAGE_DIR)
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server at http://0.0.0.0:8080");
    println!("Access the image at http://0.0.0.0:8080/image");
    println!("The server will cycle through all images in the {} directory", IMAGE_DIR);
    
    // Create and share application state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
    });
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(get_image)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
