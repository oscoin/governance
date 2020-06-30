//! Storage of secret keys.

use std::convert::Infallible;
use std::fmt;

use librad::keys;
use librad::paths;
pub use radicle_keystore::pinentry::SecUtf8;
use radicle_keystore::{
    crypto::{Pwhash, SecretBoxError},
    file, FileStorage, Keystore, SecStr, SecretKeyExt,
};
use radicle_registry_client::{ed25519, CryptoError, CryptoPair};

/// File path to librad key
const LIBRAD_KEY: &str = "librad.key";
/// File path to registry key
const REGISTRY_KEY: &str = "registry.key";

/// Storage for putting and getting the necessary cryptographic keys.
pub struct Keystorage {
    /// Store for `librad`.
    librad_store: LibradStore,
    /// Store for `registry`.
    registry_store: RegistryStore,
}

impl Keystorage {
    /// Create a new `Keystorage`.
    #[must_use = "must use CocoStore to put/get a key"]
    pub fn new(paths: &paths::Paths, pw: SecUtf8) -> Self {
        let path = paths.keys_dir();
        let librad_path = path.join(LIBRAD_KEY);
        let registry_path = path.join(REGISTRY_KEY);
        Self {
            librad_store: FileStorage::new(&librad_path, Pwhash::new(pw.clone())),
            registry_store: FileStorage::new(&registry_path, Pwhash::new(pw)),
        }
    }

    /// Fetch the [`keys::SecretKey`]
    ///
    /// # Errors
    ///
    /// Fails with [`LibradError`]
    pub fn get_librad_key(&self) -> Result<keys::SecretKey, Error> {
        Ok(self.librad_store.get_key().map(|pair| pair.secret_key)?)
    }

    /// Fetch the [`ed25519::Pair`]
    ///
    /// # Errors
    ///
    /// Fails with [`RegistryError`]
    pub fn get_registry_key(&self) -> Result<ed25519::Pair, Error> {
        Ok(self
            .registry_store
            .get_key()
            .map(|pair| pair.secret_key.0)?)
    }

    /// Attempt to get a [`keys::SecretKey`], otherwise we create one and store it.
    ///
    /// # Errors
    ///
    /// Fails with [`LibradError`]
    pub fn init_librad_key(&mut self) -> Result<keys::SecretKey, Error> {
        match self.librad_store.get_key() {
            Ok(keypair) => Ok(keypair.secret_key),
            Err(file::Error::NoSuchKey) => {
                let key = keys::SecretKey::new();
                self.librad_store.put_key(key.clone())?;
                Ok(key)
            },
            Err(err) => Err(err.into()),
        }
    }

    /// Attempt to get a [`ed25519::Pair`], otherwise we create one and store it.
    ///
    /// # Errors
    ///
    /// Fails with [`RegistryError`]
    pub fn init_registry_key(&mut self) -> Result<ed25519::Pair, Error> {
        match self.registry_store.get_key() {
            Ok(keypair) => Ok(keypair.secret_key.0),
            Err(file::Error::NoSuchKey) => {
                let (key, _): (ed25519::Pair, _) = CryptoPair::generate();
                self.registry_store.put_key(Pair(key.clone()))?;
                Ok(key)
            },
            Err(err) => Err(err.into()),
        }
    }
}

/// Synonym for an error when interacting with a store for [`librad::keys`].
type LibradError = file::Error<SecretBoxError<Infallible>, keys::IntoSecretKeyError>;
/// Synonym for storing keys related to `librad`.
type LibradStore = FileStorage<
    Pwhash<SecUtf8>,
    keys::PublicKey,
    keys::SecretKey,
    <keys::SecretKey as SecretKeyExt>::Metadata,
>;

/// Synonym for an error when interacting with a store for [`radicle_registry_client::ed25519`].
type RegistryError = file::Error<SecretBoxError<Infallible>, PairError>;
/// Synonym for storing keys related to `radicle_registry_client`.
type RegistryStore =
    FileStorage<Pwhash<SecUtf8>, ed25519::Public, Pair, <Pair as SecretKeyExt>::Metadata>;

/// The [`Keystorage`] can result in two kinds of errors depending on what storage you're using.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Errors that occurred when interacting with the `librad.key`.
    #[error(transparent)]
    Librad(#[from] LibradError),
    /// Errors that occurred when interacting with the `registry.key`.
    #[error(transparent)]
    Registry(#[from] RegistryError),
}

/// A newtype wrapper around [`CryptoError`] to allow us to define the necessary
/// [`std::error::Error`] traits.
#[derive(Debug)]
pub struct PairError(CryptoError);

impl From<CryptoError> for PairError {
    fn from(err: CryptoError) -> Self {
        Self(err)
    }
}

impl std::error::Error for PairError {}

impl fmt::Display for PairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            CryptoError::InvalidPath => write!(f, "invalid path"),
            CryptoError::InvalidSeed => write!(f, "invalid seed"),
            CryptoError::InvalidFormat => write!(f, "invalid format"),
            CryptoError::InvalidPhrase => write!(f, "invalid phrase"),
            CryptoError::InvalidPassword => write!(f, "invalid password"),
            CryptoError::InvalidSeedLength => write!(f, "invalid seed length"),
        }
    }
}

/// Wrapper around [`ed25519::Pair`] so that we can define the [`SecretKeyExt`] trait required for
/// [`FileStorage`].
struct Pair(ed25519::Pair);

impl AsRef<[u8]> for Pair {
    fn as_ref(&self) -> &[u8] {
        self.0.seed()
    }
}

impl From<Pair> for ed25519::Public {
    fn from(pair: Pair) -> Self {
        pair.0.into()
    }
}

impl SecretKeyExt for Pair {
    type Metadata = ();
    type Error = PairError;

    fn from_bytes_and_meta(bytes: SecStr, _metadata: &Self::Metadata) -> Result<Self, Self::Error> {
        Ok(Self(CryptoPair::from_seed_slice(bytes.unsecure())?))
    }

    fn metadata(&self) -> Self::Metadata {}
}

#[cfg(test)]
mod tests {
    use super::Keystorage;
    use librad::paths;
    use radicle_keystore::pinentry::SecUtf8;

    #[allow(clippy::panic)]
    #[test]
    fn can_create_librad_key() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let paths = paths::Paths::from_root(temp_dir.path())?;
        let pw = SecUtf8::from("asdf");
        let mut store = Keystorage::new(&paths, pw);

        let key = store.init_librad_key().expect("could not create key:");

        assert!(
            key == store.get_librad_key()?,
            "the stored key was not equal to the one retrieved"
        );

        Ok(())
    }

    #[test]
    fn can_create_registry_key() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let paths = paths::Paths::from_root(temp_dir.path())?;
        let pw = SecUtf8::from("asdf");
        let mut store = Keystorage::new(&paths, pw);

        let _key = store
            .init_registry_key()
            .expect("could not get or create a key:");

        // N.B. We'd like to test that the get does actually get the right key back, buuut we can't
        // compare them OH WELL.

        Ok(())
    }
}
