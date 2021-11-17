// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

use std::{collections::HashSet, net::SocketAddr};

use anyhow::Context as _;
use futures::prelude::*;

use librad::{
    git::{replication, storage::fetcher, tracking},
    net::{
        discovery::{self, Discovery as _},
        protocol, Network,
    },
    paths::Paths,
    PeerId,
};
use link_identities::git::Urn;

type LibradPeer = librad::net::peer::Peer<librad::SecretKey>;

/// Configuration for creating a new [`Peer`].
#[derive(Clone)]
pub struct Config {
    pub rad_paths: Paths,
    pub key: librad::SecretKey,
    pub listen: SocketAddr,
}

/// Wrapper around [`librad::net::peer::Peer`] that provides seed specific functionality.
#[derive(Clone)]
pub struct Peer {
    librad_peer: LibradPeer,
}

impl Peer {
    /// Create a new client.
    pub fn new(config: Config) -> Self {
        let storage = librad::net::peer::config::Storage {
            protocol: librad::net::peer::config::ProtocolStorage {
                fetch_slot_wait_timeout: Default::default(),
                pool_size: 4,
            },
            user: librad::net::peer::config::UserStorage { pool_size: 4 },
        };

        let peer_config = librad::net::peer::Config {
            signer: config.key,
            protocol: protocol::Config {
                paths: config.rad_paths,
                listen_addr: config.listen,
                advertised_addrs: None, // TODO: Should we use this?
                membership: Default::default(),
                network: Network::Main,
                replication: replication::Config::default(),
                fetch: Default::default(),
                rate_limits: Default::default(),
            },
            storage,
        };
        let librad_peer = LibradPeer::new(peer_config).expect("failed to create peer");

        Self { librad_peer }
    }

    /// Run the peer by listening for incoming connections.
    ///
    /// Returns when `shutdown_signal` resolves or an error occurs.
    pub async fn run(
        &self,
        bootstrap: Vec<(PeerId, SocketAddr)>,
        shutdown_signal: impl Future<Output = ()>,
    ) -> anyhow::Result<()> {
        let librad_peer = self.librad_peer.clone();
        let static_discovery = discovery::Static::resolve(bootstrap)
            .context("failed to resolve bootstrap addresses")?;
        let shutdown_signal = shutdown_signal.shared();

        let bound = librad_peer
            .bind()
            .await
            .context("failed to bind librad peer")?;
        tracing::info!(addrs = ?bound.listen_addrs(), "peer bound");

        let (stop_accepting, listen) = bound.accept(static_discovery.clone().discover());
        let result = match future::select(shutdown_signal.clone(), listen.boxed()).await {
            future::Either::Left((_, listen)) => {
                stop_accepting();
                listen.await
            }
            future::Either::Right((listen_result, _)) => listen_result,
        };

        match result {
            // We called `stop_accepting`.
            Err(librad::net::protocol::io::error::Accept::Done) => {
                tracing::info!("peer stopped listening");
                Ok(())
            }
            Err(err) => Err(err).context("peer listening failed"),
            Ok(never) => never,
        }
    }

    /// Fetch and track indentity `urn` from a remote peer.
    ///
    /// If `addrs` is `None` the remote peer must already be connected so that we can discover its
    /// address. Otherwise an error is returned.
    #[tracing::instrument(skip(self, urn), fields(identity = %urn))]
    pub async fn fetch_identity_from_peer(
        &self,
        urn: Urn,
        peer_id: PeerId,
        addrs: Option<Vec<SocketAddr>>,
    ) -> anyhow::Result<()> {
        tracing::info!("start fetch identity");
        let addrs = if let Some(addrs) = addrs {
            addrs
        } else {
            let stats = self.librad_peer.stats().await;
            stats
                .connected_peers
                .get(&peer_id)
                .ok_or_else(|| anyhow::anyhow!("peer is not connected"))?
                .clone()
        };

        let cfg = self.librad_peer.protocol_config().replication;

        let replication_result = self
            .librad_peer
            .using_storage({
                let urn = urn.clone();
                move |storage| -> anyhow::Result<()> {
                    tracking::track(storage, &urn, peer_id).context("failed to track identity")?;

                    // Retry 20 times every 100ms.
                    let mut retries =
                        std::iter::repeat(std::time::Duration::from_millis(100)).take(20);

                    let fetcher = loop {
                        let fetcher_result =
                            fetcher::PeerToPeer::new(urn.clone(), peer_id, addrs.clone())
                                .build(storage)
                                .context("failed to build fetcher")?;

                        match fetcher_result {
                            Ok(fetcher) => break fetcher,
                            Err(_) => {
                                if let Some(delay) = retries.next() {
                                    std::thread::sleep(delay);
                                    tracing::debug!(%urn, %peer_id, "retrying fetch");
                                    continue;
                                } else {
                                    anyhow::bail!("building fetcher exceeded maximum retries")
                                }
                            }
                        }
                    };

                    replication::replicate(storage, fetcher, cfg, None)
                        .context("librad replication failed")?;
                    Ok(())
                }
            })
            .await??;

        tracing::info!(?replication_result, "fetch identity done");

        Ok(())
    }

