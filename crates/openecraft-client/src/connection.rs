//! Connection to a source of parts: the OpenECraft parts server or the EPLAN Data
//! Portal (through [`EplanEndpoint`]). Both answer the reading requests in the types
//! of the parts server API; the other functions only exist on the parts server.

use crate::eplan::{self, EplanEndpoint};
use crate::{check, decode, decode_found, Endpoint};
use openecraft_i18n::tr;
use openecraft_parts::api::{PartQuery, ServerInfo};
use openecraft_parts::portal::SearchResult;
use openecraft_parts::Part;
use serde::{Deserialize, Serialize};

pub type Decoder<T> = Box<dyn FnOnce(ehttp::Result<ehttp::Response>) -> Result<T, String> + Send>;

/// A request and the decoding of its answer.
pub struct Call<T> {
    pub request: ehttp::Request,
    pub decode: Decoder<T>,
}

impl<T: 'static> Call<T> {
    pub fn new(request: ehttp::Request, decode: impl FnOnce(ehttp::Result<ehttp::Response>) -> Result<T, String> + Send + 'static) -> Call<T> {
        Call { request, decode: Box::new(decode) }
    }

    /// Sends the request with `fetch` (blocking) and decodes the answer.
    pub fn run(self, fetch: impl FnOnce(&ehttp::Request) -> ehttp::Result<ehttp::Response>) -> Result<T, String> {
        (self.decode)(fetch(&self.request))
    }
}

/// Kind of the server a parts connection talks to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortalKind {
    /// `openecraft-parts-server`.
    #[default]
    OpenECraft,
    /// The EPLAN Data Portal (read-only).
    Eplan,
}

impl PortalKind {
    pub const ALL: [PortalKind; 2] = [PortalKind::OpenECraft, PortalKind::Eplan];

    pub fn title(self) -> &'static str {
        match self {
            PortalKind::OpenECraft => "OpenECraft parts server",
            PortalKind::Eplan => "EPLAN Data Portal",
        }
    }

    pub fn default_url(self) -> String {
        match self {
            PortalKind::OpenECraft => Endpoint::default().url,
            PortalKind::Eplan => EplanEndpoint::default().url,
        }
    }

    /// Only searching and reading parts and their files: no changes, accounts,
    /// favorites, history or download of the whole database.
    pub fn read_only(self) -> bool {
        self == PortalKind::Eplan
    }
}

/// A parts server of either kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Connection {
    Parts(Endpoint),
    Eplan(EplanEndpoint),
}

impl Connection {
    pub fn new(kind: PortalKind, url: &str, token: &str) -> Connection {
        let (url, token) = (url.to_string(), token.to_string());
        match kind {
            PortalKind::OpenECraft => Connection::Parts(Endpoint { url, token }),
            PortalKind::Eplan => Connection::Eplan(EplanEndpoint { url, token }),
        }
    }

    pub fn kind(&self) -> PortalKind {
        match self {
            Connection::Parts(_) => PortalKind::OpenECraft,
            Connection::Eplan(_) => PortalKind::Eplan,
        }
    }

    /// The parts server for the functions only it offers.
    pub fn parts_server(&self) -> Result<&Endpoint, String> {
        match self {
            Connection::Parts(e) => Ok(e),
            Connection::Eplan(_) => Err(tr("The EPLAN Data Portal only offers searching and reading parts").into()),
        }
    }

    pub fn info(&self) -> Call<ServerInfo> {
        match self {
            Connection::Parts(e) => Call::new(e.info(), decode),
            Connection::Eplan(e) => Call::new(e.info(), eplan::decode_info),
        }
    }

    /// Faceted search; a query with part numbers loads these parts.
    pub fn search(&self, query: &PartQuery) -> Call<SearchResult> {
        match self {
            Connection::Parts(e) => Call::new(e.search(query), decode),
            Connection::Eplan(e) => {
                let q = query.clone();
                Call::new(e.search(query), move |r| eplan::decode_search(&q, r))
            }
        }
    }

    /// The part, `None` if the server does not know it.
    pub fn part(&self, number: &str) -> Call<Option<Part>> {
        match self {
            Connection::Parts(e) => Call::new(e.part(number), decode_found),
            Connection::Eplan(e) => Call::new(e.part(number), eplan::decode_part),
        }
    }

    /// A file of a part ([`openecraft_parts::portal::Attachment::id`]).
    pub fn file(&self, id: &str) -> Call<Vec<u8>> {
        match self {
            Connection::Parts(e) => Call::new(e.file(id), |r| check(r).map(|r| r.bytes)),
            Connection::Eplan(e) => Call::new(e.file(id), eplan::decode_file),
        }
    }

    /// Address of a file for the web browser; `None` if loading it needs the token.
    pub fn file_link(&self, id: &str) -> Option<String> {
        match self {
            Connection::Parts(e) => Some(e.file_url(id)),
            Connection::Eplan(e) => (!e.needs_token(id)).then(|| e.file_url(id)),
        }
    }
}

#[cfg(test)]
mod tests;
