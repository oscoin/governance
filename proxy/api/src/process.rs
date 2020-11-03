//! Provides [`run`] to run the proxy process.
use futures::prelude::*;
use std::{convert::TryFrom, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::{watch, RwLock},
};

use coco::{convert::MaybeFrom as _, seed, signer, Peer, RunConfig, SyncConfig};

use crate::{config, context, http, notification, service, session};

/// Flags accepted by the proxy binary.
#[derive(Clone, Copy)]
pub struct Args {
    /// Put proxy in test mode to use certain fixtures.
    pub test: bool,
}

/// Data required to run the peer and the API
struct Rigging {
    /// The context provided to the API
    ctx: context::Context,
    /// The [`Peer`] to run
    peer: Option<Peer>,
    /// Channel to receive updates to the seed nodes from the API
    seeds_sender: Option<watch::Sender<Vec<seed::Seed>>>,
}

/// Run the proxy process
///
/// # Errors
///
/// Errors when the setup or any of the services fatally fails.
pub async fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    // Required for `tokio::select`. We can’t put it on the element directly, though.
    #![allow(clippy::unreachable)]

    let proxy_path = config::proxy_path()?;
    let bin_dir = config::bin_dir()?;
    coco::git_helper::setup(&proxy_path, &bin_dir)?;

    let mut service_manager = service::Manager::new(args.test)?;
    let mut sighup = signal(SignalKind::hangup())?;

    let mut handle = service_manager.handle();
    tokio::spawn(async move {
        loop {
            if sighup.recv().await.is_some() {
                log::info!("SIGHUP received, reloading...");
                handle.reset();
            } else {
                break;
            }
        }
    });

    let auth_token = Arc::new(RwLock::new(None));
    loop {
        let notified_restart = service_manager.notified_restart();
        let service_handle = service_manager.handle();
        let environment = service_manager.environment()?;
        let rigging = rig(args, service_handle, environment, auth_token.clone()).await?;
        let result = run_rigging(rigging, notified_restart).await;
        match result {
            // We've been shut down, ignore
            Err(RunError::Peer(coco::peer::Error::Spawn(_))) | Ok(()) => log::debug!("aborted"),
            // Actual error, abort the process
            Err(e) => return Err(e.into()),
        }

        // Give `coco::SpawnAbortable` some time to release all the resources.
        // See https://github.com/radicle-dev/radicle-upstream/issues/1163
        tokio::time::delay_for(Duration::from_millis(50)).await
    }
}

