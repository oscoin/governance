// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

//! Proxy library errors usable for caller control flow and additional context for API responses.

use std::io;

use crate::keystore;

/// All error variants the API will return.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Keystore error.
    #[error(transparent)]
    Keystore(#[from] keystore::Error),

    /// Error interacting with [`radicle_daemon::net::peer::Peer`].
    #[error(transparent)]
    State(#[from] radicle_daemon::state::Error),

    #[error(transparent)]
    Source(#[from] radicle_source::error::Error),

    #[error(transparent)]
    Peer(#[from] radicle_daemon::peer::Error),

    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] io::Error),

    /// Issues when access persistent storage.
    #[error(transparent)]
    Store(#[from] kv::Error),

    /// The key store is sealed
    #[error("keystore is sealed")]
    KeystoreSealed,

    /// The request auth token differs from the one in context
    #[error("invalid authentication token")]
    InvalidAuthCookie,

    /// Errors stemming from [`radicle_daemon::request::waiting_room::WaitingRoom`] interactions.
    #[error(transparent)]
    WaitingRoom(#[from] radicle_daemon::request::waiting_room::Error),

    #[error("project not found")]
    ProjectNotFound,

    #[error("missing default branch")]
    MissingDefaultBranch,
}
