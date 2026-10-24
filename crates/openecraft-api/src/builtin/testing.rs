//! Test helpers of the built-in commands: a session like the one of the command line
//! (`run` a command by name on the open project).

use crate::Plugins;
use openecraft_core::model::Project;
use openecraft_core::sample::demo_project;
use serde_json::Value;

pub struct Session {
    pub plugins: Plugins,
    project: Option<Project>,
}

impl Session {
    pub fn project(&self) -> Result<&Project, String> {
        self.project.as_ref().ok_or_else(|| "No project is open".to_string())
    }

    pub fn project_mut(&mut self) -> Result<&mut Project, String> {
        self.project.as_mut().ok_or_else(|| "No project is open".to_string())
    }
}

/// Session without a project.
pub fn session() -> Session {
    Session { plugins: Plugins::new(Vec::<String>::new()), project: None }
}

/// Session with `prj` open.
pub fn with_project(mut prj: Project) -> Session {
    prj.normalize();
    Session { project: Some(prj), ..session() }
}

/// Session with a new, empty project named "Test" (like `new_project` of the command line).
pub fn new_session() -> Session {
    let mut prj = Project::new_default();
    prj.info.name = "Test".into();
    with_project(prj)
}

/// Session with the demo project.
pub fn demo_session() -> Session {
    with_project(demo_project())
}

/// Runs a command with JSON arguments.
pub fn run(s: &mut Session, name: &str, args: Value) -> Result<Value, String> {
    let cmd = s.plugins.command(name).cloned().unwrap_or_else(|| panic!("no command {name}"));
    let prj = s.project.as_mut().ok_or_else(|| "No project is open".to_string())?;
    s.plugins.run(&cmd, prj, args.as_object().unwrap())
}
