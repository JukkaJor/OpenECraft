//! Requests to Autodesk Vault through the Vault Data API (v2, Vault Professional 2025.2
//! and newer) and the decoding of its answers: signing in, browsing folders, searching
//! files and items, downloading file versions and adding or checking in files.
//! Operations of several requests are [`Job`]s: the applications chain the requests in
//! the background, the command line runs them with [`run_blocking`].

use crate::endpoint::with_content_type;
use openecraft_core::integration::Field;
use openecraft_i18n::{tr, tr_fmt};
use openecraft_parts::api::encode_component;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

/// Path of the Vault Data API on the Vault server.
pub const API_PATH: &str = "/AutodeskDM/Services/api/vault/v2";
/// Name of the application in the logs of the Vault server.
pub const APP_CODE: &str = "OpenECraft";
/// The root folder (`$`) in folder requests.
pub const ROOT: &str = "root";
/// Id of the root folder where a number is needed (adding files).
const ROOT_ID: i64 = 1;
/// Results per page (the server's upper bound).
const PAGE_SIZE: u32 = 1000;
/// Largest part of an upload; the server may ask for smaller ones.
const PART_SIZE: usize = 8 * 1024 * 1024;

/// Vault server, access token and knowledge vault; builds the requests of the API.
/// The token is sent as `Authorization: Bearer …`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VaultEndpoint {
    /// Address of the Vault server, e.g. `https://vault.example.com`.
    pub url: String,
    /// Token of a Vault session (`V:…`) or of an Autodesk account.
    pub token: String,
    /// Id of the knowledge vault.
    pub vault: String,
}

impl VaultEndpoint {
    /// The server address without the API path (users may paste either).
    fn server(&self) -> &str {
        let base = self.url.trim().trim_end_matches('/');
        base.strip_suffix(API_PATH).unwrap_or(base)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{API_PATH}{path}", self.server())
    }

    fn vault_path(&self, rest: &str) -> String {
        format!("/vaults/{}{rest}", encode_component(self.vault.trim()))
    }

    fn authorized(&self, mut req: ehttp::Request) -> ehttp::Request {
        if !self.token.trim().is_empty() {
            req.headers.insert("Authorization", format!("Bearer {}", self.token.trim()));
        }
        req
    }

    fn get(&self, path: &str) -> ehttp::Request {
        self.authorized(ehttp::Request::get(self.url(path)))
    }

    fn post(&self, path: &str, body: &impl Serialize) -> ehttp::Request {
        let req = ehttp::Request::post(self.url(path), serde_json::to_vec(body).unwrap_or_default());
        self.authorized(with_content_type(req, "application/json"))
    }

    /// POST without a body (actions such as `:checkout`).
    fn action(&self, path: &str) -> ehttp::Request {
        let mut req = ehttp::Request::post(self.url(path), Vec::new());
        req.headers.headers.retain(|(k, _)| !k.eq_ignore_ascii_case("content-type"));
        self.authorized(req)
    }

    /// GET /server-info (no sign-in needed)
    pub fn server_info(&self) -> ehttp::Request {
        ehttp::Request::get(self.url("/server-info"))
    }

    /// POST /sessions: signs in with a Vault account → [`Session`]
    pub fn sign_in(&self, vault: &str, user: &str, password: &str) -> ehttp::Request {
        let body = json!({ "input": { "vault": vault.trim(), "userName": user.trim(), "password": password, "appCode": APP_CODE } });
        VaultEndpoint { token: String::new(), ..self.clone() }.post("/sessions", &body)
    }

    /// GET /sessions/@current: the session of the token → [`Session`]
    pub fn session(&self) -> ehttp::Request {
        self.get("/sessions/@current")
    }

    /// DELETE /sessions/@current: ends the session of the token.
    pub fn sign_out(&self) -> ehttp::Request {
        self.authorized(ehttp::Request::delete(&self.url("/sessions/@current")))
    }

    /// GET /vaults → [`Collection`] of [`VaultInfo`]
    pub fn vaults(&self) -> ehttp::Request {
        self.get(&format!("/vaults?limit={PAGE_SIZE}"))
    }

    /// GET /vaults/{vault}/folders/{id}/contents: folders and latest file versions → [`entries`]
    pub fn contents(&self, folder: &str) -> ehttp::Request {
        let path = format!("/folders/{}/contents?option[latestOnly]=true&option[includeItemEcoLinks]=false&limit={PAGE_SIZE}", encode_component(folder));
        self.get(&self.vault_path(&path))
    }

