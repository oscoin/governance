//! Compute, track and announce noteworthy changes to the network.

use std::{collections::HashSet, ops::Deref as _};

use kv::Codec as _;

use librad::uri::{path::ParseError, Path, RadUrn};

use crate::{
    oid::Oid,
    peer::gossip,
    state::{self, State},
};

/// Name for the bucket used in [`kv::Store`].
const BUCKET_NAME: &str = "announcements";
/// Key for the single value used as cache.
const KEY_NAME: &str = "latest";

/// Announcement errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failures from [`kv`].
    #[error(transparent)]
    Kv(#[from] kv::Error),

    /// Failures parsing.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Error occurred when interacting with [`State`].
    #[error(transparent)]
    State(#[from] state::Error),
}

/// An update and all the required information that can be announced on the network.
pub type Announcement = (RadUrn, Oid);

/// Unique list of [`Announcement`]s.
pub type Updates = HashSet<Announcement>;

/// Announces the list of given `updates` with the [`librad::net::protocol`].
///
/// # Errors
///
/// * if the announcemnet of one of the project heads failed
pub async fn announce(state: &State, updates: impl Iterator<Item = &Announcement> + Send) {
    for (urn, hash) in updates {
        gossip::announce(state, urn, hash).await;
    }
}

/// Builds the latest list of [`Announcement`]s for the current state of the peer.
///
/// # Errors
///
/// * if listing of the projects fails
/// * if listing of the Refs for a project fails
pub async fn build(state: &State) -> Result<Updates, Error> {
    let mut list: Updates = HashSet::new();

    match state.list_projects().await {
        // TODO(xla): We need to avoid the case where there is no owner yet for the peer api, there
        // should be machinery to kick off these routines only if our app state is ready for it.
        Err(crate::state::Error::Storage(librad::git::storage::Error::Config(_err))) => Ok(list),
        Err(err) => Err(err.into()),
        Ok(projects) => {
            for project in &projects {
                let refs = state.list_owner_project_refs(project.urn()).await?;

                for (head, hash) in &refs.heads {
                    list.insert((
                        RadUrn {
                            path: head.parse::<Path>()?,
                            ..project.urn()
                        },
                        Oid::from(*hash.deref()),
                    ));
                }
            }

            Ok(list)
        },
    }
}

/// Computes the list of announcements based on the difference of the `new` and `old` state. An
/// [`Announcement`] will be included if an entry in `new` can't be found in `old`.
#[allow(clippy::implicit_hasher)]
#[must_use]
pub fn diff<'a>(old_state: &'a Updates, new_state: &'a Updates) -> Updates {
    new_state.difference(old_state).cloned().collect()
}

/// Load the cached announcements from the [`kv::Store`].
///
/// # Errors
///
/// * if the [`kv::Bucket`] can't be accessed
/// * if the access of the key in the [`kv::Bucket`] fails
pub fn load(store: &kv::Store) -> Result<Updates, Error> {
    let bucket = store.bucket::<&'static str, kv::Json<Updates>>(Some(BUCKET_NAME))?;
    let value = bucket
        .get(KEY_NAME)?
        .map_or(HashSet::new(), kv::Json::to_inner);

    Ok(value)
}

/// Runs the entire announcement procedure.
///
/// # Errors
///
/// * if it can't build the new list of updates
/// * access to the storage fails
pub async fn run(state: &State, store: &kv::Store) -> Result<Updates, Error> {
    let old = load(store)?;
    let new = build(state).await?;
    let updates = diff(&old, &new);

    announce(state, updates.iter()).await;

    if !updates.is_empty() {
        save(store, updates.clone()).map_err(Error::from)?;
    }

    Ok(updates)
}

/// Update the cache with the latest announcements.
///
/// # Errors
///
/// * if the [`kv::Bucket`] can't be accessed
/// * if the storage of the new updates fails
#[allow(clippy::implicit_hasher)]
pub fn save(store: &kv::Store, updates: Updates) -> Result<(), Error> {
    let bucket = store.bucket::<&'static str, kv::Json<Updates>>(Some(BUCKET_NAME))?;
    bucket.set(KEY_NAME, kv::Json(updates)).map_err(Error::from)
}

