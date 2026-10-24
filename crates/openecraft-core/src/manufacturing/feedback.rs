//! Feedback of the panel production: what the machines and the workshop report back
//! for the programs of a job (done, failed, rework), measured position deviations and
//! actual machine times. The feedback gives the progress of the jobs, problems for the
//! project check and the calibration of the machines' time estimates.

use super::handoff::Job;
use crate::model::Project;
use openecraft_i18n::{tr, tr_fmt};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Done,
    Started,
    Failed,
    /// Made, but has to be reworked.
    Rework,
    Skipped,
}

impl Status {
    pub const ALL: [Status; 5] = [Status::Done, Status::Started, Status::Failed, Status::Rework, Status::Skipped];

    pub fn key(self) -> &'static str {
        match self {
            Status::Done => "done",
            Status::Started => "started",
            Status::Failed => "failed",
            Status::Rework => "rework",
            Status::Skipped => "skipped",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Status::Done => tr("Done"),
            Status::Started => tr("Started"),
            Status::Failed => tr("Failed"),
            Status::Rework => tr("Rework"),
            Status::Skipped => tr("Skipped"),
        }
    }

    /// Status from the words of machines and MES systems.
    pub fn parse(s: &str) -> Option<Status> {
        match s.trim().to_lowercase().as_str() {
            "done" | "ok" | "finished" | "completed" | "complete" | "fertig" | "1" => Some(Status::Done),
            "started" | "running" | "in_progress" | "active" => Some(Status::Started),
            "failed" | "error" | "nok" | "nio" | "fehler" | "0" => Some(Status::Failed),
            "rework" | "repair" | "nacharbeit" => Some(Status::Rework),
            "skipped" | "skip" | "cancelled" | "canceled" => Some(Status::Skipped),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Feedback {
    pub job: String,
    /// Program file; empty for the whole job.
    pub program: String,
    /// Operation (sequence number); 0 for the whole program.
    pub seq: usize,
    /// Device tag or item the feedback is about.
    pub item: String,
    pub status: Status,
    /// Measured deviation of the position, mm.
    pub dx: f32,
    pub dy: f32,
    /// Actual machine time, s (0: unknown).
    pub time: f32,
    pub message: String,
    pub date: String,
}

impl Feedback {
    pub fn deviation(&self) -> f32 {
        (self.dx * self.dx + self.dy * self.dy).sqrt()
    }
}

const COLUMNS: [(&str, &[&str]); 10] = [
    ("job", &["job", "order", "job_id", "auftrag"]),
    ("program", &["program", "file", "nc_program", "programm"]),
    ("seq", &["seq", "operation", "op", "step"]),
    ("item", &["item", "tag", "device", "bmk"]),
    ("status", &["status", "state", "result"]),
    ("dx", &["dx", "deviation_x", "dev_x"]),
    ("dy", &["dy", "deviation_y", "dev_y"]),
    ("time", &["time", "duration", "time_s", "seconds"]),
    ("message", &["message", "note", "comment", "text"]),
    ("date", &["date", "timestamp", "datum"]),
];

/// Reads feedback from CSV (`;`, `,` or tab separated, with a header line) or JSON (an
/// array of objects, or an object with `feedback`), by the column names of
/// [`COLUMNS`] and their aliases.
pub fn parse(text: &str) -> Result<Vec<Feedback>, String> {
    let t = text.trim_start_matches('\u{feff}').trim();
    let rows: Vec<Vec<(String, String)>> = if t.starts_with('[') || t.starts_with('{') {
        let v: serde_json::Value = serde_json::from_str(t).map_err(|e| tr_fmt!("Invalid feedback: {}", e))?;
        let list = v.get("feedback").cloned().unwrap_or(v);
        let arr = list.as_array().ok_or_else(|| tr_fmt!("Invalid feedback: {}", tr("no list")))?;
        arr.iter()
            .filter_map(|r| r.as_object())
            .map(|o| o.iter().map(|(k, v)| (k.to_lowercase(), v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))).collect())
            .collect()
    } else {
        let mut lines = t.lines().filter(|l| !l.trim().is_empty());
        let head = lines.next().ok_or_else(|| tr_fmt!("Invalid feedback: {}", tr("no header")))?;
        let sep = [';', '\t', ','].into_iter().max_by_key(|c| head.matches(*c).count()).unwrap_or(';');
        let names: Vec<String> = head.split(sep).map(|h| h.trim().trim_matches('"').to_lowercase()).collect();
        lines.map(|l| names.iter().cloned().zip(l.split(sep).map(|c| c.trim().trim_matches('"').to_string())).collect()).collect()
    };
    let mut out = Vec::new();
    for (n, row) in rows.iter().enumerate() {
        let get = |key: &str| -> &str {
            let aliases = COLUMNS.iter().find(|c| c.0 == key).map(|c| c.1).unwrap_or(&[]);
            row.iter().find(|(k, _)| aliases.contains(&k.as_str())).map(|(_, v)| v.as_str()).unwrap_or("")
        };
        let num = |key: &str| get(key).replace(',', ".").parse::<f32>().unwrap_or(0.0);
        let status = Status::parse(get("status")).ok_or_else(|| tr_fmt!("Row {}: unknown status {}", n + 1, get("status")))?;
        let job = get("job").to_string();
        if job.is_empty() {
            return Err(tr_fmt!("Row {}: no job", n + 1));
        }
        out.push(Feedback {
            job,
            program: get("program").to_string(),
            seq: get("seq").parse().unwrap_or(0),
            item: get("item").to_string(),
            status,
            dx: num("dx"),
            dy: num("dy"),
            time: num("time").max(0.0),
            message: get("message").to_string(),
            date: get("date").to_string(),
        });
    }
    Ok(out)
}

/// Adds feedback to the project; feedback of unknown jobs is refused. Returns the
/// number of records.
pub fn import(prj: &mut Project, text: &str) -> Result<usize, String> {
    let list = parse(text)?;
    let jobs = &prj.layout.manufacturing.jobs;
    if let Some(f) = list.iter().find(|f| !jobs.iter().any(|j| j.id == f.job)) {
        return Err(tr_fmt!("Unknown job {}", f.job));
    }
    let n = list.len();
    prj.layout.manufacturing.feedback.extend(list);
    Ok(n)
}

/// State of a job from its feedback.
#[derive(Clone, Debug, PartialEq)]
pub struct JobStatus {
    pub job: String,
    pub enclosure: String,
    pub programs: usize,
    /// Programs reported done (whole program, latest feedback).
    pub done: usize,
    pub failed: usize,
    pub rework: usize,
    /// Estimated and actual time of the done programs with a time, s.
    pub estimated: f32,
    pub actual: f32,
}

impl JobStatus {
    pub fn progress(&self) -> f32 {
        if self.programs == 0 { 0.0 } else { 100.0 * self.done as f32 / self.programs as f32 }
    }

    pub fn title(&self) -> &'static str {
        if self.failed > 0 {
            tr("Problem")
        } else if self.programs > 0 && self.done == self.programs {
            tr("Completed")
        } else if self.done > 0 || self.rework > 0 {
            tr("In production")
        } else {
            tr("Released")
        }
    }
}

