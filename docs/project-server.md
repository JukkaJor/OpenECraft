# Project server

Shared projects for a team: several users work on the same project, each page is locked by one of them.

`openecraft-project-server` stores projects for a team. Several users open the same
project at the same time and work on different pages; a page is edited by one user at a
time.

```bash
cargo run --release -p openecraft-project-server -- --bind 0.0.0.0:8091 --data projects --token <secret>
cargo run --release -p openecraft-project-server -- --config project-server.toml
```

- Configuration file (`--config <file>` or `OPENECRAFT_PROJECT_SERVER_CONFIG`): the
  settings of the command line options in TOML (`bind`, `data`, `token`, `admin_user`,
  `access`; see
  [`project-server/project-server.example.toml`](../project-server/project-server.example.toml)),
  as for the parts server.
- Project names differing only in case are one name (Windows and macOS do not tell
  their files apart); Windows device names (`CON`, `NUL`, `COM1` …) and names ending
  with a dot are refused.
- Each project is stored as `<name>.oec` (a normal project file) next to
  `<name>.state.json` (revision, id counter) in the data directory.
- With `--token` (or `OPENECRAFT_PROJECT_TOKEN`) every request needs
  `Authorization: Bearer <token>`; without it, user accounts and directories everyone
  who reaches the server can change its projects.
- **User accounts** (`users.json` in the data directory, Argon2 password hashes):
  viewers open projects read-only, reviewers also save comments, markup and approvals,
  editors lock, change, save and create projects, project managers also delete
  projects and manage who may use them, administrators also manage the users; a user's
  role can be limited to some projects (names with `*` / `?`). `--admin-user <name>`
  creates the first administrator with the password from
  `OPENECRAFT_PROJECT_ADMIN_PASSWORD`. Users sign in from *Tools › Project server…*
  (user name, password, *Sign in*); the server uses the account's name on the locks,
  whatever the request says.
- **Access control** (`access.json` in the data directory, `--access`): users sign in
  with a local account or with Active Directory / LDAP directories; roles (built-in or
  defined by administrators) are granted to users and directory groups per project,
  deny grants take rights away. Whoever creates a project manages it; project managers
  edit its grants in the *Project access* section of the project server window. See
  [docs/access-control.md](access-control.md).
- Locks: every page and the project data (page list and order, devices, parts, panel
  layout, settings) are locked separately. Editing something you have not locked
  requests the lock automatically; if another user holds it, the change is taken back.
  Pages locked by others are read-only and marked with 🔒 and the user's name in the
  page list. Locks end when they are released, when the project is closed, or 90 s after
  the editor stopped sending its status (every 15 s).
- Saving merges only what you have locked into the server's project; everything else
  comes from the server. Changes of other users are loaded automatically.
  A save names the revision it was edited from (`base_revision`; the editor does):
  the server then also takes over changed pages and project data nobody has locked
  (it locks them for you first) and answers with the changes it left out
  (`ignored`): locked by another user, changed by another user since that revision or
  not allowed. With `strict` nothing is saved in that case (`409`). The server keeps
  the last 16 revisions per project in memory; a save from an older revision (or
  without `base_revision`) saves only what is locked.
- Each session gets its own range of object ids, so users never create the same id.
- In the editor: *Tools › Project server…* – enter the server URL, your user name and the
  token, list the projects and open one, or upload the current project. While a shared
  project is open, *Save* (`Ctrl+S`) saves to the server; the window and the page list's
  context menu lock and release pages.
- REST API (`/api/v1`): `GET /info`, `GET|POST /projects`, `GET|PUT|DELETE /projects/{name}`,
  `POST /projects/{name}/open|status|lock|release|close`, `GET|PUT /projects/{name}/access`,
  `POST /login`, `POST /logout`, `GET /users`, `PUT|DELETE /users/{name}`, `GET|PUT /access`
  (see `openecraft_core::collab`).
