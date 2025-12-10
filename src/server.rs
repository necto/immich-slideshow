use actix_web::{App, HttpServer, web};
use clap::Parser;
use dotenv::dotenv;
use image_server_lib::server_lib::{AppState, setup_app};
use std::sync::atomic::AtomicUsize;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory containing images to serve
    #[arg(long, env = "IMAGE_DIR", default_value = "images")]
    image_dir: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file if present
    dotenv().ok();

    // Parse command line arguments
    let args = Args::parse();

    println!("Starting server at http://0.0.0.0:8080");
    println!("Access the image at http://0.0.0.0:8080/image");
    println!("The server will cycle through all images in the {} directory", args.image_dir);

    // Create and share application state
    let app_state = web::Data::new(AppState {
        counter: AtomicUsize::new(0),
        image_dir: args.image_dir,
        params_file: "params.json".to_string(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .configure(setup_app)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
