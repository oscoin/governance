//! Management of local session state like the currently used identity, wallet related data and
//! configuration of all sorts.

use crate::error;
use crate::identity;
use crate::registry;

/// Name for the storage bucket used for all session data.
const BUCKET_NAME: &str = "session";

/// Container for all local state.
#[derive(Debug)]
pub struct Session {
    /// The currently used [`identity::Identity`].
    pub identity: Option<identity::Identity>,
    /// List of the orgs of the user associated with the current identity.
    pub orgs: Vec<registry::Org>,
}

/// Resets the session state.
///
/// # Errors
///
/// Errors if the state on disk can't be accessed.
pub fn clear(store: &kv::Store) -> Result<(), error::Error> {
    Ok(store
        .bucket::<kv::Raw, String>(Some(BUCKET_NAME))?
        .clear()?)
}

/// Reads the current session.
///
/// # Errors
///
/// Errors if access to the session state fails, or associated data like the [`identity::Identity`]
/// can't be found.
pub async fn get<R: registry::Client>(
    registry: R,
    store: &kv::Store,
) -> Result<Session, error::Error> {
    let bucket = store.bucket::<&str, String>(Some(BUCKET_NAME))?;

    let identity = bucket
        .get("identity")?
        .and_then(|id| identity::get(id.as_ref()).expect("unable to retrieve identity"));
    // TODO(xla): Get actual attested handle from identity metadata. Alternatively use the stored
    // keypair of the current session to find the associated user and look it up that way.
    let orgs = registry.list_orgs("".to_string()).await?;

    Ok(Session { identity, orgs })
}

/// Stores the Session in its entirety.
///
/// # Errors
///
/// Errors if access to the session state fails.
pub fn set(store: &kv::Store, sess: Session) -> Result<(), error::Error> {
    let bucket = store.bucket::<&str, String>(Some(BUCKET_NAME))?;

    if let Some(identity) = sess.identity {
        bucket.set("identity", identity.id)?;
    }

    Ok(())
}
