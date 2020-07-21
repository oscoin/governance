//! Utility to work with the peer api of librad.

use std::convert::TryFrom;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nonempty::NonEmpty;
use serde::Serialize;

use librad::keys;
use librad::meta::entity;
use librad::meta::project;
use librad::meta::user;
use librad::net::discovery;
pub use librad::net::peer::{PeerApi, PeerConfig};
use librad::paths;
use librad::peer::PeerId;
use librad::uri::RadUrn;
use radicle_surf::vcs::git::{self, git2, BranchType};

use super::source;
use crate::error;
use crate::identity;
use crate::project::Project;

/// Export a verified [`user::User`] type.
pub type User = user::User<entity::Verified>;

/// Bundled response to retrieve both branches and tags for a user repo.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRevisions {
    /// Owner of the repo.
    pub(crate) identity: identity::Identity,
    /// List of [`source::Branch`].
    pub(crate) branches: Vec<source::Branch>,
    /// List of [`source::Tag`].
    pub(crate) tags: Vec<source::Tag>,
}

/// High-level interface to the coco monorepo and gossip layer.
pub struct Api {
    /// Thread-safe wrapper around [`PeerApi`].
    peer_api: Arc<Mutex<PeerApi>>,
}

impl Api {
    /// Create a new `PeerApi` given a `PeerConfig`.
    ///
    /// # Errors
    ///
    /// If turning the config into a `Peer` fails
    /// If trying to accept on the socket fails
    pub async fn new<I>(
        config: PeerConfig<discovery::Static<I, SocketAddr>>,
    ) -> Result<Self, error::Error>
    where
        I: Iterator<Item = (PeerId, SocketAddr)> + Send + 'static,
    {
        let peer = config.try_into_peer().await?;
        // TODO(finto): discarding the run loop below. Should be used to subsrcibe to events and
        // publish events.
        let (api, _futures) = peer.accept()?;

        Ok(Self {
            peer_api: Arc::new(Mutex::new(api)),
        })
    }

    /// Returns the [`PathBuf`] to the underlying monorepo.
    #[must_use]
    pub fn monorepo(&self) -> PathBuf {
        let api = self.peer_api.lock().expect("unable to acquire lock");
        api.paths().git_dir().join("")
    }

    /// Returns the underlying [`paths::Paths`].
    #[must_use]
    pub fn paths(&self) -> paths::Paths {
        let api = self.peer_api.lock().expect("unable to acquire lock");
        api.paths().clone()
    }

    /// Convenience method to trigger a reopen of the storage.
    ///
    /// # Errors
    ///
    /// When the underlying lock acquisition fails or opening the storage.
    pub fn reopen(&self) -> Result<(), error::Error> {
        let api = self.peer_api.lock().expect("unable to acquire lock");
        api.storage().reopen()?;

        Ok(())
    }

    /// Our current peers [`PeerId`].
    #[must_use]
    pub fn peer_id(&self) -> PeerId {
        let api = self.peer_api.lock().expect("unable to acquire lock");
        api.peer_id().clone()
    }

    /// Get the default owner for this `PeerApi`.
    #[must_use]
    pub fn default_owner(&self) -> Option<user::User<entity::Draft>> {
        let api = self.peer_api.lock().expect("unable to acquire lock");

        match api.storage().default_rad_self() {
            Ok(user) => Some(user),
            Err(err) => {
                log::warn!("an error occurred while trying to get 'rad/self': {}", err);
                None
            },
        }
    }

    /// Set the default owner for this `PeerApi`.
    ///
    /// # Errors
    ///
    ///   * Fails to set the default `rad/self` for this `PeerApi`.
    pub fn set_default_owner(&self, user: User) -> Result<(), error::Error> {
        let api = self.peer_api.lock().expect("unable to acquire lock");
        Ok(api.storage().set_default_rad_self(user)?)
    }

    /// Initialise a [`User`] and make them the default owner of this `PeerApi`.
    ///
    /// # Errors
    ///
    ///   * Fails to initialise `User`.
    ///   * Fails to verify `User`.
    ///   * Fails to set the default `rad/self` for this `PeerApi`.
    pub fn init_owner(&self, key: keys::SecretKey, handle: &str) -> Result<User, error::Error> {
        let user = self.init_user(key, handle)?;
        let user = verify_user(user)?;

        self.set_default_owner(user.clone())?;

        Ok(user)
    }

