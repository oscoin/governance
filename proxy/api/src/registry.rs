//! Integrations with the radicle Registry.

#![allow(clippy::empty_line_after_outer_attr)]

use async_trait::async_trait;
use hex::ToHex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_cbor::from_reader;
use std::str::FromStr;

use radicle_registry_client::{self as protocol, ClientT, CryptoPair};
pub use radicle_registry_client::{
    parse_ss58_address, AccountId, Balance, BlockHash, Id, IdStatus, ProjectDomain, ProjectName,
    MINIMUM_TX_FEE, REGISTRATION_FEE,
};

use crate::avatar;
use crate::coco;
use crate::error;

mod transaction;
pub use transaction::{Cache, Cacher, Message, State, Timestamp, Transaction, MIN_CONFIRMATIONS};

/// The type of domain under which a project is registered.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DomainType {
    /// An org
    Org,
    /// A user
    User,
}

/// Wrapper for [`protocol::Hash`] to add serialization.
#[derive(Clone, Debug, PartialEq)]
pub struct Hash(pub protocol::Hash);

// TODO(xla): This should go into the radicle-registry.
impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;

        let hash = protocol::TxHash::from_str(s).map_err(|err| {
            serde::de::Error::custom(err)
            // serde::de::Error::invalid_value(serde::de::Unexpected::Str(s), &"a TxHash")
        })?;

        Ok(Self(hash))
    }
}

// TODO(xla): This should go into the radicle-registry.
impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.encode_hex::<String>())
    }
}

/// `ProjectID` wrapper for serde de/serialization
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    /// Librad project ID.
    pub id: coco::Urn,
    /// Metadata version.
    pub version: u8,
}

/// Configured thresholds for acceptance criteria of transaction progress.
pub struct Thresholds {
    /// Number of blocks after which a [`Transaction`] is assumed to be confirmed.
    pub confirmation: u64,
    /// Number of blocks after which a [`Transaction`] is assumed to be settled.
    pub settlement: u64,
}

/// The registered org with identifier and avatar
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Org {
    /// The unique identifier of the org
    pub id: Id,
    /// The public key of the org
    pub account_id: protocol::ed25519::Public,
    /// Unambiguous identifier pointing at this identity.
    pub shareable_entity_identifier: String,
    /// Generated fallback avatar
    pub avatar_fallback: avatar::Avatar,
    /// List of members of the org
    pub members: Vec<User>,
}

/// A project registered under an [`Org`] or [`User`] on the Registry.
pub struct Project {
    /// Name of the project, unique under the top-level entity.
    pub name: ProjectName,
    /// The domain of the project.
    pub domain: ProjectDomain,
    /// Optionally associated project id for attestation in other systems.
    pub maybe_project_id: Option<coco::Urn>,
}

/// The registered user with associated coco id.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// Unique handle regsistered on the Regisry.
    pub handle: Id,
    /// Associated entity id for attestion.
    pub maybe_entity_id: Option<String>,
    /// The public key of the user
    pub account_id: protocol::ed25519::Public,
}

