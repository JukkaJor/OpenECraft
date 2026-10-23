use crate::endpoint::with_content_type;
use openecraft_core::collab::{
    project_path, CreateRequest, LockRequest, OpenRequest, SaveRequest, Session, Target, API_PREFIX, DEFAULT_PORT,
};
use openecraft_core::model::Project;
use openecraft_parts::api;
use serde::Serialize;

/// Address and access token of a project server; builds the requests of its API
/// (`openecraft_core::collab`). Every request carries the token, if one is set.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectEndpoint {
    pub url: String,
    pub token: String,
}

impl Default for ProjectEndpoint {
    fn default() -> Self {
        ProjectEndpoint { url: format!("http://127.0.0.1:{DEFAULT_PORT}"), token: String::new() }
    }
}

impl ProjectEndpoint {
    fn request(&self, mut req: ehttp::Request) -> ehttp::Request {
        if !self.token.trim().is_empty() {
            req.headers.insert("Authorization", format!("Bearer {}", self.token.trim()));
        }
        req
    }

    fn get(&self, path: &str) -> ehttp::Request {
        self.request(ehttp::Request::get(api::url(&self.url, path)))
    }

    fn json(&self, method: &str, path: &str, body: &impl Serialize) -> ehttp::Request {
        let body = serde_json::to_vec(body).unwrap_or_default();
        let url = api::url(&self.url, path);
        let req = if method == "PUT" { ehttp::Request::put(url, body) } else { ehttp::Request::post(url, body) };
        self.request(with_content_type(req, "application/json"))
    }

    /// GET /api/v1/info
    pub fn info(&self) -> ehttp::Request {
        self.get(&format!("{API_PREFIX}/info"))
    }

    /// GET /api/v1/projects
    pub fn list(&self) -> ehttp::Request {
        self.get(&format!("{API_PREFIX}/projects"))
    }

    /// POST /api/v1/projects: stores a new project and opens it.
    pub fn create(&self, name: &str, user: &str, project: &Project) -> ehttp::Request {
        let req = CreateRequest { name: name.into(), user: user.into(), project: project.clone() };
        self.json("POST", &format!("{API_PREFIX}/projects"), &req)
    }

    /// GET /api/v1/projects/{name}
    pub fn snapshot(&self, name: &str) -> ehttp::Request {
        self.get(&project_path(name))
    }

    /// POST /api/v1/projects/{name}/open
    pub fn open(&self, name: &str, user: &str) -> ehttp::Request {
        self.json("POST", &format!("{}/open", project_path(name)), &OpenRequest { user: user.into() })
    }

    /// PUT /api/v1/projects/{name}: saves what the session has locked.
    pub fn save(&self, name: &str, session: &Session, project: &Project, release: Vec<Target>) -> ehttp::Request {
        let req = SaveRequest { session: session.clone(), project: project.clone(), release };
        self.json("PUT", &project_path(name), &req)
    }

    /// POST /api/v1/projects/{name}/status
    pub fn status(&self, name: &str, session: &Session) -> ehttp::Request {
        self.json("POST", &format!("{}/status", project_path(name)), session)
    }

    /// POST /api/v1/projects/{name}/lock
    pub fn lock(&self, name: &str, session: &Session, targets: Vec<Target>) -> ehttp::Request {
        self.json("POST", &format!("{}/lock", project_path(name)), &LockRequest { session: session.clone(), targets })
    }

    /// POST /api/v1/projects/{name}/release
    pub fn release(&self, name: &str, session: &Session, targets: Vec<Target>) -> ehttp::Request {
        self.json("POST", &format!("{}/release", project_path(name)), &LockRequest { session: session.clone(), targets })
    }

    /// POST /api/v1/projects/{name}/close
    pub fn close(&self, name: &str, session: &Session) -> ehttp::Request {
        self.json("POST", &format!("{}/close", project_path(name)), session)
    }
}

#[cfg(test)]
mod tests;