    /// Returns the list of [`project::Project`]s for your peer.
    ///
    /// # Errors
    ///
    ///   * The retrieving the project entities from the store fails.
    #[allow(
        clippy::match_wildcard_for_single_variants,
        clippy::wildcard_enum_match_arm
    )]
    pub fn list_projects(&self) -> Result<Vec<Project>, error::Error> {
        let project_meta = {
            let api = self.peer_api.lock().expect("unable to acquire lock");
            let storage = api.storage().reopen()?;
            let owner = storage.default_rad_self()?;

            let meta = storage.all_metadata()?;
            meta.flat_map(|entity| {
                let entity = entity.ok()?;
                let rad_self = storage.get_rad_self(&entity.urn()).ok()?;

                // We only list projects that are owned by the peer
                if rad_self.urn() != owner.urn() {
                    return None;
                }

                entity.try_map(|info| match info {
                    entity::data::EntityInfo::Project(info) => Some(info),
                    _ => None,
                })
            })
            .collect::<Vec<_>>()
        };

        project_meta
            .into_iter()
            .map(|project| {
                self.with_browser(&project.urn(), |browser| {
                    let stats = browser.get_stats()?;
                    Ok((project, stats).into())
                })
            })
            .collect()
    }

    /// Returns the list of [`user::User`]s known for your peer.
    ///
    /// # Errors
    ///
    ///   * Retrieval of the user entities from the store fails.
    #[allow(
        clippy::match_wildcard_for_single_variants,
        clippy::wildcard_enum_match_arm
    )]
    pub fn list_users(&self) -> Result<Vec<user::User<entity::Draft>>, error::Error> {
        let api = self.peer_api.lock().expect("unable to acquire lock");
        let storage = api.storage();

        let mut entities = vec![];
        for entity in storage.all_metadata()? {
            let entity = entity?;

            if let Some(e) = entity.try_map(|info| match info {
                entity::data::EntityInfo::User(info) => Some(info),
                _ => None,
            }) {
                entities.push(e);
            }
        }

        Ok(entities)
    }

    /// Get all [`UserRevisions`] for a given project.
    ///
    /// # Parameters
    ///
    /// * `owner` - the owner of this peer, i.e. the current user
    /// * `urn` - the [`RadUrn`] pointing to the project we're interested in
    ///
    /// # Errors
    ///
    ///   * [`error::Error::LibradLock`]
    ///   * [`error::Error::Git`]
    pub fn revisions(
        &self,
        owner: &User,
        urn: &RadUrn,
    ) -> Result<NonEmpty<UserRevisions>, error::Error> {
        let project = self.get_project(urn)?;
        let mut user_revisions = vec![];

        let (local_branches, local_tags) = self.with_browser(urn, |browser| {
            Ok((
                source::branches(browser, Some(BranchType::Local))?,
                source::tags(browser)?,
            ))
        })?;

        if !local_branches.is_empty() {
            user_revisions.push(UserRevisions {
                identity: (self.peer_id(), owner.clone()).into(),
                branches: local_branches,
                tags: local_tags,
            })
        }

        let tracked_peers = {
            let api = self.peer_api.lock().expect("unable to acquire lock");
            let storage = api.storage().reopen()?;
            let repo = storage.open_repo(urn.clone())?;
            repo.tracked()?
        };

        for peer_id in tracked_peers {
            let remote_branches = self.with_browser(&project.urn(), |browser| {
                source::branches(
                    browser,
                    Some(BranchType::Remote {
                        name: Some(format!("{}/heads", peer_id)),
                    }),
                )
            })?;

            let api = self.peer_api.lock().expect("unable to acquire lock");
            let storage = api.storage().reopen()?;
            let user = storage.get_rad_self_of(urn, peer_id.clone())?;

            user_revisions.push(UserRevisions {
                identity: (peer_id, user).into(),
                branches: remote_branches,
                // TODO(rudolfs): implement remote peer tags once we decide how
                // https://radicle.community/t/git-tags/214
                tags: vec![],
            });
        }

        NonEmpty::from_vec(user_revisions).ok_or(error::Error::EmptyUserRevisions)
    }

    /// Get the project found at `urn`.
    ///
    /// # Errors
    ///
    ///   * Resolving the project fails.
    pub fn get_project(
        &self,
        urn: &RadUrn,
    ) -> Result<project::Project<entity::Draft>, error::Error> {
        let api = self.peer_api.lock().expect("unable to acquire lock");
        let storage = api.storage().reopen()?;

        Ok(storage.metadata(urn)?)
    }

    /// Get the user found at `urn`.
    ///
    /// # Errors
    ///
    ///   * Resolving the user fails.
    ///   * Could not successfully acquire a lock to the API.
    pub fn get_user(&self, urn: &RadUrn) -> Result<user::User<entity::Draft>, error::Error> {
        let api = self.peer_api.lock().expect("unable to acquire lock");
        let storage = api.storage().reopen()?;

        Ok(storage.metadata(urn)?)
    }

    /// Get a repo browser for a project.
    ///
    /// # Errors
    ///
    /// The function will result in an error if the mutex guard was poisoned. See
    /// [`std::sync::Mutex::lock`] for further details.
    pub fn with_browser<F, T>(&self, urn: &RadUrn, callback: F) -> Result<T, error::Error>
    where
        F: Send + FnOnce(&mut git::Browser) -> Result<T, error::Error>,
    {
        let git_dir = self.monorepo();

        let project = self.get_project(urn)?;
        let default_branch = git::Branch::local(project.default_branch());
        let repo = git::Repository::new(git_dir)?;
        let namespace = git::Namespace::try_from(project.urn().id.to_string().as_str())?;
        let mut browser = git::Browser::new_with_namespace(&repo, &namespace, default_branch)?;

        callback(&mut browser)
    }

    /// Initialize a [`project::Project`] that is owned by the `owner`.
    /// This kicks off the history of the project, tracked by `librad`'s mono-repo.
    ///
    /// # Errors
    ///
    /// Will error if:
    ///     * The signing of the project metadata fails.
    ///     * The interaction with `librad` [`librad::git::storage::Storage`] fails.
    #[allow(clippy::needless_pass_by_value)] // We don't want to keep `SecretKey` in memory.
    pub fn init_project(
        &self,
        key: &keys::SecretKey,
        owner: &User,
        path: impl AsRef<std::path::Path> + Send,
        name: &str,
        description: &str,
        default_branch: &str,
    ) -> Result<project::Project<entity::Draft>, error::Error> {
        let api = self.peer_api.lock().expect("unable to acquire lock");

        // Test if the repo has setup rad remote.
        if let Ok(repo) = git2::Repository::open(&path) {
            if repo.find_remote("rad").is_ok() {
                return Err(error::Error::RadRemoteExists(format!(
                    "{}",
                    path.as_ref().display(),
                )));
            }
        }

        let meta: Result<project::Project<entity::Draft>, error::Error> = {
            // Create the project meta
            let mut meta =
                project::Project::<entity::Draft>::create(name.to_string(), owner.urn())?
                    .to_builder()
                    .set_description(description.to_string())
                    .set_default_branch(default_branch.to_string())
                    .add_key(key.public())
                    .add_certifier(owner.urn())
                    .build()?;
            meta.sign_owned(key)?;
            let urn = meta.urn();

            let storage = api.storage().reopen()?;

            if storage.has_urn(&urn)? {
                return Err(error::Error::EntityExists(urn));
            } else {
                let repo = storage.create_repo(&meta)?;
                repo.set_rad_self(librad::git::storage::RadSelfSpec::Urn(owner.urn()))?;
            }
            Ok(meta)
        };

        // Doing ? above breaks inference. Gaaaawwwwwd Rust!
        let meta = meta?;

        setup_remote(&api, path, &meta.urn().id, default_branch)?;

        Ok(meta)
    }

    /// Create a [`user::User`] with the provided `handle`. This assumes that you are creating a
    /// user that uses the secret key the `PeerApi` was configured with.
    ///
    /// # Errors
    ///
    /// Will error if:
    ///     * The signing of the user metadata fails.
    ///     * The interaction with `librad` [`librad::git::storage::Storage`] fails.
    #[allow(clippy::needless_pass_by_value)] // We don't want to keep `SecretKey` in memory.
    pub fn init_user(
        &self,
        key: keys::SecretKey,
        handle: &str,
    ) -> Result<user::User<entity::Draft>, error::Error> {
        // Create the project meta
        let mut user = user::User::<entity::Draft>::create(handle.to_string(), key.public())?;
        user.sign_owned(&key)?;
        let urn = user.urn();

        // Initialising user in the storage.
        {
            let api = self.peer_api.lock().expect("unable to acquire lock");
            let storage = api.storage().reopen()?;

            if storage.has_urn(&urn)? {
                return Err(error::Error::EntityExists(urn));
            } else {
                let _repo = storage.create_repo(&user)?;
            }
        }

        Ok(user)
    }

    /// Wrapper around the storage track.
    ///
    /// # Errors
    ///
    /// * When the storage operation fails.
    pub fn track(&self, urn: &RadUrn, remote: &PeerId) -> Result<(), error::Error> {
        let api = self.peer_api.lock().expect("unable to acquire lock");
        Ok(api.storage().track(urn, remote)?)
    }
}

