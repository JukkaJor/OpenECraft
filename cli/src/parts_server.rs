//! Blocking client for the parts server (`openecraft-parts-server`). The requests come from
//! `openecraft_client`; the transport is a trait so that the tools can be tested
//! against an in-memory server.

use crate::args::Options;
use openecraft_client::{check, decode, decode_found, ehttp, text, Endpoint};
use openecraft_parts::api::{ImportResult, PartQuery, ServerInfo, Session, UserUpdate};
use openecraft_parts::portal::{AttachmentKind, Catalog, HistoryEntry, SearchResult, User};
use openecraft_parts::Part;

pub trait Transport {
    fn fetch(&self, request: &ehttp::Request) -> ehttp::Result<ehttp::Response>;
}

pub struct Http;

impl Transport for Http {
    fn fetch(&self, request: &ehttp::Request) -> ehttp::Result<ehttp::Response> {
        ehttp::fetch_blocking(request)
    }
}

pub struct Server {
    pub endpoint: Endpoint,
    pub transport: Box<dyn Transport>,
}

impl Server {
    /// Server from `--server` / `--token`, otherwise from the environment
    /// (`OPENECRAFT_SERVER`, `OPENECRAFT_TOKEN`), otherwise the local default.
    pub fn from_options(o: &Options) -> Server {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let default = Endpoint::default();
        let endpoint = Endpoint {
            url: o.server.clone().or_else(|| env("OPENECRAFT_SERVER")).unwrap_or(default.url),
            token: o.token.clone().or_else(|| env("OPENECRAFT_TOKEN")).unwrap_or_default(),
        };
        Server { endpoint, transport: Box::new(Http) }
    }

    fn fetch(&self, request: ehttp::Request) -> ehttp::Result<ehttp::Response> {
        self.transport.fetch(&request)
    }

    pub fn info(&self) -> Result<ServerInfo, String> {
        decode(self.fetch(self.endpoint.info()))
    }

    /// The part, `None` if the server does not know it.
    pub fn get(&self, number: &str) -> Result<Option<Part>, String> {
        decode_found(self.fetch(self.endpoint.part(number)))
    }

    pub fn put(&self, part: &Part) -> Result<(), String> {
        check(self.fetch(self.endpoint.save(part))).map(|_| ())
    }

    pub fn delete(&self, number: &str) -> Result<(), String> {
        check(self.fetch(self.endpoint.delete(number))).map(|_| ())
    }

    /// Uploads the content of a CSV or JSON parts file.
    pub fn import(&self, content: Vec<u8>) -> Result<ImportResult, String> {
        decode(self.fetch(self.endpoint.import(content, "text/plain; charset=utf-8")))
    }

    /// All parts as CSV.
    pub fn export(&self) -> Result<String, String> {
        text(self.fetch(self.endpoint.export()))
    }

    pub fn search(&self, query: &PartQuery) -> Result<SearchResult, String> {
        decode(self.fetch(self.endpoint.search(query)))
    }

    pub fn history(&self, number: &str) -> Result<Vec<HistoryEntry>, String> {
        decode(self.fetch(self.endpoint.history(number)))
    }

    pub fn history_part(&self, number: &str, revision: u64) -> Result<Option<Part>, String> {
        decode_found(self.fetch(self.endpoint.history_part(number, revision)))
    }

    pub fn attach(&self, number: &str, name: &str, kind: Option<AttachmentKind>, data: Vec<u8>) -> Result<Part, String> {
        decode(self.fetch(self.endpoint.attach(number, name, kind, data)))
    }

    pub fn detach(&self, number: &str, id: &str) -> Result<Part, String> {
        decode(self.fetch(self.endpoint.detach(number, id)))
    }

    pub fn file(&self, id: &str) -> Result<Vec<u8>, String> {
        check(self.fetch(self.endpoint.file(id))).map(|r| r.bytes)
    }

    pub fn catalogs(&self) -> Result<Vec<Catalog>, String> {
        decode(self.fetch(self.endpoint.catalogs()))
    }

    pub fn save_catalog(&self, catalog: &Catalog) -> Result<Catalog, String> {
        decode(self.fetch(self.endpoint.save_catalog(catalog)))
    }

    pub fn delete_catalog(&self, id: &str) -> Result<(), String> {
        check(self.fetch(self.endpoint.delete_catalog(id))).map(|_| ())
    }

    /// Signs in and uses the session's token from now on.
    pub fn login(&mut self, name: &str, password: &str) -> Result<Session, String> {
        let session: Session = decode(self.fetch(self.endpoint.login(name, password)))?;
        self.endpoint.token = session.token.clone();
        Ok(session)
    }

    pub fn favorites(&self) -> Result<Vec<String>, String> {
        decode(self.fetch(self.endpoint.favorites()))
    }

    pub fn set_favorites(&self, numbers: &[String]) -> Result<Vec<String>, String> {
        decode(self.fetch(self.endpoint.set_favorites(numbers)))
    }

    pub fn users(&self) -> Result<Vec<User>, String> {
        decode(self.fetch(self.endpoint.users()))
    }

    pub fn save_user(&self, name: &str, update: &UserUpdate) -> Result<User, String> {
        decode(self.fetch(self.endpoint.save_user(name, update)))
    }

    pub fn delete_user(&self, name: &str) -> Result<(), String> {
        check(self.fetch(self.endpoint.delete_user(name))).map(|_| ())
    }
}