/// Error running either the peer or the API.
#[derive(Debug, Error)]
enum RunError {
    /// The peer errored
    #[error(transparent)]
    Peer(#[from] coco::peer::Error),

    /// Warp errored
    #[error(transparent)]
    Warp(#[from] warp::Error),
}

/// Run the API and peer.
///
/// Returns when either the peer or the API stops.
///
/// # Errors
///
/// Errors when either the peer or the API error.
async fn run_rigging(
    rigging: Rigging,
    restart_signal: impl Future<Output = ()> + Send + 'static,
) -> Result<(), RunError> {
    // Required for `tokio::select`. We can’t put it on the element directly, though.
    #![allow(clippy::unreachable)]
    let Rigging {
        ctx,
        peer,
        seeds_sender,
    } = rigging;

    if let Some(seeds_sender) = seeds_sender {
        let seeds_store = ctx.store().clone();
        coco::SpawnAbortable::new(async move {
            let mut last_seeds: Vec<seed::Seed> = vec![];
            let mut timer = tokio::time::interval(Duration::from_secs(1));

            loop {
                let _timestamp = timer.tick().await;

                let seeds = session_seeds(&seeds_store)
                    .await
                    .expect("Failed to read session store");

                if seeds == last_seeds {
                    continue;
                }

                if seeds_sender.broadcast(seeds.clone()).is_err() {
                    break;
                }

                last_seeds = seeds;
            }
        });
    }

    let subscriptions = notification::Subscriptions::default();
    let peer_subscriptions = subscriptions.clone();

    let server = async move {
        log::info!("starting API");
        let api = http::api(ctx, subscriptions.clone());
        let (_, server) = warp::serve(api).try_bind_with_graceful_shutdown(
            ([127, 0, 0, 1], 8080),
            async move {
                restart_signal.await;
                subscriptions.clear().await;
            },
        )?;

        server.await;
        Ok(())
    };

    if let Some(peer) = peer {
        coco::SpawnAbortable::new({
            let mut peer_events = peer.subscribe();

            async move {
                loop {
                    if let Some(notification) = notification::Notification::maybe_from(
                        peer_events
                            .recv()
                            .await
                            .expect("Failed to receive peer event"),
                    ) {
                        peer_subscriptions.broadcast(notification).await
                    }
                }
            }
        });
        let peer = async move {
            log::info!("starting peer");
            peer.into_running().await
        };

        let result = tokio::select! {
            server_status = server => server_status,
            peer_status = peer => Ok(peer_status?),
        };
        result
    } else {
        server.await
    }
}

lazy_static::lazy_static! {
    /// Fixed key to use in test mode
    static ref TEST_KEY: coco::keys::SecretKey = coco::keys::SecretKey::new();
}

/// Create [`Rigging`] to run the peer and API.
async fn rig(
    args: Args,
    service_handle: service::Handle,
    environment: &service::Environment,
    auth_token: Arc<RwLock<Option<String>>>,
) -> Result<Rigging, Box<dyn std::error::Error>> {
    let (paths, store) = if let Some(temp_dir) = &environment.temp_dir {
        std::env::set_var("RAD_HOME", temp_dir.path());
        let paths =
            coco::Paths::try_from(coco::config::Paths::FromRoot(temp_dir.path().to_path_buf()))?;
        let store = {
            let path = temp_dir.path().join("store");
            kv::Store::new(kv::Config::new(path).flush_every_ms(100))
        }?;
        (paths, store)
    } else {
        let paths = coco::Paths::try_from(coco::config::Paths::default())?;
        let store = {
            let path = config::dirs().data_dir().join("store");
            kv::Store::new(kv::Config::new(path).flush_every_ms(100))
        }?;
        (paths, store)
    };

    if let Some(_key) = environment.key {
        // We ignore `environment.key` for now and use a hard-coded passphrase
        let pw = coco::keystore::SecUtf8::from("radicle-upstream");
        let key = if args.test {
            *TEST_KEY
        } else {
            coco::keystore::Keystorage::file(&paths, pw).init()?
        };
        let signer = signer::BoxedSigner::new(signer::SomeSigner { signer: key });

        let (peer, state, seeds_sender) = if args.test {
            let config = coco::config::configure(
                paths,
                key,
                *coco::config::INADDR_ANY,
                coco::config::static_seed_discovery(vec![]),
            );
            let (peer, state) =
                coco::into_peer_state(config, signer.clone(), store.clone(), coco_run_config())
                    .await?;

            (peer, state, None)
        } else {
            let seeds = session_seeds(&store).await?;
            let (seeds_sender, seeds_receiver) = watch::channel(seeds);

            let config = coco::config::configure(
                paths,
                key,
                *coco::config::INADDR_ANY,
                coco::config::StreamDiscovery::new(seeds_receiver),
            );

            let (peer, state) =
                coco::into_peer_state(config, signer.clone(), store.clone(), coco_run_config())
                    .await?;

            (peer, state, Some(seeds_sender))
        };

        let peer_control = peer.control();
        let ctx = context::Context::Unsealed(context::Unsealed {
            peer_control,
            state,
            store,
            test: args.test,
            service_handle: service_handle.clone(),
            auth_token,
        });

        Ok(Rigging {
            ctx,
            peer: Some(peer),
            seeds_sender,
        })
    } else {
        let ctx = context::Context::Sealed(context::Sealed {
            store,
            test: args.test,
            service_handle,
            auth_token,
        });
        Ok(Rigging {
            ctx,
            peer: None,
            seeds_sender: None,
        })
    }
}

/// Get and resolve seed settings from the session store.
async fn session_seeds(
    store: &kv::Store,
) -> Result<Vec<coco::seed::Seed>, Box<dyn std::error::Error>> {
    let seeds = session::seeds(store).await?;
    Ok(seed::resolve(&seeds).await.unwrap_or_else(|err| {
        log::error!("Error parsing seed list {:?}: {}", seeds, err);
        vec![]
    }))
}

/// [`RunConfig`] for the coco peer.
fn coco_run_config() -> RunConfig {
    RunConfig {
        sync: SyncConfig {
            max_peers: 1,
            on_startup: true,
            period: Duration::from_secs(5),
        },
        ..RunConfig::default()
    }
}