/// Latest whole-program feedback of each program of a job.
fn latest<'a>(fb: &'a [Feedback], job: &'a Job) -> Vec<(&'a str, &'a Feedback)> {
    let mut v: Vec<(&str, &Feedback)> = Vec::new();
    for f in fb.iter().filter(|f| f.job == job.id && f.seq == 0) {
        let names: Vec<&str> = if f.program.is_empty() { job.programs.iter().map(|p| p.name.as_str()).collect() } else { vec![f.program.as_str()] };
        for name in names {
            match v.iter_mut().find(|x| x.0 == name) {
                Some(x) => x.1 = f,
                None => v.push((name, f)),
            }
        }
    }
    v
}

pub fn status(prj: &Project) -> Vec<JobStatus> {
    let mf = &prj.layout.manufacturing;
    mf.jobs
        .iter()
        .map(|job| {
            let last = latest(&mf.feedback, job);
            let count = |s: Status| last.iter().filter(|(_, f)| f.status == s).count();
            let ops_failed = mf.feedback.iter().filter(|f| f.job == job.id && f.seq > 0 && f.status == Status::Failed).count();
            let (mut estimated, mut actual) = (0.0, 0.0);
            for (name, f) in last.iter().filter(|(_, f)| f.status == Status::Done && f.time > 0.0) {
                if let Some(p) = job.programs.iter().find(|p| p.name == *name) {
                    estimated += p.time;
                    actual += f.time / if f.program.is_empty() { job.programs.len().max(1) as f32 } else { 1.0 };
                }
            }
            JobStatus { job: job.id.clone(), enclosure: job.enclosure.clone(), programs: job.programs.len(), done: count(Status::Done), failed: count(Status::Failed) + ops_failed, rework: count(Status::Rework), estimated, actual }
        })
        .collect()
}