/// Methods to interact with the Registry in a uniform way.
#[async_trait]
pub trait Client: Clone + Send + Sync {
    /// Check whether a given account exists on chain.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn account_exists(
        &self,
        account_id: &protocol::ed25519::Public,
    ) -> Result<bool, error::Error>;

    /// Get the free balance of a given account on chain.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn free_balance(
        &self,
        account_id: &protocol::ed25519::Public,
    ) -> Result<Balance, error::Error>;

    /// Fetch the current best height by virtue of checking the block header of the best chain.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn best_height(&self) -> Result<u32, error::Error>;

    /// Fetch the block header for the block with the given block hash.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a block with the given has can not be found in the Registry.
    async fn get_block_header(
        &self,
        block: BlockHash,
    ) -> Result<protocol::BlockHeader, error::Error>;

    /// Fetch the status of a given id.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn get_id_status(&self, id: &Id) -> Result<IdStatus, error::Error>;

    /// Try to retrieve org from the Registry by id.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn get_org(&self, id: Id) -> Result<Option<Org>, error::Error>;

    /// List orgs of the Registry.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn list_orgs(&self, handle: Id) -> Result<Vec<Org>, error::Error>;

    /// Create a new unique Org on the Registry.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn register_org(
        &self,
        author: &protocol::ed25519::Pair,
        org_id: Id,
        fee: Balance,
    ) -> Result<Transaction, error::Error>;

    /// Remove a registered Org from the Registry.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn unregister_org(
        &self,
        author: &protocol::ed25519::Pair,
        org_id: Id,
        fee: Balance,
    ) -> Result<Transaction, error::Error>;

    /// Register a User as a member of an Org on the Registry.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn register_member(
        &self,
        author: &protocol::ed25519::Pair,
        org_id: Id,
        user_id: Id,
        fee: Balance,
    ) -> Result<Transaction, error::Error>;

    /// Try to retrieve project from the Registry by name for an id.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn get_project(
        &self,
        project_domain: ProjectDomain,
        project_name: ProjectName,
    ) -> Result<Option<Project>, error::Error>;

    /// List all projects of the Registry for an org.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn list_org_projects(&self, id: Id) -> Result<Vec<Project>, error::Error>;

    /// List projects of the Registry.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn list_projects(&self) -> Result<Vec<protocol::ProjectId>, error::Error>;

    /// Register a new project on the chain.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn register_project(
        &self,
        author: &protocol::ed25519::Pair,
        project_domain: ProjectDomain,
        project_name: ProjectName,
        maybe_project_id: Option<coco::Urn>,
        fee: Balance,
    ) -> Result<Transaction, error::Error>;

    /// Try to retrieve user from the Registry by handle.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn get_user(&self, handle: Id) -> Result<Option<User>, error::Error>;

    /// Create a new unique user on the Registry.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn register_user(
        &self,
        author: &protocol::ed25519::Pair,
        handle: Id,
        id: Option<String>,
        fee: Balance,
    ) -> Result<Transaction, error::Error>;

    /// Remove a registered User from the Registry.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn unregister_user(
        &self,
        author: &protocol::ed25519::Pair,
        handle: Id,
        fee: Balance,
    ) -> Result<Transaction, error::Error>;

    /// Transfer tokens from the user to the recipient.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn transfer_from_user(
        &self,
        author: &protocol::ed25519::Pair,
        recipient: protocol::ed25519::Public,
        amount: Balance,
        fee: Balance,
    ) -> Result<Transaction, error::Error>;

    /// Transfer tokens from an org to the recipient.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn transfer_from_org(
        &self,
        author: &protocol::ed25519::Pair,
        org_id: Id,
        recipient: protocol::ed25519::Public,
        amount: Balance,
        fee: Balance,
    ) -> Result<Transaction, error::Error>;

    /// Graciously pay some tokens to the recipient out of Alices pocket.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a protocol error occurs.
    async fn prepay_account(
        &self,
        recipient: AccountId,
        amount: Balance,
    ) -> Result<(), error::Error>;

    /// Replaces the underlying client. Useful to reset the state of an emulator client, or connect
    /// to a different nework.
    fn reset(&mut self, client: protocol::Client);
}

/// Registry client wrapper.
#[derive(Clone)]
pub struct Registry {
    /// Registry client, whether an emulator or otherwise.
    client: protocol::Client,
}

/// A fake credit balance which we use in integration tests.
const PREPAID_AMOUNT_MICRO_RAD: Balance = 321 * 1_000_000;

/// Registry client wrapper methods
impl Registry {
    /// Wraps a registry client.
    #[must_use]
    pub const fn new(client: protocol::Client) -> Self {
        Self { client }
    }

    /// Returns the configured thresholds for [`Transaction`] acceptance stages.
    #[must_use]
    pub const fn thresholds() -> Thresholds {
        Thresholds {
            confirmation: 3,
            settlement: 9,
        }
    }

    /// Create a new signed [`protocol::Transaction`].
    ///
    /// Fetches the author account nonce and runtime version from the chain.
    async fn new_signed_transaction<M: protocol::Message>(
        &self,
        author: &protocol::ed25519::Pair,
        message: M,
        fee: Balance,
    ) -> Result<protocol::Transaction<M>, error::Error> {
        let nonce = self.client.account_nonce(&author.public()).await?;
        let runtime_transaction_version = self.client.runtime_version().await?.transaction_version;
        let extra = protocol::TransactionExtra {
            nonce,
            genesis_hash: self.client.genesis_hash(),
            runtime_transaction_version,
            fee,
        };
        Ok(protocol::Transaction::new_signed(author, message, extra))
    }

