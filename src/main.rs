use actix_files::NamedFile;
use actix_web::{get, App, HttpServer, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct AppState {
    counter: AtomicUsize,
    paths: Vec<String>,
}

#[get("/image")]
async fn get_image(data: actix_web::web::Data<AppState>) -> Result<NamedFile> {
    // Increment counter and check if it's odd or even
    let count = data.counter.fetch_add(1, Ordering::SeqCst);
    if count == data.paths.len() - 1 {
        *data.counter.get_mut() = 0;
    }
    assert!(count < data.paths.len());
    
    // Choose image based on odd/even count
    let path: PathBuf = data.paths[count].clone().into();
    
    Ok(NamedFile::open(path)?)
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
