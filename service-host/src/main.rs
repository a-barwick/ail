use std::ffi::OsString;
use std::fs;
use std::io;

use ail_service_host::{CatalogProvider, ServiceHost, canonical_config, canonical_workspace};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (catalog_path, port) = parse_arguments(std::env::args_os())?;
    let catalog = fs::read_to_string(&catalog_path)?;
    let provider = CatalogProvider::from_json(&catalog)?;
    let workspace = canonical_workspace().expect("canonical M32 source must compile");
    let config = canonical_config(&workspace).expect("r1 metadata must exist");
    let host = ServiceHost::new(&workspace, Box::new(provider), config)
        .expect("M32 pins must match compiler metadata");
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    axum::serve(listener, host.router()).await?;
    Ok(())
}

fn parse_arguments(
    mut arguments: impl Iterator<Item = OsString>,
) -> Result<(OsString, u16), io::Error> {
    let executable = arguments.next().unwrap_or_default();
    let catalog_path = arguments.next().ok_or_else(|| usage_error(&executable))?;
    let port = arguments
        .next()
        .map(|value| {
            value
                .to_str()
                .and_then(|value| value.parse::<u16>().ok())
                .filter(|port| *port != 0)
                .ok_or_else(|| usage_error(&executable))
        })
        .transpose()?
        .unwrap_or(3000);
    if arguments.next().is_some() {
        return Err(usage_error(&executable));
    }
    Ok((catalog_path, port))
}

fn usage_error(executable: &std::ffi::OsStr) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "usage: {} <catalog.json> [loopback-port]",
            executable.to_string_lossy()
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::parse_arguments;

    fn arguments<'a>(values: &'a [&'a str]) -> impl Iterator<Item = OsString> + 'a {
        values.iter().map(OsString::from)
    }

    #[test]
    fn explicit_loopback_port_is_optional_and_strict() {
        assert_eq!(
            parse_arguments(arguments(&["ail-service-host", "catalog.json"]))
                .unwrap()
                .1,
            3000
        );
        assert_eq!(
            parse_arguments(arguments(&["ail-service-host", "catalog.json", "3100"]))
                .unwrap()
                .1,
            3100
        );
        for invalid in ["0", "65536", "not-a-port"] {
            assert!(
                parse_arguments(arguments(&["ail-service-host", "catalog.json", invalid])).is_err()
            );
        }
        assert!(parse_arguments(arguments(&["ail-service-host"])).is_err());
        assert!(
            parse_arguments(arguments(&[
                "ail-service-host",
                "catalog.json",
                "3100",
                "extra"
            ]))
            .is_err()
        );
    }
}