#[allow(clippy::panic)]
#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use pretty_assertions::assert_eq;

    use librad::{hash::Hash, keys::SecretKey, uri};

    use crate::{config, oid, signer, state::State};

    #[tokio::test(core_threads = 2)]
    async fn announce() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir().expect("failed to create temdir");
        let key = SecretKey::new();
        let signer = signer::BoxedSigner::new(signer::SomeSigner { signer: key });
        let config = config::default(key, tmp_dir.path())?;
        let (api, _run_loop) = config.try_into_peer().await?.accept()?;
        let state = State::new(api, signer.clone());

        let _owner = state.init_owner(&signer, "cloudhead").await?;

        // TODO(xla): Build up proper testnet to assert that haves are announced.
        let updates = super::build(&state).await?;
        super::announce(&state, updates.iter()).await;

        Ok(())
    }

    #[test]
    fn diff() -> Result<(), Box<dyn std::error::Error>> {
        let both = vec![
            (project0("dev"), "68986574".parse::<oid::Oid>()?),
            (project0("master"), "c8d2ad44".parse::<oid::Oid>()?),
            (project0("stable"), "2d2e1408".parse::<oid::Oid>()?),
            (
                project0("cloudhead/cool-feature"),
                "68986574".parse::<oid::Oid>()?,
            ),
            (
                project0("fintohaps/doc-tests"),
                "f90353ba".parse::<oid::Oid>()?,
            ),
            (project1("dev"), "c8d2ad44".parse::<oid::Oid>()?),
            (project0("master"), "2d2e1408".parse::<oid::Oid>()?),
            (project1("stable"), "a3403e2d".parse::<oid::Oid>()?),
        ];
        let old = vec![
            (
                project0("igor/zero-assertions"),
                "72a78226".parse::<oid::Oid>()?,
            ),
            (project0("thoshol/remove"), "7c69d71a".parse::<oid::Oid>()?),
            (project1("rudolfs/release"), "8c085d58".parse::<oid::Oid>()?),
        ];
        let new = vec![
            (
                project0("cloudhead/new-language"),
                "7dec3269".parse::<oid::Oid>()?,
            ),
            (
                project0("fintohaps/notations"),
                "b4d3276d".parse::<oid::Oid>()?,
            ),
            (
                project0("kalt/eat-my-impls"),
                "2206e5dc".parse::<oid::Oid>()?,
            ),
            (project1("backport"), "869e5740".parse::<oid::Oid>()?),
        ];

        let left: HashSet<_> = [&both[..], &old[..]].concat().iter().cloned().collect();
        let right: HashSet<_> = [&both[..], &new[..]].concat().iter().cloned().collect();
        let announcements = super::diff(&left, &right);

        assert_eq!(announcements, new.iter().cloned().collect::<HashSet<_>>());

        Ok(())
    }

    #[test]
    fn save_and_load() -> Result<(), Box<dyn std::error::Error>> {
        let updates: HashSet<_> = vec![
            (
                uri::RadUrn {
                    id: Hash::hash(b"project0"),
                    proto: uri::Protocol::Git,
                    path: "cloudhead/new-language".parse::<uri::Path>()?,
                },
                "7dec3269".parse::<oid::Oid>()?,
            ),
            (
                uri::RadUrn {
                    id: Hash::hash(b"project0"),
                    proto: uri::Protocol::Git,
                    path: "fintohaps/notations".parse::<uri::Path>()?,
                },
                "b4d3276d".parse::<oid::Oid>()?,
            ),
            (
                uri::RadUrn {
                    id: Hash::hash(b"project0"),
                    proto: uri::Protocol::Git,
                    path: "kalt/loops".parse::<uri::Path>()?,
                },
                "2206e5dc".parse::<oid::Oid>()?,
            ),
            (
                uri::RadUrn {
                    id: Hash::hash(b"project1"),
                    proto: uri::Protocol::Git,
                    path: "backport".parse::<uri::Path>()?,
                },
                "869e5740".parse::<oid::Oid>()?,
            ),
        ]
        .iter()
        .cloned()
        .collect();
        let dir = tempfile::tempdir()?;
        let store = kv::Store::new(kv::Config::new(dir.path().join("store")))?;

        super::save(&store, updates.clone())?;

        assert_eq!(super::load(&store)?, updates);

        Ok(())
    }

    fn project0(head: &str) -> uri::RadUrn {
        uri::RadUrn {
            id: Hash::hash(b"project0"),
            proto: uri::Protocol::Git,
            path: head.parse::<uri::Path>().expect("unable to parse head"),
        }
    }
    fn project1(head: &str) -> uri::RadUrn {
        uri::RadUrn {
            id: Hash::hash(b"project1"),
            proto: uri::Protocol::Git,
            path: head.parse::<uri::Path>().expect("unable to parse head"),
        }
    }
}
