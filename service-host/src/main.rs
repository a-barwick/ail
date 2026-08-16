use std::fs;
use std::io;

use ail_service_host::{CatalogProvider, ServiceHost, canonical_config, canonical_workspace};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os();
    let executable = arguments.next().unwrap_or_default();
    let catalog_path = arguments.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage: {} <catalog.json>", executable.to_string_lossy()),
        )
    })?;
    if arguments.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage: {} <catalog.json>", executable.to_string_lossy()),
        )
        .into());
    }
    let catalog = fs::read_to_string(&catalog_path)?;
    let provider = CatalogProvider::from_json(&catalog)?;
    let workspace = canonical_workspace().expect("canonical M32 source must compile");
    let config = canonical_config(&workspace).expect("r1 metadata must exist");
    let host = ServiceHost::new(&workspace, Box::new(provider), config)
        .expect("M32 pins must match compiler metadata");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, host.router()).await?;
    Ok(())
}
