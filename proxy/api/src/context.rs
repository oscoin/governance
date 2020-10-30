//! Datastructure and machinery to safely share the common dependencies across components.

use std::sync::Arc;

use tokio::sync::RwLock;

use coco::PeerControl;

use crate::service;

#[cfg(test)]
use coco::{keystore, signer, RunConfig};

/// Container to pass down dependencies into HTTP filter chains.
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Context {
    /// Coco peer API is sealed and unavailable
    Sealed(Sealed),
    /// Coco peer API is unsealed and available
    Unsealed(Unsealed),
}

impl Context {
    /// Returns `true` if the stack is set up in test mode.
    pub fn test(&self) -> bool {
        match self {
            Self::Sealed(sealed) => sealed.test,
            Self::Unsealed(unsealed) => unsealed.test,
        }
    }

    /// Returns the [`kv::Store`] for persistent storage.
    pub fn store(&self) -> &kv::Store {
        match self {
            Self::Sealed(sealed) => &sealed.store,
            Self::Unsealed(unsealed) => &unsealed.store,
        }
    }

    /// Returns a mutable reference to the authentication cookie value.
    pub fn auth_token(&self) -> Arc<RwLock<Option<String>>> {
        match self {
            Self::Sealed(sealed) => sealed.auth_token.clone(),
            Self::Unsealed(unsealed) => unsealed.auth_token.clone(),
        }
    }

    /// Returns a handle to control the service configuration
    pub fn service_handle(&mut self) -> &mut service::Handle {
        match self {
            Self::Sealed(sealed) => &mut sealed.service_handle,
            Self::Unsealed(unsealed) => &mut unsealed.service_handle,
        }
    }
}

impl From<Unsealed> for Context {
    fn from(unsealed: Unsealed) -> Self {
        Self::Unsealed(unsealed)
    }
}

impl From<Sealed> for Context {
    fn from(sealed: Sealed) -> Self {
        Self::Sealed(sealed)
    }
}

/// Context for HTTP requests with access to coco peer APIs.
#[derive(Clone)]
pub struct Unsealed {
    /// Handle to inspect state and perform actions on the currently running local [`coco::Peer`].
    pub peer_control: PeerControl,
    /// [`coco::State`] to operate on the local monorepo.
    pub state: coco::State,
    /// [`kv::Store`] used for session state and cache.
    pub store: kv::Store,
    /// Flag to control if the stack is set up in test mode.
    pub test: bool,
    /// Handle to control the service configuration.
    pub service_handle: service::Handle,
    /// Cookie set on unsealing the key store.
    pub auth_token: Arc<RwLock<Option<String>>>,
}

/// Context for HTTP request if the coco peer APIs have not been initialized yet.
#[derive(Clone)]
pub struct Sealed {
    /// [`kv::Store`] used for session state and cache.
    pub store: kv::Store,
    /// Flag to control if the stack is set up in test mode.
    pub test: bool,
    /// Handle to control the service configuration.
    pub service_handle: service::Handle,
    /// Cookie set on unsealing the key store.
    pub auth_token: Arc<RwLock<Option<String>>>,
}

impl Unsealed {
    /// Initialises a new [`Unsealed`] context with the store and coco state in the given temporary
    /// directory.
    ///
    /// # Errors
    ///
    /// * coco key creation fails
    /// * creation of the [`kv::Store`] fails
    #[cfg(test)]
    pub async fn tmp(tmp_dir: &tempfile::TempDir) -> Result<Self, crate::error::Error> {
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;

        let pw = keystore::SecUtf8::from("radicle-upstream");
        let key = keystore::Keystorage::memory(pw)?.get();
        let signer = signer::BoxedSigner::from(signer::SomeSigner { signer: key });

        let (peer_control, state) = {
            let config = coco::config::default(key, tmp_dir.path())?;
            let (peer, state) =
                coco::into_peer_state(config, signer.clone(), store.clone(), RunConfig::default())
                    .await?;

            let peer_control = peer.control();
            tokio::spawn(peer.into_running());

            (peer_control, state)
        };

        Ok(Self {
            peer_control,
            state,
            store,
            test: false,
            service_handle: service::Handle::dummy(),
            auth_token: Arc::new(RwLock::new(None)),
        })
    }
}