/// Verify a user using a fake resolver that resolves the user to itself.
///
/// TODO(finto): Should not live here permanently, because resolvers should solve this verification.
///
/// # Errors
///
/// If any of the verification steps fail
pub fn verify_user(user: user::User<entity::Draft>) -> Result<User, error::Error> {
    let fake_resolver = FakeUserResolver(user.clone());
    let verified_user = user.check_history_status(&fake_resolver, &fake_resolver)?;
    Ok(verified_user)
}

/// Equips a repository with a rad remote for the given id. If the directory at the given path
/// is not managed by git yet we initialise it first.
fn setup_remote(
    peer: &PeerApi,
    path: impl AsRef<std::path::Path>,
    id: &librad::hash::Hash,
    default_branch: &str,
) -> Result<(), error::Error> {
    // Check if directory at path is a git repo.
    if git2::Repository::open(&path).is_err() {
        let repo = git2::Repository::init(&path)?;
        // First use the config to initialize a commit signature for the user.
        let sig = repo.signature()?;
        // Now let's create an empty tree for this commit
        let tree_id = {
            let mut index = repo.index()?;

            // For our purposes, we'll leave the index empty for now.
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        // Normally creating a commit would involve looking up the current HEAD
        // commit and making that be the parent of the initial commit, but here this
        // is the first commit so there will be no parent.
        repo.commit(
            Some(&format!("refs/heads/{}", default_branch)),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        )?;
    }

    let repo = git2::Repository::open(path)?;

    if let Err(err) = repo.resolve_reference_from_short_name(default_branch) {
        log::error!("error while trying to find default branch: {:?}", err);
        return Err(error::Error::DefaultBranchMissing(
            id.to_string(),
            default_branch.to_string(),
        ));
    }

    let monorepo = peer.paths().git_dir().join("");
    let namespace_prefix = format!("refs/namespaces/{}/refs", id);
    let mut remote = repo.remote_with_fetch(
        "rad",
        &format!(
            "file://{}",
            monorepo.to_str().expect("unable to get str for monorepo")
        ),
        &format!("+{}/heads/*:refs/heads/*", namespace_prefix),
    )?;
    repo.remote_add_push(
        "rad",
        &format!("+refs/heads/*:{}/heads/*", namespace_prefix),
    )?;
    remote.push(
        &[&format!(
            "refs/heads/{}:{}/heads/{}",
            default_branch, namespace_prefix, default_branch
        )],
        None,
    )?;

    Ok(())
}

/// Acting as a fake resolver where a User resolves to itself.
/// This allows us to check the history status of a single User.
/// TODO(finto): Remove this once Resolvers are complete.
struct FakeUserResolver(user::User<entity::Draft>);

impl entity::Resolver<user::User<entity::Draft>> for FakeUserResolver {
    fn resolve(&self, _uri: &RadUrn) -> Result<user::User<entity::Draft>, entity::Error> {
        Ok(self.0.clone())
    }

    fn resolve_revision(
        &self,
        _uri: &RadUrn,
        _revision: u64,
    ) -> Result<user::User<entity::Draft>, entity::Error> {
        Ok(self.0.clone())
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod test {
    use librad::keys::SecretKey;

    use crate::coco::config;
    use crate::coco::control;
    use crate::error::Error;

    use super::Api;

    #[tokio::test]
    async fn test_can_create_user() -> Result<(), Error> {
        let tmp_dir = tempfile::tempdir().expect("failed to create temdir");
        let key = SecretKey::new();
        let config = config::default(key.clone(), tmp_dir.path())?;
        let api = Api::new(config).await?;

        let annie = api.init_user(key, "annie_are_you_ok?");
        assert!(annie.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_can_create_project() -> Result<(), Error> {
        let tmp_dir = tempfile::tempdir().expect("failed to create temdir");
        let repo_path = tmp_dir.path().join("radicle");
        let key = SecretKey::new();
        let config = config::default(key.clone(), tmp_dir.path())?;
        let api = Api::new(config).await?;

        let user = api.init_owner(key.clone(), "cloudhead")?;
        let project =
            api.init_project(&key, &user, &repo_path, "radicalise", "the people", "power");

        assert!(project.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_cannot_create_user_twice() -> Result<(), Error> {
        let tmp_dir = tempfile::tempdir().expect("failed to create temdir");
        let key = SecretKey::new();
        let config = config::default(key.clone(), tmp_dir.path())?;
        let api = Api::new(config).await?;

        let user = api.init_owner(key.clone(), "cloudhead")?;
        let err = api.init_user(key, "cloudhead");

        if let Err(Error::EntityExists(urn)) = err {
            assert_eq!(urn, user.urn())
        } else {
            panic!(
                "unexpected error when creating the user a second time: {:?}",
                err
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_cannot_create_project_twice() -> Result<(), Error> {
        let tmp_dir = tempfile::tempdir().expect("failed to create temdir");
        let repo_path = tmp_dir.path().join("radicle");
        let key = SecretKey::new();
        let config = config::default(key.clone(), tmp_dir.path())?;
        let api = Api::new(config).await?;

        let user = api.init_owner(key.clone(), "cloudhead")?;
        let _project =
            api.init_project(&key, &user, &repo_path, "radicalise", "the people", "power")?;

        let err = api.init_project(&key, &user, &repo_path, "radicalise", "the people", "power");

        if let Err(Error::RadRemoteExists(path)) = err {
            assert_eq!(path, format!("{}", repo_path.display()))
        } else {
            panic!(
                "unexpected error when creating the project a second time: {:?}",
                err
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn list_projects() -> Result<(), Error> {
        let tmp_dir = tempfile::tempdir().expect("failed to create temdir");
        let repo_path = tmp_dir.path().join("radicle");

        let key = SecretKey::new();
        let config = config::default(key.clone(), tmp_dir.path())?;
        let api = Api::new(config).await?;

        let user = api.init_owner(key.clone(), "cloudhead")?;

        control::setup_fixtures(&api, key.clone(), &user)?;

        let kalt = api.init_user(key.clone(), "kalt")?;
        let kalt = super::verify_user(kalt)?;
        let fakie = api.init_project(
            &key,
            &kalt,
            &repo_path,
            "fakie-nose-kickflip-backside-180-to-handplant",
            "rad git tricks",
            "dope",
        )?;

        let projects = api.list_projects()?;
        let mut project_names = projects
            .into_iter()
            .map(|project| project.metadata.name)
            .collect::<Vec<_>>();
        project_names.sort();

        assert_eq!(
            project_names,
            vec!["Monadic", "monokel", "open source coin", "radicle"]
        );

        assert!(!project_names.contains(&fakie.name().to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_list_users() -> Result<(), Error> {
        let tmp_dir = tempfile::tempdir().expect("failed to create temdir");
        let key = SecretKey::new();
        let config = config::default(key.clone(), tmp_dir.path())?;
        let api = Api::new(config).await?;

        let cloudhead = api.init_user(key.clone(), "cloudhead")?;
        let _cloudhead = super::verify_user(cloudhead)?;
        let kalt = api.init_user(key, "kalt")?;
        let _kalt = super::verify_user(kalt)?;

        let users = api.list_users()?;
        let mut user_handles = users
            .into_iter()
            .map(|user| user.name().to_string())
            .collect::<Vec<_>>();
        user_handles.sort();

        assert_eq!(user_handles, vec!["cloudhead", "kalt"],);

        Ok(())
    }
}
