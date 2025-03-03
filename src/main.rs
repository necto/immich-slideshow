use actix_files::NamedFile;
use actix_web::{get, App, HttpServer, Result, HttpRequest, HttpResponse, http::header};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct AppState {
    counter: AtomicUsize,
    paths: Vec<String>,
}

#[get("/image")]
async fn get_image(data: actix_web::web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse> {
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