    /// Sign a message, submit it to the chain and wait for it to be confirmed. Return the
    /// transaction hash and the number of the block it was included in.
    ///
    /// # Errors
    ///
    /// Fails with [`error::Error::Runtime`] if applying the transction errors.
    async fn submit_transaction(
        &self,
        author: &protocol::ed25519::Pair,
        message: impl protocol::Message,
        fee: Balance,
    ) -> Result<(Hash, protocol::BlockNumber), error::Error> {
        let tx = self.new_signed_transaction(author, message, fee).await?;
        let applied = self.client.submit_transaction(tx).await?.await?;
        applied.result?;
        let block_hash = applied.block;
        let block = self.get_block_header(block_hash).await?;
        Ok((Hash(applied.tx_hash), block.number))
    }
}

#[async_trait]
impl Client for Registry {
    async fn account_exists(
        &self,
        account_id: &protocol::ed25519::Public,
    ) -> Result<bool, error::Error> {
        self.client
            .account_exists(account_id)
            .await
            .map_err(|e| e.into())
    }

    async fn free_balance(
        &self,
        account_id: &protocol::ed25519::Public,
    ) -> Result<Balance, error::Error> {
        let exists = self.account_exists(account_id).await?;
        if exists {
            self.client
                .free_balance(account_id)
                .await
                .map_err(|e| e.into())
        } else {
            Err(error::Error::AccountNotFound(*account_id))
        }
    }

    async fn best_height(&self) -> Result<u32, error::Error> {
        let header = self.client.block_header_best_chain().await?;

        Ok(header.number)
    }

