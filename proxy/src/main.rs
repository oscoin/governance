use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;

use proxy::coco;
use proxy::env;
use proxy::http;
use proxy::registry;

/// Flags accepted by the proxy binary.
struct Args {
    /// Signaling which backend type to use.
    registry: String,
    /// Put proxy in test mode to use certain fixtures to serve.
    test: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_if_unset("RUST_BACKTRACE", "full");
    env::set_if_unset("RUST_LOG", "info");
    pretty_env_logger::init();

    let mut args = pico_args::Arguments::from_env();
    let args = Args {
        registry: args.value_from_str("--registry")?,
        test: args.contains("--test"),
    };

    let devnet_host = url17::Host::parse("35.241.138.91")?;
    let registry_client = match args.registry.as_str() {
        "devnet" => radicle_registry_client::Client::create_with_executor(devnet_host)
            .await
            .expect("unable to construct devnet client"),
        "emulator" => radicle_registry_client::Client::new_emulator(),
        _ => panic!(format!("unknown registry source '{}'", args.registry)),
    };

    let temp_dir = tempfile::tempdir().expect("test dir creation failed");
    let librad_paths = if args.test {
        let librad_paths =
            librad::paths::Paths::from_root(temp_dir.path()).expect("librad paths failed");

        coco::setup_fixtures(
            &librad_paths,
            temp_dir.path().to_str().expect("path extraction failed"),
        )
        .expect("fixture creation failed");

        librad_paths
    } else {
        librad::paths::Paths::new()?
    };
    let store = if args.test {
        kv::Store::new(kv::Config::new(temp_dir.path().join("store")))?
    } else {
        let dir = directories::ProjectDirs::from("xyz", "radicle", "upstream").unwrap();
        kv::Store::new(kv::Config::new(dir.data_dir().join("store")))?
    };

    log::info!("Starting API");

    let lib_paths = Arc::new(RwLock::new(librad_paths));
    let reg = Arc::new(RwLock::new(registry::Registry::new(registry_client)));
    let st = Arc::new(RwLock::new(store));
    let routes = http::routes(lib_paths, reg, st, args.test).with(
        warp::cors()
            .allow_any_origin()
            .allow_headers(&[warp::http::header::CONTENT_TYPE])
            .allow_methods(&[
                warp::http::Method::GET,
                warp::http::Method::POST,
                warp::http::Method::OPTIONS,
            ]),
    );

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;

    Ok(())
}