    /// Searches the properties of files and folders: below `folder` (with its
    /// subfolders), or the whole vault → [`entries`]
    pub fn search(&self, text: &str, folder: Option<&str>) -> ehttp::Request {
        let q = encode_component(text.trim());
        let path = match folder {
            Some(f) => format!("/folders/{}/contents?q={q}&option[searchSubFolders]=true&option[includeItemEcoLinks]=false&limit={PAGE_SIZE}", encode_component(f)),
            None => format!("/search-results?q={q}&limit={PAGE_SIZE}"),
        };
        self.get(&self.vault_path(&path))
    }

    /// GET /vaults/{vault}/item-versions?q=…: latest item versions with their
    /// properties → [`Collection`] of [`ItemVersion`]
    pub fn items(&self, text: &str) -> ehttp::Request {
        let path = format!("/item-versions?q={}&option[propDefIds]=all&limit={PAGE_SIZE}", encode_component(text.trim()));
        self.get(&self.vault_path(&path))
    }

    /// The next page of a collection (`pagination.nextUrl`).
    pub fn next(&self, next_url: &str) -> ehttp::Request {
        let url = if next_url.starts_with("http://") || next_url.starts_with("https://") { next_url.to_string() } else { format!("{}{next_url}", self.server()) };
        self.authorized(ehttp::Request::get(url))
    }

    /// GET /vaults/{vault}/file-versions/{id} → [`FileVersion`]
    pub fn file_version(&self, id: &str) -> ehttp::Request {
        self.get(&self.vault_path(&format!("/file-versions/{}", encode_component(id))))
    }

    /// GET /vaults/{vault}/file-versions/{id}/content: the file itself
    pub fn download(&self, id: &str) -> ehttp::Request {
        self.get(&self.vault_path(&format!("/file-versions/{}/content", encode_component(id))))
    }

    /// GET /vaults/{vault}/files/{id} → [`FileObject`]
    pub fn file(&self, id: &str) -> ehttp::Request {
        self.get(&self.vault_path(&format!("/files/{}", encode_component(id))))
    }

    /// POST /vaults/{vault}/files/{id}:checkout
    pub fn checkout(&self, file: &str) -> ehttp::Request {
        self.action(&self.vault_path(&format!("/files/{}:checkout", encode_component(file))))
    }

    /// POST /vaults/{vault}/files/{id}:undo-checkout
    pub fn undo_checkout(&self, file: &str) -> ehttp::Request {
        self.action(&self.vault_path(&format!("/files/{}:undo-checkout", encode_component(file))))
    }

    /// POST /vaults/{vault}/file-uploads → [`UploadSession`]
    pub fn create_upload(&self, name: &str) -> ehttp::Request {
        self.post(&self.vault_path("/file-uploads"), &json!({ "fileName": name }))
    }

    /// PUT /vaults/{vault}/file-uploads/{upload}/parts/{index} → [`UploadSession`]
    pub fn upload_part(&self, upload: &str, index: usize, session: &str, data: Vec<u8>) -> ehttp::Request {
        let path = self.vault_path(&format!("/file-uploads/{}/parts/{index}", encode_component(upload)));
        let mut req = with_content_type(ehttp::Request::put(self.url(&path), data), "application/octet-stream");
        req.headers.insert("X-Vault-Upload-Session", session);
        self.authorized(req)
    }

    /// POST /vaults/{vault}/file-uploads/{upload}:complete → [`UploadCompletion`]
    pub fn complete_upload(&self, upload: &str, session: &str) -> ehttp::Request {
        let mut req = self.action(&self.vault_path(&format!("/file-uploads/{}:complete", encode_component(upload))));
        req.headers.insert("X-Vault-Upload-Session", session);
        req
    }

    /// POST /vaults/{vault}/files: a new file from a completed upload → [`FileObject`]
    pub fn add_file(&self, folder: &str, ticket: &str, name: &str, comment: &str) -> ehttp::Request {
        let folder_id = folder.trim().parse::<i64>().unwrap_or(ROOT_ID);
        let mut body = json!({ "folderId": folder_id, "uploadTicket": ticket, "name": name });
        if !comment.trim().is_empty() {
            body["comment"] = comment.trim().into();
        }
        self.post(&self.vault_path("/files"), &body)
    }

