//! Management of local session state like the currently used identity, wallet related data and
//! configuration of all sorts.

use serde::{Deserialize, Serialize};

use coco;
use crate::error;
use crate::identity;

/// Name for the storage bucket used for all session data.
const BUCKET_NAME: &str = "session";
/// Name of the item used for the currently active session.
const KEY_CURRENT: &str = "current";

/// Container for all local state.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// The currently used [`identity::Identity`].
    pub identity: Option<identity::Identity>,
    /// Permissions of the user to control actions.
    pub permissions: Permissions,
    /// User controlled parameters to control the behaviour and state of the application.
    pub settings: settings::Settings,
}

/// Set of permitted actions the user can perform.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    /// Permission to register a user handle
    pub register_handle: bool,
    /// Permission to register an org
    pub register_org: bool,
    /// Permission to register a project
    pub register_project: bool,
}

/// Resets the session state.
///
/// # Errors
///
/// Errors if the state on disk can't be accessed.
pub fn clear_current(store: &kv::Store) -> Result<(), error::Error> {
    Ok(store
        .bucket::<&str, kv::Json<Session>>(Some(BUCKET_NAME))?
        .remove(KEY_CURRENT)?)
}

/// Read the current settings.
///
/// # Errors
///
/// Errors if access to the setttings fails.
pub async fn settings(store: &kv::Store) -> Result<settings::Settings, error::Error> {
    let session = get(store, KEY_CURRENT)?;
    Ok(session.settings)
}

/// Reads the current session.
///
/// # Errors
///
/// Errors if access to the session state fails, or associated data like the [`identity::Identity`]
/// can't be found.
pub async fn current(
    api: &coco::Api,
    store: &kv::Store,
) -> Result<Session, error::Error>
where
{
    let mut session = get(store, KEY_CURRENT)?;

    // Reset the permissions
    session.permissions = Permissions::default();

    if let Some(id) = session.identity.clone() {
        identity::get(api, &id.urn)?;
    }

    Ok(session)
}

/// Stores the [`identity::Identity`] in the current session.
///
/// # Errors
///
/// Errors if access to the session state fails, or associated data like the [`identity::Identity`]
/// can't be found.
pub fn set_identity(store: &kv::Store, id: identity::Identity) -> Result<(), error::Error> {
    let mut sess = get(store, KEY_CURRENT)?;
    sess.identity = Some(id);

    set(store, KEY_CURRENT, sess)
}

/// Stores the [`settings::Settings`] in the current session.
///
/// # Errors
///
/// Errors if access to the session state fails.
pub fn set_settings(store: &kv::Store, settings: settings::Settings) -> Result<(), error::Error> {
    let mut sess = get(store, KEY_CURRENT)?;
    sess.settings = settings;

    set(store, KEY_CURRENT, sess)
}

/// Fetches the session for the given item key.
fn get(store: &kv::Store, key: &str) -> Result<Session, error::Error> {
    Ok(store
        .bucket::<&str, kv::Json<Session>>(Some(BUCKET_NAME))?
        .get(key)?
        .map(kv::Codec::to_inner)
        .unwrap_or_default())
}

/// Stores the session for the given item key.
fn set(store: &kv::Store, key: &str, sess: Session) -> Result<(), error::Error> {
    Ok(store
        .bucket::<&str, kv::Json<Session>>(Some(BUCKET_NAME))?
        .set(key, kv::Json(sess))?)
}

/// User controlled parameters for application appearance, behaviour and state.
pub mod settings {
    use std::collections::HashMap;
    use serde::{Deserialize, Serialize};
    use warp::document::{self, ToDocumentedType};

    /// User controlled parameters for application appearance, behaviour and state.
    #[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Settings {
        /// Currently set appearance parameters.
        pub appearance: Appearance,
        /// Currently set registry parameters.
        pub registry: Registry,
        /// User-determined p2p parameters.
        pub coco: CoCo,
    }

    /// Knobs for the look and feel.
    #[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Appearance {
        /// Currently active color scheme.
        pub theme: Theme,
        /// User dismissable hints.
        pub hints: Hints,
    }

    /// Color schemes available.
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum Theme {
        /// A dark theme.
        Dark,
        /// A light theme.
        Light,
    }

    impl Default for Theme {
        fn default() -> Self {
            Self::Light
        }
    }

    /// User dismissable textual hints.
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Hints {
        /// Whether to show hints about how to set up the remote helper.
        pub show_remote_helper: bool,
    }

    impl Default for Hints {
        fn default() -> Self {
            Self {
                show_remote_helper: true,
            }
        }
    }

    /// Registry parameters.
    #[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Registry {
        /// Currently configured network.
        pub network: Network,
    }

    /// Known networks the application can connect to.
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Network {
        /// In-memory registry, which only lives as long as the app does.
        Emulator,
        /// The friends-n-family network. For the loved ones.
        FFnet,
        /// Test network.
        Testnet,
    }

    impl Default for Network {
        fn default() -> Self {
            Self::Emulator
        }
    }

    /// `CoCo` config parameters subject to user preferences
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    pub struct CoCo {
        /// Peers to connect to at startup.
        pub seeds: Vec<String>,
    }

    impl Default for CoCo {
        fn default() -> Self {
            Self {
                seeds: vec!["seed.radicle.xyz"]
                    .into_iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            }
        }
    }

    /* ToDocumentedType Implementations */
    impl ToDocumentedType for super::Session {
        fn document() -> document::DocumentedType {
            let mut properties = HashMap::with_capacity(1);
            properties.insert(
                "identity".into(),
                crate::identity::Identity::document().nullable(true),
            );
            properties.insert("settings".into(), Settings::document());

            document::DocumentedType::from(properties).description("Session")
        }
    }

    impl ToDocumentedType for Settings {
        fn document() -> document::DocumentedType {
            let mut properties = HashMap::with_capacity(2);
            properties.insert(
                "appearance".into(),
                Appearance::document(),
            );
            properties.insert("registry".into(), Registry::document());

            document::DocumentedType::from(properties).description("Settings")
        }
    }

    impl ToDocumentedType for Appearance {
        fn document() -> document::DocumentedType {
            let mut properties = HashMap::with_capacity(1);
            properties.insert("theme".into(), Theme::document());

            document::DocumentedType::from(properties).description("Appearance")
        }
    }

    impl ToDocumentedType for Theme {
        fn document() -> document::DocumentedType {
            document::enum_string(vec!["dark".into(), "light".into()])
                .description("Variants for possible color schemes.")
                .example("dark")
        }
    }

    impl ToDocumentedType for Registry {
        fn document() -> document::DocumentedType {
            let mut properties = HashMap::with_capacity(1);
            properties.insert("network".into(), Network::document());

            document::DocumentedType::from(properties).description("Registry")
        }
    }

    impl ToDocumentedType for Network {
        fn document() -> document::DocumentedType {
            document::enum_string(vec!["emulator".into(), "ffnet".into(), "testnet".into()])
                .description("Variants for possible networks of the Registry to connect to.")
                .example("testnet")
        }
    }
}