    async fn get_org(&self, org_id: Id) -> Result<Option<Org>, error::Error> {
        if let Some(org) = self.client.get_org(org_id.clone()).await? {
            let mut members = Vec::new();
            for member in org.members().clone() {
                members.push(
                    self.get_user(member)
                        .await?
                        .expect("Couldn't retrieve org member"),
                );
            }
            Ok(Some(Org {
                id: org_id.clone(),
                account_id: org.account_id(),
                shareable_entity_identifier: format!("%{}", org_id.clone()),
                avatar_fallback: avatar::Avatar::from(&org_id.to_string(), avatar::Usage::Org),
                members,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_block_header(
        &self,
        block: BlockHash,
    ) -> Result<protocol::BlockHeader, error::Error> {
        self.client
            .block_header(block)
            .await?
            .ok_or(error::Error::BlockNotFound(block))
    }

    async fn get_id_status(&self, id: &Id) -> Result<IdStatus, error::Error> {
        let status = self.client.get_id_status(id).await?;
        Ok(status)
    }

    async fn list_orgs(&self, handle: Id) -> Result<Vec<Org>, error::Error> {
        let mut orgs = Vec::new();
        for id in &self.client.list_orgs().await? {
            let org = self.get_org(id.clone()).await?.expect("org missing for id");
            if org.members.iter().any(|m| m.handle == handle) {
                orgs.push(org);
            }
        }

        Ok(orgs)
    }

    async fn register_org(
        &self,
        author: &protocol::ed25519::Pair,
        org_id: Id,
        fee: Balance,
    ) -> Result<Transaction, error::Error> {
        // Prepare and submit org registration transaction.
        let register_message = protocol::message::RegisterOrg {
            org_id: org_id.clone(),
        };
        let (tx_hash, block_number) = self
            .submit_transaction(author, register_message, fee)
            .await?;
        let tx = Transaction::confirmed(
            tx_hash,
            block_number,
            Message::OrgRegistration { id: org_id.clone() },
            fee,
            Some(REGISTRATION_FEE),
        );

        // TODO(xla): Remove automatic prepayment once we have proper balances.
        let org = self.client.get_org(org_id).await?.expect("org not present");
        self.prepay_account(org.account_id(), PREPAID_AMOUNT_MICRO_RAD)
            .await?;

        Ok(tx)
    }

    async fn unregister_org(
        &self,
        author: &protocol::ed25519::Pair,
        org_id: Id,
        fee: Balance,
    ) -> Result<Transaction, error::Error> {
        // Prepare and submit org unregistration transaction.
        let unregister_message = protocol::message::UnregisterOrg {
            org_id: org_id.clone(),
        };
        let (tx_hash, block_number) = self
            .submit_transaction(author, unregister_message, fee)
            .await?;
        Ok(Transaction::confirmed(
            tx_hash,
            block_number,
            Message::OrgUnregistration { id: org_id },
            fee,
            None,
        ))
    }

    async fn register_member(
        &self,
        author: &protocol::ed25519::Pair,
        org_id: Id,
        user_id: Id,
        fee: Balance,
    ) -> Result<Transaction, error::Error> {
        // Prepare and submit member registration transaction.
        let register_message = protocol::message::RegisterMember {
            org_id: org_id.clone(),
            user_id: user_id.clone(),
        };
        let (tx_hash, block_number) = self
            .submit_transaction(author, register_message, fee)
            .await?;
        Ok(Transaction::confirmed(
            tx_hash,
            block_number,
            Message::MemberRegistration {
                org_id: org_id.clone(),
                handle: user_id,
            },
            fee,
            None,
        ))
    }

    async fn get_project(
        &self,
        project_domain: ProjectDomain,
        project_name: ProjectName,
    ) -> Result<Option<Project>, error::Error> {
        Ok(self
            .client
            .get_project(project_name.clone(), project_domain.clone())
            .await?
            .map(|project| {
                let metadata_vec: Vec<u8> = project.metadata().clone().into();
                Project {
                    name: project_name.clone(),
                    domain: project_domain,
                    maybe_project_id: if metadata_vec[..].is_empty() {
                        None
                    } else {
                        let maybe_metadata: Result<Metadata, serde_cbor::error::Error> =
                            from_reader(&metadata_vec[..]);
                        Some(maybe_metadata.expect("Could not read Metadata").id)
                    },
                }
            }))
    }

    async fn list_org_projects(&self, org_id: Id) -> Result<Vec<Project>, error::Error> {
        let ids = self.client.list_projects().await?;
        let mut projects = Vec::new();
        for (name, domain) in &ids {
            if domain.clone() == protocol::ProjectDomain::Org(org_id.clone()) {
                projects.push(
                    self.get_project(domain.clone(), name.clone())
                        .await?
                        .expect("project not present"),
                );
            }
        }
        Ok(projects)
    }

    async fn list_projects(&self) -> Result<Vec<protocol::ProjectId>, error::Error> {
        self.client.list_projects().await.map_err(|e| e.into())
    }

    async fn register_project(
        &self,
        author: &protocol::ed25519::Pair,
        project_domain: ProjectDomain,
        project_name: ProjectName,
        maybe_project_id: Option<coco::Urn>,
        fee: Balance,
    ) -> Result<Transaction, error::Error> {
        // Prepare and submit checkpoint transaction.
        let checkpoint_message = protocol::message::CreateCheckpoint {
            project_hash: protocol::H256::random(),
            previous_checkpoint_id: None,
        };
        let checkpoint_tx = self
            .new_signed_transaction(author, checkpoint_message, fee)
            .await?;
        let checkpoint_id = self
            .client
            .submit_transaction(checkpoint_tx)
            .await?
            .await?
            .result?;

        let register_metadata_vec = if let Some(pid_string) = maybe_project_id {
            let pid_cbor = Metadata {
                id: pid_string,
                version: 1,
            };
            // TODO(garbados): unpanic
            serde_cbor::to_vec(&pid_cbor).expect("unable to serialize project metadata")
        } else {
            vec![]
        };

        // TODO: remove .expect() call, see: https://github.com/radicle-dev/radicle-registry/issues/185
        let register_metadata =
            protocol::Bytes128::from_vec(register_metadata_vec).expect("unable construct metadata");

        // Prepare and submit project registration transaction.
        let register_message = protocol::message::RegisterProject {
            project_name: project_name.clone(),
            project_domain: project_domain.clone(),
            checkpoint_id,
            metadata: register_metadata,
        };
        let (tx_hash, block_number) = self
            .submit_transaction(author, register_message, fee)
            .await?;

        let (domain_type, domain_id) = match project_domain {
            ProjectDomain::Org(id) => (DomainType::Org, id),
            ProjectDomain::User(id) => (DomainType::User, id),
        };

        Ok(Transaction::confirmed(
            tx_hash,
            block_number,
            Message::ProjectRegistration {
                project_name,
                domain_type,
                domain_id,
            },
            fee,
            None,
        ))
    }

    async fn get_user(&self, handle: Id) -> Result<Option<User>, error::Error> {
        Ok(self
            .client
            .get_user(handle.clone())
            .await?
            .map(|user| User {
                handle,
                maybe_entity_id: None,
                account_id: user.account_id(),
            }))
    }

    async fn register_user(
        &self,
        author: &protocol::ed25519::Pair,
        handle: Id,
        id: Option<String>,
        fee: Balance,
    ) -> Result<Transaction, error::Error> {
        // TODO(xla): Remove automatic prepayment once we have proper balances.
        self.prepay_account(author.public(), PREPAID_AMOUNT_MICRO_RAD)
            .await?;
        // Prepare and submit user registration transaction.
        let register_message = protocol::message::RegisterUser {
            user_id: handle.clone(),
        };
        let (tx_hash, block_number) = self
            .submit_transaction(author, register_message, fee)
            .await?;
        Ok(Transaction::confirmed(
            tx_hash,
            block_number,
            Message::UserRegistration { handle, id },
            fee,
            Some(REGISTRATION_FEE),
        ))
    }

    async fn unregister_user(
        &self,
        author: &protocol::ed25519::Pair,
        handle: Id,
        fee: Balance,
    ) -> Result<Transaction, error::Error> {
        // Prepare and submit user unregistration transaction.
        let unregister_message = protocol::message::UnregisterUser {
            user_id: handle.clone(),
        };
        let (tx_hash, block_number) = self
            .submit_transaction(author, unregister_message, fee)
            .await?;

        Ok(Transaction::confirmed(
            tx_hash,
            block_number,
            Message::UserUnregistration { id: handle },
            fee,
            None,
        ))
    }

    async fn transfer_from_user(
        &self,
        author: &protocol::ed25519::Pair,
        recipient: protocol::ed25519::Public,
        amount: Balance,
        fee: Balance,
    ) -> Result<Transaction, error::Error> {
        // Prepare and submit transfer transaction.
        let transfer_message = protocol::message::Transfer { recipient, amount };
        let (tx_hash, block_number) = self
            .submit_transaction(author, transfer_message, fee)
            .await?;

        Ok(Transaction::confirmed(
            tx_hash,
            block_number,
            Message::Transfer { recipient, amount },
            fee,
            None,
        ))
    }

    async fn transfer_from_org(
        &self,
        author: &protocol::ed25519::Pair,
        org_id: Id,
        recipient: protocol::ed25519::Public,
        amount: Balance,
        fee: Balance,
    ) -> Result<Transaction, error::Error> {
        // Prepare and submit transfer transaction.
        let transfer_message = protocol::message::TransferFromOrg {
            org_id: org_id.clone(),
            recipient,
            amount,
        };
        let (tx_hash, block_number) = self
            .submit_transaction(author, transfer_message, fee)
            .await?;

        Ok(Transaction::confirmed(
            tx_hash,
            block_number,
            Message::TransferFromOrg {
                org_id,
                recipient,
                amount,
            },
            fee,
            None,
        ))
    }

    async fn prepay_account(
        &self,
        recipient: AccountId,
        amount: Balance,
    ) -> Result<(), error::Error> {
        let alice = protocol::ed25519::Pair::from_legacy_string("//Alice", None);

        // We don't want this transfer to happen from alice to herself.
        if recipient == alice.public() {
            return Ok(());
        }

        self.client
            .sign_and_submit_message(&alice, protocol::message::Transfer { recipient, amount }, 1)
            .await?
            .await?
            .result?;

        Ok(())
    }

    fn reset(&mut self, client: protocol::Client) {
        self.client = client;
    }
}

#[allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use radicle_registry_client::{self as protocol, ClientT, CryptoPair};
    use serde_cbor::from_reader;
    use std::convert::TryFrom as _;

    use crate::avatar;
    use crate::coco;
    use crate::error;

    use super::{Client, Id, Metadata, ProjectDomain, ProjectName, Registry};

    #[tokio::test]
    async fn test_account_exists() -> Result<(), error::Error> {
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());

        let existing_account =
            protocol::ed25519::Pair::from_legacy_string("//Alice", None).public();
        assert!(
            registry.account_exists(&existing_account).await.unwrap(),
            "Account should exist on chain"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_account_does_not_exist() -> Result<(), error::Error> {
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());

        let random_account = protocol::ed25519::Pair::generate().0.public();
        assert!(
            !registry.account_exists(&random_account).await.unwrap(),
            "Account should not be on chain"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_register_org() -> Result<(), error::Error> {
        // Test that org registration submits valid transactions and they succeed.
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = Id::try_from("alice")?;
        let org_id = Id::try_from("monadic")?;

        // Register the user
        let user_registration = registry
            .register_user(&author, handle.clone(), Some("123abcd.git".into()), 100)
            .await;
        assert!(user_registration.is_ok());

        // Register the org
        let initial_balance = registry.free_balance(&author.public()).await?;
        let fee = 2;
        let result = registry.register_org(&author, org_id.clone(), fee).await;
        assert!(result.is_ok());

        let maybe_org = client.get_org(org_id).await?;
        assert!(maybe_org.is_some());
        let org = maybe_org.unwrap();
        assert_eq!(org.members()[0], handle);

        // The amount prepaid on org registration.
        // TODO(nuno): delete once we no longer prepay accounts
        let prepaid_transfer_costs = super::PREPAID_AMOUNT_MICRO_RAD + 1;
        assert_eq!(
            registry.free_balance(&author.public()).await?,
            initial_balance - prepaid_transfer_costs - fee - protocol::REGISTRATION_FEE
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_unregister_user() -> Result<(), error::Error> {
        // Test that org unregistration submits valid transactions and they succeed.
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = Id::try_from("alice")?;

        // Register the user
        let fee = 2;
        let user_registration = registry
            .register_user(&author, handle.clone(), Some("123abcd.git".into()), fee)
            .await;
        assert!(user_registration.is_ok());

        // Unregister the user
        let initial_balance = registry.free_balance(&author.public()).await?;
        let fee = 2;
        let unregistration = registry.unregister_user(&author, handle, fee).await;
        assert!(unregistration.is_ok());

        assert_eq!(
            registry.free_balance(&author.public()).await?,
            initial_balance - fee
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_unregister_org() -> Result<(), error::Error> {
        // Test that org unregistration submits valid transactions and they succeed.
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = Id::try_from("alice")?;
        let org_id = Id::try_from("monadic")?;

        // Register the user
        let user_registration = registry
            .register_user(&author, handle, Some("123abcd.git".into()), 100)
            .await;
        assert!(user_registration.is_ok());

        // Register the org
        let registration = registry.register_org(&author, org_id.clone(), 10).await;
        assert!(registration.is_ok());

        // Unregister the org
        let initial_balance = registry.free_balance(&author.public()).await?;
        let fee = 2;
        let unregistration = registry.unregister_org(&author, org_id, fee).await;
        assert!(unregistration.is_ok());

        assert_eq!(
            registry.free_balance(&author.public()).await?,
            initial_balance - fee
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_register_member() -> Result<(), error::Error> {
        // Test that member registration submits valid transactions and they succeed.
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = Id::try_from("alice")?;
        let org_id = Id::try_from("monadic")?;

        // Register the user
        let user_registration = registry
            .register_user(&author, handle, Some("123abcd.git".into()), 100)
            .await;
        assert!(user_registration.is_ok());

        let result = registry.register_org(&author, org_id.clone(), 10).await;
        assert!(result.is_ok());

        // Register the second user
        let author2 = protocol::ed25519::Pair::from_legacy_string("//Bob", None);
        let handle2 = Id::try_from("bob")?;
        let user_registration2 = registry
            .register_user(&author2, handle2.clone(), Some("456efgh.git".into()), 100)
            .await;
        assert!(user_registration2.is_ok());

        // Register the second user as a member
        let org = client.get_org(org_id.clone()).await?.unwrap();
        let initial_balance = registry.free_balance(&org.account_id()).await?;
        let fee = 2;
        let member_registration = registry
            .register_member(&author, org_id.clone(), handle2, fee)
            .await;
        assert!(member_registration.is_ok());

        let org = client.get_org(org_id).await?.unwrap();
        assert_eq!(org.members().len(), 2);
        assert!(org.members().contains(&protocol::Id::try_from("alice")?));
        assert!(org.members().contains(&protocol::Id::try_from("bob")?));

        assert_eq!(
            registry.free_balance(&org.account_id()).await?,
            initial_balance - fee
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_get_org() -> Result<(), error::Error> {
        // Test that a registered org can be retrieved.
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = Id::try_from("alice")?;
        let org_id = Id::try_from("monadic")?;

        // Register the user
        let user_registration = registry
            .register_user(&author, handle, Some("123abcd.git".into()), 100)
            .await;
        assert!(user_registration.is_ok());

        // Register the org
        let registration = registry.register_org(&author, org_id.clone(), 10).await;
        assert!(registration.is_ok());

        // Query the org
        let org = registry.get_org(org_id.clone()).await?.unwrap();
        assert_eq!(org.id, org_id);
        assert_eq!(
            org.avatar_fallback,
            avatar::Avatar::from("monadic", avatar::Usage::Org)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_org() -> Result<(), error::Error> {
        // Test that a registered org can be retrieved.
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = Id::try_from("alice")?;
        let org_id = Id::try_from("monadic")?;

        // Register the user
        let user_registration = registry
            .register_user(&author, handle.clone(), Some("123abcd.git".into()), 100)
            .await;
        assert!(user_registration.is_ok());

        // Register the org
        let org_registration = registry.register_org(&author, org_id.clone(), 10).await;
        assert!(org_registration.is_ok());

        // List the orgs
        let orgs = registry.list_orgs(handle).await?;
        assert_eq!(orgs.len(), 1);
        assert_eq!(orgs[0].id, org_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_org_projects() -> Result<(), error::Error> {
        // Test that a registered project is included in the list of org projects.
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = Id::try_from("alice")?;
        let org_id = Id::try_from("monadic")?;
        let project_name = ProjectName::try_from("upstream")?;
        let urn = coco::Urn::new(
            librad::hash::Hash::hash(b"cloudhead"),
            librad::uri::Protocol::Git,
            librad::uri::Path::new(),
        );

        // Register the user
        let user_registration = registry
            .register_user(&author, handle, Some("123abcd.git".into()), 100)
            .await;
        assert!(user_registration.is_ok());

        // Register the org
        let org_registration = registry.register_org(&author, org_id.clone(), 10).await;
        assert!(org_registration.is_ok());

        // Register the project
        let result = registry
            .register_project(
                &author,
                ProjectDomain::Org(org_id.clone()),
                project_name.clone(),
                Some(urn),
                10,
            )
            .await;
        assert!(result.is_ok());

        // List the projects
        let projects = registry.list_org_projects(org_id).await?;
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, project_name);
        assert_eq!(
            projects[0].maybe_project_id,
            Some("rad:git:hwd1yrerjqujexs8p9barieeoo3q6nwczgdn48g9zf8msw5bn9dnsy5eqph".parse()?)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_register_project_under_org() -> Result<(), error::Error> {
        // Test that project registration submits valid transactions and they succeed.
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = Id::try_from("alice")?;
        let org_id = Id::try_from("monadic")?;
        let project_name = ProjectName::try_from("radicle")?;
        let urn = coco::Urn::new(
            librad::hash::Hash::hash(b"cloudhead"),
            librad::uri::Protocol::Git,
            librad::uri::Path::new(),
        );

        // Register the user
        let user_registration = registry
            .register_user(&author, handle, Some("123abcd.git".into()), 100)
            .await;
        assert!(user_registration.is_ok());

        // Register the org
        let org_result = registry.register_org(&author, org_id.clone(), 10).await;
        assert!(org_result.is_ok());

        // Register the project
        let org = registry
            .get_org(org_id.clone())
            .await
            .unwrap()
            .expect("org should exist");
        let initial_balance = registry.free_balance(&org.account_id).await?;
        let fee = 2;
        let result = registry
            .register_project(
                &author,
                ProjectDomain::Org(org_id.clone()),
                project_name.clone(),
                Some(urn),
                fee,
            )
            .await;
        assert!(result.is_ok());

        let maybe_project = client
            .get_project(
                project_name.clone(),
                protocol::ProjectDomain::Org(org_id.clone()),
            )
            .await?;

        assert!(maybe_project.is_some());

        let project = maybe_project.unwrap();
        let metadata_vec: Vec<u8> = project.metadata().clone().into();
        let metadata: Metadata = from_reader(&metadata_vec[..]).unwrap();
        assert_eq!(metadata.version, 1);

        assert_eq!(
            registry.free_balance(&org.account_id).await?,
            initial_balance - fee
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_register_project_under_user() -> Result<(), error::Error> {
        // Test that project registration submits valid transactions and they succeed.
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client.clone());
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = Id::try_from("alice")?;
        let project_name = ProjectName::try_from("radicle")?;
        let urn = coco::Urn::new(
            librad::hash::Hash::hash(b"upstream"),
            librad::uri::Protocol::Git,
            librad::uri::Path::new(),
        );

        // Register the user
        let user_registration = registry
            .register_user(&author, handle.clone(), Some("123abcd.git".into()), 100)
            .await;
        assert!(user_registration.is_ok());

        // Register the project
        let initial_balance = registry.free_balance(&author.public()).await?;
        let fee = 2;
        let result = registry
            .register_project(
                &author,
                ProjectDomain::User(handle.clone()),
                project_name.clone(),
                Some(urn),
                fee,
            )
            .await;
        assert!(result.is_ok());

        let maybe_project = client
            .get_project(
                project_name.clone(),
                protocol::ProjectDomain::User(handle.clone()),
            )
            .await?;

        assert!(maybe_project.is_some());

        let project = maybe_project.unwrap();
        let metadata_vec: Vec<u8> = project.metadata().clone().into();
        let metadata: Metadata = from_reader(&metadata_vec[..]).unwrap();
        assert_eq!(metadata.version, 1);

        assert_eq!(
            registry.free_balance(&author.public()).await?,
            // one fee for project checkpoint setting and one for registration
            initial_balance - 2 * fee
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_register_user() -> Result<(), error::Error> {
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client);
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = Id::try_from("cloudhead")?;

        let initial_balance = registry.free_balance(&author.public()).await?;
        let fee = 2;
        let res = registry
            .register_user(&author, handle, Some("123abcd.git".into()), fee)
            .await;
        assert!(res.is_ok());

        assert_eq!(
            registry.free_balance(&author.public()).await?,
            initial_balance - fee - protocol::REGISTRATION_FEE
        );

        Ok(())
    }

    #[tokio::test]
    async fn transfer_from_user() -> Result<(), error::Error> {
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client);
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);

        // Register the user
        let handle = Id::try_from("alice")?;
        let user_registration = registry
            .register_user(&author, handle, Some("123abcd.git".into()), 100)
            .await;
        assert!(user_registration.is_ok());

        // Register the org
        let org_id = Id::try_from("monadic")?;
        let org_result = registry.register_org(&author, org_id.clone(), 10).await;
        assert!(org_result.is_ok());
        let org = registry
            .client
            .get_org(org_id.clone())
            .await?
            .expect("org not present");

        // Transfer from user to org
        let initial_balance = registry.free_balance(&author.public()).await?;
        let fee = 2;
        let amount = 100;
        let res = registry
            .transfer_from_user(&author, org.account_id(), amount, fee)
            .await;
        assert!(res.is_ok());

        assert_eq!(
            registry.free_balance(&author.public()).await?,
            initial_balance - fee - amount
        );

        Ok(())
    }

    #[tokio::test]
    async fn transfer_from_org() -> Result<(), error::Error> {
        let (client, _) = protocol::Client::new_emulator();
        let registry = Registry::new(client);
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);

        // Register the user
        let handle = Id::try_from("alice")?;
        let user_registration = registry
            .register_user(&author, handle, Some("123abcd.git".into()), 100)
            .await;
        assert!(user_registration.is_ok());

        // Register the org
        let org_id = Id::try_from("monadic")?;
        let org_result = registry.register_org(&author, org_id.clone(), 10).await;
        assert!(org_result.is_ok());
        let org = registry
            .client
            .get_org(org_id.clone())
            .await
            .unwrap()
            .expect("org not present");

        // Transfer from org to user
        let initial_balance = registry.free_balance(&org.account_id()).await?;
        let fee = 2;
        let amount = 100;
        let res = registry
            .transfer_from_org(&author, org_id, author.public(), amount, fee)
            .await;
        assert!(res.is_ok());

        assert_eq!(
            registry.free_balance(&org.account_id()).await?,
            initial_balance - fee - amount
        );

        Ok(())
    }
}
