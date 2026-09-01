use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Health {
    Starting,
    Running,
    Degraded,
}

impl Health {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Degraded => "degraded",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Snapshot {
    health: Health,
    agent_pid: Option<u32>,
    last_error: Option<String>,
}

pub struct StatusWriter {
    path: PathBuf,
    mode: &'static str,
    last: Option<Snapshot>,
}

impl StatusWriter {
    pub fn new(runtime_dir: &Path, mode: &'static str) -> io::Result<Self> {
        fs::create_dir_all(runtime_dir)?;
        let mut permissions = fs::metadata(runtime_dir)?.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o700);
            fs::set_permissions(runtime_dir, permissions)?;
        }
        Ok(Self {
            path: runtime_dir.join("status"),
            mode,
            last: None,
        })
    }

    /// Rewrites the status file only when its content changes; the file lives on
    /// tmpfs and is read by `status`, so it is renamed into place but never synced.
    pub fn write(
        &mut self,
        health: Health,
        agent_pid: Option<u32>,
        last_error: Option<&str>,
    ) -> io::Result<()> {
        let snapshot = Snapshot {
            health,
            agent_pid,
            last_error: last_error.map(|error| error.replace(['\n', '\r'], " ")),
        };
        if self.last.as_ref() == Some(&snapshot) {
            return Ok(());
        }
        let temporary = self.path.with_extension("tmp");
        let mut file = File::create(&temporary)?;
        writeln!(file, "health={}", snapshot.health.as_str())?;
        writeln!(file, "mode={}", self.mode)?;
        writeln!(file, "broker_pid={}", std::process::id())?;
        writeln!(
            file,
            "agent_pid={}",
            snapshot
                .agent_pid
                .map_or_else(|| "-".to_owned(), |pid| pid.to_string())
        )?;
        writeln!(
            file,
            "last_error={}",
            snapshot.last_error.as_deref().unwrap_or("-")
        )?;
        fs::rename(temporary, &self.path)?;
        self.last = Some(snapshot);
        Ok(())
    }
}