    /// Returns stream that emits an item whenever the membership of the gossip layer changes.
    ///
    /// The stream never ends.
    pub fn membership(&self) -> impl Stream<Item = librad::net::peer::MembershipInfo> + 'static {
        self.events().filter_map({
            let librad_peer = self.librad_peer.clone();

            move |event| match event {
                librad::net::peer::ProtocolEvent::Membership(_) => {
                    let librad_peer = librad_peer.clone();
                    async move { Some(librad_peer.membership().await) }.left_future()
                }
                _ => futures::future::ready(None).right_future(),
            }
        })
    }

    /// Returns stream that emits the set of connected peers whenever it changes.
    ///
    /// The stream never ends.
    pub fn connected_peers(&self) -> impl Stream<Item = HashSet<PeerId>> + 'static {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        tokio_stream::wrappers::IntervalStream::new(interval)
            .then({
                let librad_peer = self.librad_peer.clone();

                move |_| {
                    let librad_peer = librad_peer.clone();
                    async move {
                        librad_peer
                            .stats()
                            .await
                            .connected_peers
                            .keys()
                            .copied()
                            .collect::<HashSet<_>>()
                    }
                }
            })
            .filter_map({
                let mut prev = HashSet::new();
                move |connected| {
                    futures::future::ready(if connected == prev {
                        None
                    } else {
                        prev = connected.clone();
                        Some(connected)
                    })
                }
            })
    }

    /// Stream that emits an item whenever new peers connect.
    ///
    /// The stream never ends.
    pub fn new_connections(&self) -> impl Stream<Item = Vec<PeerId>> + 'static {
        let mut prev_connected = HashSet::<PeerId>::new();
        self.connected_peers().filter_map(move |connected| {
            let added = connected
                .difference(&prev_connected)
                .copied()
                .collect::<Vec<_>>();
            prev_connected = connected;
            if added.is_empty() {
                future::ready(None)
            } else {
                future::ready(Some(added))
            }
        })
    }

    /// Broadcast “Have” gossip messages for all tracked peers in all projects.
    ///
    /// If getting the list of peers for one project or announcing this list for one project fails
    /// no error is returned and a message is logged instead.
    pub async fn announce_all_projects(&self) -> anyhow::Result<()> {
        let storage = self
            .librad_peer
            .storage()
            .await
            .context("failed to access librad storage")?;
        let projects =
            rad_identities::project::list(storage.as_ref()).context("failed to list projects")?;
        for project_result in projects {
            let project = match project_result {
                Ok(project) => project,
                Err(err) => {
                    tracing::error!(?err, "failed to read project");
                    continue;
                }
            };
            let urn = project.urn();

            let tracked_peers = match rad_identities::project::tracked(storage.as_ref(), &urn) {
                Ok(tracked_peers) => tracked_peers,
                Err(err) => {
                    tracing::error!(?err, %urn, "failed to get tracked peers");
                    continue;
                }
            };

            for peer_info in tracked_peers {
                let payload = librad::net::protocol::gossip::Payload {
                    urn: project.urn(),
                    rev: None,
                    origin: Some(peer_info.peer_id()),
                };
                tracing::debug!(?payload, "sending announcement");
                self.librad_peer
                    .announce(payload)
                    .map_err(|_| anyhow::anyhow!("librad peer not bound"))?;
            }
        }

        Ok(())
    }

    /// Stream of events from [`LibradPeer`].
    ///
    /// It’s not guaranteed that all peer events are delivered to the stream. If items from the
    /// stream are not processed in time events may be skipped.
    ///
    /// The stream will never end.
    fn events(
        &self,
    ) -> impl Stream<Item = librad::net::peer::ProtocolEvent> + Unpin + Send + 'static {
        self.librad_peer
            .subscribe()
            .scan((), |(), res| async move {
                use tokio::sync::broadcast::error::RecvError;
                match res {
                    Ok(item) => Some(Some(item)),
                    Err(err) => match err {
                        RecvError::Closed => None,
                        RecvError::Lagged(_) => {
                            tracing::warn!("skipped peer events");
                            Some(None)
                        }
                    },
                }
            })
            .filter_map(futures::future::ready)
            .boxed()
    }
}
