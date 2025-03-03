use actix_files::NamedFile;
use actix_web::{get, App, HttpServer, Result};
use std::path::PathBuf;

#[get("/image")]
async fn get_image() -> Result<NamedFile> {
    let path: PathBuf = "static/sample.png".into();
    Ok(NamedFile::open(path)?)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server at http://127.0.0.1:8080");
    println!("Access the image at http://127.0.0.1:8080/image");
    
    HttpServer::new(|| {
        App::new()
            .service(get_image)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
