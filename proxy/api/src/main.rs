use std::convert::TryFrom;

use coco::announcement;
use coco::keystore;
use coco::seed;
use coco::signer;

use api::config;
use api::context;
use api::env;
use api::http;
use api::notification;
use api::session;

/// Flags accepted by the proxy binary.
struct Args {
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
        test: args.contains("--test"),
    };

    let proxy_path = config::proxy_path()?;
    let bin_dir = config::bin_dir()?;
    coco::git_helper::setup(&proxy_path, &bin_dir)?;

    let temp_dir = tempfile::tempdir()?;
    log::debug!(
        "Temporary path being used for this run is: {:?}",
        temp_dir.path()
    );

    let paths_config = if args.test {
        std::env::set_var("RAD_HOME", temp_dir.path());
        coco::config::Paths::FromRoot(temp_dir.path().to_path_buf())
    } else {
        coco::config::Paths::default()
    };
    let paths = coco::Paths::try_from(paths_config)?;

    let store = {
        let store_path = if args.test {
            temp_dir.path().join("store")
        } else {
            let dirs = config::dirs();
            dirs.data_dir().join("store")
        };
        let config = kv::Config::new(store_path).flush_every_ms(100);

        kv::Store::new(config)?
    };

    let pw = keystore::SecUtf8::from("radicle-upstream");
    let mut keystore = keystore::Keystorage::new(&paths, pw);
    let key = keystore.init()?;
    let signer = signer::BoxedSigner::new(signer::SomeSigner {
        signer: key.clone(),
    });

    let peer_api = {
        let seeds = session::settings(&store).await?.coco.seeds;
        let seeds = seed::resolve(&seeds).await.unwrap_or_else(|err| {
            log::error!("Error parsing seed list {:?}: {}", seeds, err);
            vec![]
        });
        let config =
            coco::config::configure(paths, key.clone(), *coco::config::LOCALHOST_ANY, seeds);

        coco::Api::new(config).await?
    };

    if args.test {
        // TODO(xla): Given that we have proper ownership and user handling in coco, we should
        // evaluate how meaningful these fixtures are.
        let owner = peer_api.init_owner(&signer, "cloudhead")?;
        coco::control::setup_fixtures(&peer_api, &signer, &owner)?;
    }

    let subscriptions = notification::Subscriptions::default();

    let ctx = context::Ctx::from(context::Context {
        peer_api,
        signer,
        store,
    });

    {
        let ctx = ctx.clone();
        log::info!("Starting Announcement watcher");
        tokio::task::spawn(announcement_watcher(ctx));
    }

    log::info!("Starting API");
    let api = http::api(ctx, subscriptions, args.test);

    warp::serve(api).run(([127, 0, 0, 1], 8080)).await;

    Ok(())
}

async fn announcement_watcher(ctx: context::Ctx) {
    // TODO(xla): Take interval timings from config/settings.
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(10));

    loop {
        timer.tick().await;

        if let Err(err) = announce(ctx.clone()).await {
            log::info!("Announcement watcher errored: {:?}", err);
        }
    }
}

async fn announce(ctx: context::Ctx) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ctx.read().await;
    let old = session::announcements::load(&ctx.store)?;
    let new = announcement::build(&ctx.peer_api)?;
    let updates = announcement::diff(&old, &new);
    let count = updates.len();

    let updates = updates.clone();
    announcement::announce(ctx.peer_api.protocol(), updates.iter()).await?;

    session::announcements::save(&ctx.store, updates)?;
    log::debug!("announced {} updates", count);

    Ok(())
}