    /// POST /vaults/{vault}/files/{id}:checkin: a new version from a completed upload → [`FileVersion`]
    pub fn checkin(&self, file: &str, ticket: &str, comment: &str, keep_checked_out: bool) -> ehttp::Request {
        let mut body = json!({ "uploadTicket": ticket, "keepCheckedOut": keep_checked_out });
        if !comment.trim().is_empty() {
            body["comment"] = comment.trim().into();
        }
        self.post(&self.vault_path(&format!("/files/{}:checkin", encode_component(file))), &body)
    }
}

/// Answer of signing in (or of the current session).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Session {
    pub id: String,
    pub access_token: String,
    pub vault_information: Option<VaultInfo>,
    pub user_information: Option<UserInfo>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VaultInfo {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ServerInfo {
    pub name: String,
    pub product_version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Pagination {
    pub total_results: usize,
    pub next_url: Option<String>,
}

/// One page of results.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Collection<T> {
    pub pagination: Pagination,
    pub results: Vec<T>,
}

impl<T> Default for Collection<T> {
    fn default() -> Self {
        Collection { pagination: Pagination::default(), results: Vec::new() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Folder {
    pub id: String,
    pub name: String,
    /// Vault path, e.g. `$/Projects/Plant 1`.
    pub full_name: String,
}

/// The file master: the same for all versions of a file.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileMaster {
    pub id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileVersion {
    /// Id of this version (downloads).
    pub id: String,
    pub name: String,
    pub version: f64,
    pub revision: String,
    pub state: String,
    pub category: String,
    pub is_checked_out: bool,
    pub size: f64,
    pub checkin_date: String,
    pub create_user_name: String,
    pub parent_folder_id: String,
    pub file: Option<FileMaster>,
}

impl FileVersion {
    /// Id of the file master (checkout, check-in).
    pub fn file_id(&self) -> &str {
        self.file.as_ref().map(|f| f.id.as_str()).unwrap_or_default()
    }

    pub fn version_number(&self) -> u64 {
        self.version.max(0.0).round() as u64
    }
}

/// A file with its version, as `GET /files/{id}` and adding a file answer.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileObject {
    pub id: String,
    pub file_version: Option<FileVersion>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PropertyDefinition {
    pub display_name: String,
    pub system_name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Property {
    pub property_definition_id: String,
    pub definition: Option<PropertyDefinition>,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ItemVersion {
    pub id: String,
    /// Item number.
    pub number: String,
    pub title: String,
    pub revision: String,
    pub state: String,
    pub category: String,
    pub version: f64,
    pub properties: Vec<Property>,
}

impl ItemVersion {
    /// Value of the first property named like one of `names` (display or system
    /// name, compared without case, spaces and punctuation).
    pub fn property(&self, names: &[&str]) -> Option<&str> {
        let norm = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect::<String>();
        let wanted: Vec<String> = names.iter().map(|n| norm(n)).collect();
        self.properties
            .iter()
            .filter(|p| p.definition.as_ref().is_some_and(|d| wanted.contains(&norm(&d.display_name)) || wanted.contains(&norm(&d.system_name))))
            .find_map(|p| p.value.as_deref().map(str::trim).filter(|v| !v.is_empty()))
    }
}

/// An entry of a folder or of search results.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Folder(Folder),
    File(FileVersion),
    Item(ItemVersion),
}

impl Entry {
    pub fn name(&self) -> &str {
        match self {
            Entry::Folder(f) => &f.name,
            Entry::File(f) => &f.name,
            Entry::Item(i) => &i.number,
        }
    }

    /// Kind of the entry by its `entityType`, or by its fields for servers without it.
    fn from_value(v: Value) -> Option<Entry> {
        let kind = v.get("entityType").and_then(Value::as_str).unwrap_or_default().to_lowercase();
        let has = |k: &str| v.get(k).is_some();
        if kind.contains("folder") || (kind.is_empty() && has("fullName") && !has("file")) {
            serde_json::from_value(v).ok().map(Entry::Folder)
        } else if kind.contains("file") || (kind.is_empty() && (has("file") || has("parentFolderId"))) {
            serde_json::from_value(v).ok().map(Entry::File)
        } else if kind.contains("item") || (kind.is_empty() && has("number")) {
            serde_json::from_value(v).ok().map(Entry::Item)
        } else {
            None
        }
    }
}

/// Error text of a failed request: the server's title and detail, otherwise the HTTP status.
fn error_text(resp: &ehttp::Response) -> String {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct ErrorResponse {
        title: String,
        detail: String,
    }
    let e: ErrorResponse = serde_json::from_slice(&resp.bytes).unwrap_or_default();
    let parts: Vec<&str> = [e.title.trim(), e.detail.trim()].into_iter().filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        format!("{} {}", resp.status, resp.status_text).trim().to_string()
    } else {
        parts.join(": ")
    }
}

/// Successful response or a readable error.
pub fn check(result: ehttp::Result<ehttp::Response>) -> Result<ehttp::Response, String> {
    match result {
        Err(e) => Err(tr_fmt!("connection failed: {}", crate::reason(&e))),
        Ok(resp) if resp.ok => Ok(resp),
        Ok(resp) => Err(error_text(&resp)),
    }
}

/// Turns a response into the expected JSON type or a readable error.
pub fn decode<T: DeserializeOwned>(result: ehttp::Result<ehttp::Response>) -> Result<T, String> {
    let resp = check(result)?;
    serde_json::from_slice(&resp.bytes).map_err(|e| tr_fmt!("unexpected answer: {}", e))
}

/// Folders, files and items of a folder or of search results (other entities, such as
/// change orders and links, are left out).
pub fn entries(result: ehttp::Result<ehttp::Response>) -> Result<Collection<Entry>, String> {
    let page: Collection<Value> = decode(result)?;
    Ok(Collection { pagination: page.pagination, results: page.results.into_iter().filter_map(Entry::from_value).collect() })
}

/// Next request of an operation of several requests, or its result.
pub enum Step<T> {
    Send(ehttp::Request),
    Done(T),
}

/// An operation of several requests: each answer gives the next request.
pub trait Job: Send + 'static {
    type Output: Send + 'static;
    fn start(&mut self) -> ehttp::Request;
    fn next(&mut self, result: ehttp::Result<ehttp::Response>) -> Result<Step<Self::Output>, String>;
    /// Request that undoes what a failed job left behind (a checkout).
    fn cleanup(&self) -> Option<ehttp::Request> {
        None
    }
    /// What the job is doing, for the status line.
    fn status(&self) -> String;
}

/// Runs a job with a blocking transport; after a failure its cleanup request is sent.
pub fn run_blocking<J: Job>(mut job: J, mut fetch: impl FnMut(&ehttp::Request) -> ehttp::Result<ehttp::Response>) -> Result<J::Output, String> {
    let mut req = job.start();
    loop {
        match job.next(fetch(&req)) {
            Ok(Step::Send(next)) => req = next,
            Ok(Step::Done(out)) => return Ok(out),
            Err(e) => {
                if let Some(undo) = job.cleanup() {
                    let _ = fetch(&undo);
                }
                return Err(e);
            }
        }
    }
}

/// Where an upload goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// Into a folder (id or [`ROOT`]): a file of the same name gets a new version,
    /// otherwise the file is added.
    Folder(String),
    /// A new version of a file (id of the file master).
    File(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    Lookup,
    Checkout,
    Create,
    Part(usize),
    Complete,
    Commit,
    Done,
}

/// Result of an upload.
#[derive(Clone, Debug, PartialEq)]
pub struct Uploaded {
    pub version: FileVersion,
    /// The file was added (otherwise a new version was checked in).
    pub added: bool,
}

/// Puts a file into the vault: finds an existing file of the same name in the target
/// folder, checks it out, uploads the content in parts and adds the file or checks in
/// a new version.
pub struct Upload {
    endpoint: VaultEndpoint,
    target: Target,
    pub name: String,
    data: Vec<u8>,
    pub comment: String,
    pub keep_checked_out: bool,
    stage: Stage,
    /// File master, once an existing file was found.
    file: Option<String>,
    /// This upload checked the file out (undone after a failure).
    checked_out: bool,
    upload: String,
    session: String,
    part_size: usize,
    sent: usize,
    ticket: String,
}

impl Upload {
    pub fn new(endpoint: VaultEndpoint, target: Target, name: &str, data: Vec<u8>, comment: &str) -> Upload {
        Upload {
            endpoint,
            target,
            name: name.trim().to_string(),
            data,
            comment: comment.to_string(),
            keep_checked_out: false,
            stage: Stage::Lookup,
            file: None,
            checked_out: false,
            upload: String::new(),
            session: String::new(),
            part_size: PART_SIZE,
            sent: 0,
            ticket: String::new(),
        }
    }

    /// After the lookup: check out an existing file that is not checked out yet.
    fn found(&mut self, file: String, checked_out: bool) -> ehttp::Request {
        self.file = Some(file);
        if checked_out {
            self.create()
        } else {
            self.stage = Stage::Checkout;
            self.endpoint.checkout(self.file.as_deref().unwrap_or_default())
        }
    }

    fn create(&mut self) -> ehttp::Request {
        self.stage = Stage::Create;
        self.endpoint.create_upload(&self.name)
    }

    fn part(&mut self, index: usize) -> ehttp::Request {
        self.stage = Stage::Part(index);
        let end = (self.sent + self.part_size).min(self.data.len());
        self.endpoint.upload_part(&self.upload, index, &self.session, self.data[self.sent..end].to_vec())
    }

    fn continue_with(&mut self, session: UploadSession) -> Result<(), String> {
        if session.upload_session_token.trim().is_empty() {
            return Err(tr("Vault did not continue the upload").into());
        }
        self.session = session.upload_session_token;
        Ok(())
    }
}

/// Upload session of the server.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UploadSession {
    pub upload_id: String,
    pub upload_session_token: String,
    pub max_part_size: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UploadCompletion {
    pub upload_ticket: String,
}

impl Job for Upload {
    type Output = Uploaded;

    fn start(&mut self) -> ehttp::Request {
        self.stage = Stage::Lookup;
        match &self.target {
            Target::Folder(f) => self.endpoint.contents(f),
            Target::File(id) => self.endpoint.file(id),
        }
    }

    fn next(&mut self, result: ehttp::Result<ehttp::Response>) -> Result<Step<Uploaded>, String> {
        let req = match self.stage {
            Stage::Lookup => match &self.target {
                Target::Folder(_) => {
                    let page = entries(result)?;
                    let name = self.name.to_lowercase();
                    let same = page.results.into_iter().find_map(|e| match e {
                        Entry::File(f) if f.name.to_lowercase() == name && !f.file_id().is_empty() => Some(f),
                        _ => None,
                    });
                    match (same, page.pagination.next_url) {
                        (Some(f), _) => self.found(f.file_id().to_string(), f.is_checked_out),
                        (None, Some(next)) => self.endpoint.next(&next),
                        (None, None) => self.create(),
                    }
                }
                Target::File(_) => {
                    let file: FileObject = decode(result)?;
                    let checked_out = file.file_version.as_ref().is_some_and(|v| v.is_checked_out);
                    if let Some(v) = file.file_version.as_ref().filter(|v| self.name.is_empty() && !v.name.is_empty()) {
                        self.name = v.name.clone();
                    }
                    self.found(file.id, checked_out)
                }
            },
            Stage::Checkout => {
                check(result)?;
                self.checked_out = true;
                self.create()
            }
            Stage::Create => {
                let session: UploadSession = decode(result)?;
                if session.upload_id.trim().is_empty() {
                    return Err(tr("Vault did not continue the upload").into());
                }
                self.upload = session.upload_id.clone();
                if session.max_part_size > 0 {
                    self.part_size = PART_SIZE.min(usize::try_from(session.max_part_size).unwrap_or(PART_SIZE));
                }
                self.continue_with(session)?;
                self.part(0)
            }
            Stage::Part(index) => {
                self.continue_with(decode(result)?)?;
                self.sent = (self.sent + self.part_size).min(self.data.len());
                if self.sent < self.data.len() {
                    self.part(index + 1)
                } else {
                    self.stage = Stage::Complete;
                    self.endpoint.complete_upload(&self.upload, &self.session)
                }
            }
            Stage::Complete => {
                let done: UploadCompletion = decode(result)?;
                self.ticket = done.upload_ticket;
                self.stage = Stage::Commit;
                match &self.file {
                    Some(file) => self.endpoint.checkin(file, &self.ticket, &self.comment, self.keep_checked_out),
                    None => {
                        let folder = match &self.target {
                            Target::Folder(f) => f.as_str(),
                            Target::File(_) => ROOT,
                        };
                        self.endpoint.add_file(folder, &self.ticket, &self.name, &self.comment)
                    }
                }
            }
            Stage::Commit => {
                let uploaded = match &self.file {
                    Some(_) => Uploaded { version: decode(result)?, added: false },
                    None => {
                        let file: FileObject = decode(result)?;
                        let mut version = file.file_version.unwrap_or_default();
                        if version.file.is_none() {
                            version.file = Some(FileMaster { id: file.id });
                        }
                        Uploaded { version, added: true }
                    }
                };
                self.stage = Stage::Done;
                self.checked_out = false;
                return Ok(Step::Done(uploaded));
            }
            Stage::Done => return Err(tr("The upload is finished").into()),
        };
        Ok(Step::Send(req))
    }

    fn cleanup(&self) -> Option<ehttp::Request> {
        self.file.as_deref().filter(|_| self.checked_out).map(|f| self.endpoint.undo_checkout(f))
    }

    fn status(&self) -> String {
        match self.stage {
            Stage::Part(_) if !self.data.is_empty() => tr_fmt!("Uploading {} to Vault ({} %)…", self.name, self.sent * 100 / self.data.len()),
            _ => tr_fmt!("Uploading {} to Vault…", self.name),
        }
    }
}

/// Finds the Vault items of part numbers, one search per number. An item belongs to
/// a part when its number or its *Part Number* property is the part number.
pub struct ItemLookup {
    endpoint: VaultEndpoint,
    numbers: Vec<String>,
    at: usize,
    found: Vec<(String, ItemVersion)>,
}

/// Property names of the part number of an item.
const PART_NUMBER: &[&str] = &["Part Number", "PartNumber"];

impl ItemLookup {
    pub fn new(endpoint: VaultEndpoint, numbers: impl IntoIterator<Item = String>) -> Result<ItemLookup, String> {
        let numbers: BTreeSet<String> = numbers.into_iter().map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect();
        if numbers.is_empty() {
            return Err(tr("The project has no parts").into());
        }
        Ok(ItemLookup { endpoint, numbers: numbers.into_iter().collect(), at: 0, found: Vec::new() })
    }
}

/// The item of `number` among search results: the latest version with that item
/// number, otherwise with that part number.
fn item_of<'a>(number: &str, items: &'a [ItemVersion]) -> Option<&'a ItemVersion> {
    let latest = |list: Vec<&'a ItemVersion>| list.into_iter().max_by(|a, b| a.version.total_cmp(&b.version));
    latest(items.iter().filter(|i| i.number.trim() == number).collect()).or_else(|| latest(items.iter().filter(|i| i.property(PART_NUMBER) == Some(number)).collect()))
}

impl Job for ItemLookup {
    /// Part number and its item.
    type Output = Vec<(String, ItemVersion)>;

    fn start(&mut self) -> ehttp::Request {
        self.at = 0;
        self.found.clear();
        self.endpoint.items(&self.numbers[0])
    }

    fn next(&mut self, result: ehttp::Result<ehttp::Response>) -> Result<Step<Self::Output>, String> {
        let page: Collection<ItemVersion> = decode(result)?;
        let number = &self.numbers[self.at];
        if let Some(item) = item_of(number, &page.results) {
            self.found.push((number.clone(), item.clone()));
        }
        self.at += 1;
        match self.numbers.get(self.at) {
            Some(n) => Ok(Step::Send(self.endpoint.items(n))),
            None => Ok(Step::Done(std::mem::take(&mut self.found))),
        }
    }

    fn status(&self) -> String {
        tr_fmt!("Reading Vault items ({} of {})…", self.at.min(self.numbers.len()), self.numbers.len())
    }
}

/// The items of the parts as a file for the PLM connector of the integration layer:
/// item number as item ID, revision and lifecycle state per part number.
pub fn item_records(found: &[(String, ItemVersion)]) -> String {
    let rows: Vec<Value> = found
        .iter()
        .map(|(number, item)| {
            let mut m = Map::new();
            m.insert(Field::PartNumber.code().into(), number.clone().into());
            m.insert(Field::ItemId.code().into(), item.number.clone().into());
            m.insert(Field::Revision.code().into(), item.revision.clone().into());
            m.insert(Field::Lifecycle.code().into(), item.state.clone().into());
            Value::Object(m)
        })
        .collect();
    serde_json::to_string_pretty(&rows).unwrap_or_default()
}

#[cfg(test)]
mod tests;
