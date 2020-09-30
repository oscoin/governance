//! Datastructure and machinery to safely share the common dependencies across components.

use coco::signer;

/// Container to pass down dependencies into HTTP filter chains.
#[derive(Clone)]
pub struct Context {
    /// [`coco::State`] to operate on the local monorepo.
    pub state: coco::State,
    /// [`coco::signer::BoxedSigner`] for write operations on the monorepo.
    pub signer: signer::BoxedSigner,
    /// [`kv::Store`] used for session state and cache.
    pub store: kv::Store,
}

impl Context {
    /// Initialises a new [`Context`] the given temporary directory.
    ///
    /// # Errors
    ///
    /// * coco key creation fails
    /// * creation of the [`kv::Store`] fails
    #[cfg(test)]
    pub async fn tmp(tmp_dir: &tempfile::TempDir) -> Result<Self, crate::error::Error> {
        use coco::{
            keystore,
            request::waiting_room::{self, WaitingRoom},
            shared::Shared,
            RunConfig,
        };

        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;

        let pw = keystore::SecUtf8::from("radicle-upstream");
        let key = keystore::Keystorage::memory(pw)?.get();
        let signer = signer::BoxedSigner::from(signer::SomeSigner { signer: key });

        let waiting_room = Shared::from(WaitingRoom::new(waiting_room::Config::default()));
        let (_peer, state) = {
            let config = coco::config::default(key, tmp_dir.path())?;
            coco::into_peer_state(
                config,
                signer.clone(),
                store.clone(),
                waiting_room,
                RunConfig::default(),
            )
            .await?
        };

        Ok(Self {
            state,
            signer,
            store,
        })
    }
}
