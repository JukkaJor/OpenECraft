//! Blocking client for Autodesk Vault (`openecraft_client::vault`). It uses the
//! transport of the parts server client, so the tools can be tested against an
//! in-memory vault.

use crate::parts_server::{Http, Transport};
use openecraft_client::ehttp;
use openecraft_client::vault::{self, check, decode, entries, run_blocking, Collection, Entry, FileVersion, ItemLookup, ItemVersion, ServerInfo, Session, Upload, Uploaded, VaultEndpoint};
use openecraft_i18n::tr;

/// The project file the open project came from; saving the project to the vault
/// checks in a new version of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Origin {
    pub folder: String,
    pub name: String,
}

pub struct Vault {
    pub endpoint: VaultEndpoint,
    pub transport: Box<dyn Transport>,
    /// Name of the knowledge vault and the signed-in user (as far as known).
    pub vault_name: String,
    pub user: String,
    pub origin: Option<Origin>,
}

impl Vault {
    pub fn new(transport: Box<dyn Transport>) -> Vault {
        Vault { endpoint: VaultEndpoint::default(), transport, vault_name: String::new(), user: String::new(), origin: None }
    }

    /// Server, token and vault id from the environment (`OPENECRAFT_VAULT_URL`,
    /// `OPENECRAFT_VAULT_TOKEN`, `OPENECRAFT_VAULT_ID`).
    pub fn from_env() -> Vault {
        let env = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_string()).unwrap_or_default();
        let mut v = Vault::new(Box::new(Http));
        v.endpoint = VaultEndpoint { url: env("OPENECRAFT_VAULT_URL"), token: env("OPENECRAFT_VAULT_TOKEN"), vault: env("OPENECRAFT_VAULT_ID") };
        v
    }

    fn fetch(&self, request: &ehttp::Request) -> ehttp::Result<ehttp::Response> {
        self.transport.fetch(request)
    }

    /// Server, token and vault are known.
    pub fn connected(&self) -> Result<(), String> {
        let e = &self.endpoint;
        if e.url.trim().is_empty() || e.token.trim().is_empty() || e.vault.trim().is_empty() {
            return Err(tr("Not connected to a vault: use vault_connect first").into());
        }
        Ok(())
    }

    fn take(&mut self, session: &Session) {
        if !session.access_token.is_empty() {
            self.endpoint.token = session.access_token.clone();
        }
        if let Some(v) = &session.vault_information {
            self.endpoint.vault = v.id.clone();
            self.vault_name = v.name.clone();
        }
        if let Some(u) = &session.user_information {
            self.user = u.name.clone();
        }
    }

    /// Signs in with a Vault account; the session's token is used from now on.
    pub fn sign_in(&mut self, vault: &str, user: &str, password: &str) -> Result<Session, String> {
        let session: Session = decode(self.fetch(&self.endpoint.sign_in(vault, user, password)))?;
        self.take(&session);
        Ok(session)
    }

    /// The session of the token (e.g. of an Autodesk account), with its vault.
    pub fn adopt_session(&mut self) -> Result<Session, String> {
        let session: Session = decode(self.fetch(&self.endpoint.session()))?;
        self.take(&session);
        Ok(session)
    }

    pub fn server_info(&self) -> Result<ServerInfo, String> {
        decode(self.fetch(&self.endpoint.server_info()))
    }

    /// Contents of a folder, or search results below it (`folder`) or in the vault.
    pub fn entries(&self, folder: Option<&str>, query: Option<&str>) -> Result<Collection<Entry>, String> {
        self.connected()?;
        let req = match query {
            Some(q) => self.endpoint.search(q, folder),
            None => self.endpoint.contents(folder.unwrap_or(vault::ROOT)),
        };
        entries(self.fetch(&req))
    }

    pub fn items(&self, query: &str) -> Result<Collection<ItemVersion>, String> {
        self.connected()?;
        decode(self.fetch(&self.endpoint.items(query)))
    }

    pub fn file_version(&self, id: &str) -> Result<FileVersion, String> {
        self.connected()?;
        decode(self.fetch(&self.endpoint.file_version(id)))
    }

    pub fn download(&self, id: &str) -> Result<Vec<u8>, String> {
        self.connected()?;
        check(self.fetch(&self.endpoint.download(id))).map(|r| r.bytes)
    }

    pub fn upload(&self, upload: Upload) -> Result<Uploaded, String> {
        self.connected()?;
        run_blocking(upload, |r| self.fetch(r))
    }

    /// The items of the part numbers.
    pub fn lookup(&self, numbers: impl IntoIterator<Item = String>) -> Result<Vec<(String, ItemVersion)>, String> {
        self.connected()?;
        run_blocking(ItemLookup::new(self.endpoint.clone(), numbers)?, |r| self.fetch(r))
    }
}