/// Problems from the feedback: failed and reworked programs and operations, position
/// deviations beyond the tolerance of the strategy (at least 0.5 mm).
pub fn problems(prj: &Project) -> Vec<String> {
    let mf = &prj.layout.manufacturing;
    let tolerance = mf.strategy.tolerance.max(0.5);
    let mut v = Vec::new();
    for f in &mf.feedback {
        let what = match (f.program.is_empty(), f.seq) {
            (true, _) => f.job.clone(),
            (false, 0) => format!("{} {}", f.job, f.program),
            (false, n) => format!("{} {} #{n}", f.job, f.program),
        };
        let item = if f.item.is_empty() { String::new() } else { format!(" ({})", f.item) };
        match f.status {
            Status::Failed => v.push(tr_fmt!("Production of {}{} failed: {}", what, item, f.message)),
            Status::Rework => v.push(tr_fmt!("{}{} has to be reworked: {}", what, item, f.message)),
            _ => {}
        }
        if f.deviation() > tolerance + 1e-4 {
            v.push(tr_fmt!("{}{} is {} mm off its position (tolerance {} mm)", what, item, super::num(f.deviation()), super::num(tolerance)));
        }
    }
    v
}

/// Time factors of the machines from the done programs with an actual time: (machine,
/// current factor, calibrated factor, programs): actual time against the estimate
/// without factor. With `apply` the factors are stored
/// (built-in machines are copied into the project).
pub fn calibrate(prj: &mut Project, apply: bool) -> Vec<(String, f32, f32, usize)> {
    let mf = &prj.layout.manufacturing;
    let mut sums: Vec<(String, f32, f32, usize)> = Vec::new();
    for job in &mf.jobs {
        for (name, f) in latest(&mf.feedback, job).into_iter().filter(|(_, f)| f.status == Status::Done && f.time > 0.0 && !f.program.is_empty()) {
            let Some(p) = job.programs.iter().find(|p| p.name == name).filter(|p| p.time > 0.0) else { continue };
            match sums.iter_mut().find(|s| s.0 == p.machine) {
                Some(s) => {
                    s.1 += p.time / p.factor.max(0.01);
                    s.2 += f.time;
                    s.3 += 1;
                }
                None => sums.push((p.machine.clone(), p.time / p.factor.max(0.01), f.time, 1)),
            }
        }
    }
    let mut out = Vec::new();
    for (id, est, act, n) in sums {
        let Some(mut m) = super::machines::find(prj, &id) else { continue };
        let factor = (act / est.max(0.01)).clamp(0.1, 10.0);
        out.push((id.clone(), m.time_factor, factor, n));
        if apply {
            m.time_factor = factor;
            let own = &mut prj.layout.manufacturing.machines;
            match own.iter_mut().find(|x| x.id == id) {
                Some(x) => *x = m,
                None => own.push(m),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
