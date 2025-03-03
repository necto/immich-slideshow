use actix_files::NamedFile;
use actix_web::{get, App, HttpServer, Result, HttpResponse, http::header};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct AppState {
    counter: AtomicUsize,
    paths: Vec<String>,
}

#[get("/image")]
async fn get_image(data: actix_web::web::Data<AppState>) -> Result<HttpResponse> {
    // Increment counter and get current value
    let count = data.counter.fetch_add(1, Ordering::SeqCst);
    
    // Reset counter if we've reached the end of the paths
    if count == data.paths.len() - 1 {
        data.counter.store(0, Ordering::SeqCst);
    }
    
    // Ensure we're within bounds
    assert!(count < data.paths.len());
    
    // Choose image based on odd/even count
    let path: PathBuf = data.paths[count].clone().into();
    
    // Open the file
    let file = NamedFile::open(path)?;
    
    // Create a response with no-cache headers
    let response = file
        .into_response(&actix_web::HttpRequest::default())
        .map_into_boxed_body()
        .into_parts()
        .0;
    
    // Add cache control headers to prevent caching
    Ok(response
        .set_header(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate, max-age=0")
        .set_header(header::PRAGMA, "no-cache")
        .set_header(header::EXPIRES, "0"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server at http://127.0.0.1:8080");
    println!("Access the image at http://127.0.0.1:8080/image");
    println!("The server will alternate between sample.png and sample-flipped.png");
    
    // Create and share application state
    let app_state = actix_web::web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        paths: vec!["static/sample.png".into(), "static/sample-flipped.png".into()]
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
