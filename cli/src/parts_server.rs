//! Blocking client for the parts server (`openecraft-parts-server`). The requests come from
//! `openecraft_client`; the transport is a trait so that the tools can be tested
//! against an in-memory server.

use crate::args::Options;
use openecraft_client::{check, decode, decode_found, ehttp, text, Endpoint};
use openecraft_parts::api::{ImportResult, PartList, PartQuery, ServerInfo};
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

    pub fn list(&self, query: &PartQuery) -> Result<PartList, String> {
        decode(self.fetch(self.endpoint.list(query)))
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
}
