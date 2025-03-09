use clap::Parser;
use anyhow;

mod mock_immich_server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(long)]
    album_id: String,

    #[arg(long)]
    asset_id: String,

    #[arg(long)]
    test_image_path: String,

    #[arg(long)]
    port: Option<u16>,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("Starting mock Immich server with:");
    println!("Album ID: {}", args.album_id);
    println!("Asset ID: {}", args.asset_id);
    println!("Test Image: {}", args.test_image_path);

    // Start the server
    let addr = mock_immich_server::start_mock_server(
        &args.album_id,
        &args.asset_id,
        &args.test_image_path,
        args.port
    ).await?;
    
    println!("Mock server started on {}", addr);
    
    // Keep the server running until we receive a ctrl+c signal
    tokio::signal::ctrl_c().await?;
    println!("Shutting down mock server");
    
    Ok(())
}
