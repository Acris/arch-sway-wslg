use std::fs::File;
use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use calloop::generic::Generic;
use calloop::{Interest, LoopHandle, Mode, PostAction, RegistrationToken};
use clipboard_core::protocol::{Frame, FrameDecoder};

use crate::broker::BrokerState;
use crate::io::{set_nonblocking, write_with_deadline};

pub const RESTART_LIMIT: usize = 5;
pub const RESTART_RESET_AFTER: Duration = Duration::from_secs(60);
pub const RESTART_DELAY: Duration = Duration::from_secs(1);
// WSL interop moves the full 16 MiB in about a second on an idle system. The
// allowance per started 2 MiB keeps a loaded one from looking like a lost
// agent, while the largest frame (12 s) still ends before the heartbeat timeout.
const WRITE_TIMEOUT: Duration = Duration::from_secs(3);
const WRITE_TIMEOUT_PER_CHUNK: Duration = Duration::from_secs(1);
const WRITE_TIMEOUT_CHUNK: usize = 2 * 1024 * 1024;
const EXIT_GRACE: Duration = Duration::from_secs(1);
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);
const READ_CHUNK: usize = 64 * 1024;

/// One Win32 agent process owned by the broker event loop.
///
/// Its stdout is a `Generic` source on the same loop, so frames arrive without a
/// supervisor thread and a process is replaced only after its source is gone.
pub struct AgentProcess {
    child: Child,
    input: ChildStdin,
    chunk: Vec<u8>,
    decoder: FrameDecoder,
    token: RegistrationToken,
}

impl AgentProcess {
    pub fn spawn(
        executable: &Path,
        write_only: bool,
        handle: &LoopHandle<'static, BrokerState>,
    ) -> io::Result<Self> {
        let mut command = Command::new(executable);
        if write_only {
            command.arg("--write-only");
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let input = child.stdin.take().expect("piped agent stdin");
        let output = File::from(OwnedFd::from(
            child.stdout.take().expect("piped agent stdout"),
        ));
        // Both pipes are driven from the event loop: the level-triggered source
        // reads one chunk per wakeup and must never block on a pipe that only
        // looked readable, and writes are bounded by `write_with_deadline`.
        let registration = (|| {
            set_nonblocking(&output)?;
            set_nonblocking(&input)?;
            handle
                .insert_source(
                    Generic::new(output, Interest::READ, Mode::Level),
                    |_, output, state: &mut BrokerState| {
                        state.read_agent_output(output);
                        Ok(PostAction::Continue)
                    },
                )
                .map_err(|error| io::Error::other(error.to_string()))
        })();
        let token = match registration {
            Ok(token) => token,
            Err(error) => {
                // Even a pipe setup failure owns a running child already.
                reap(child, input);
                return Err(error);
            }
        };
        Ok(Self {
            child,
            input,
            chunk: vec![0_u8; READ_CHUNK],
            decoder: FrameDecoder::default(),
            token,
        })
    }

    /// Moves what the pipe holds into the decoder; `Ok(0)` means the agent closed it.
    pub fn fill(&mut self, output: &File) -> io::Result<usize> {
        let count = (&*output).read(&mut self.chunk)?;
        self.decoder.extend(&self.chunk[..count]);
        Ok(count)
    }

    pub fn next_frame(&mut self) -> Result<Option<Frame>, clipboard_core::protocol::ProtocolError> {
        self.decoder.next_frame()
    }

    pub fn send(&mut self, frame: &Frame) -> io::Result<()> {
        let mut encoded = Vec::new();
        frame.write_to(&mut encoded).map_err(io::Error::other)?;
        let timeout = write_timeout(encoded.len());
        write_with_deadline(&mut self.input, &encoded, timeout)
    }

    /// Closes stdin first, waits a bounded grace period, then kills and reaps.
    /// Finish before replacement or broker exit: a detached reaper can disappear
    /// with the broker, leaving its Windows child behind.
    pub fn shutdown(self, handle: &LoopHandle<'static, BrokerState>) {
        handle.remove(self.token);
        reap(self.child, self.input);
    }
}

fn write_timeout(len: usize) -> Duration {
    let chunks = u32::try_from(len.div_ceil(WRITE_TIMEOUT_CHUNK)).unwrap_or(u32::MAX);
    WRITE_TIMEOUT + WRITE_TIMEOUT_PER_CHUNK.saturating_mul(chunks)
}

// The interop stub only gets killed when the agent ignores its closed pipe.
fn reap(mut child: Child, input: ChildStdin) {
    drop(input);
    let deadline = Instant::now() + EXIT_GRACE;
    while matches!(child.try_wait(), Ok(None)) && Instant::now() < deadline {
        thread::sleep(EXIT_POLL_INTERVAL);
    }
    // Both are no-ops for a child that already exited, and `wait` is what keeps
    // a stubborn or unpollable child from lingering as a zombie.
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use clipboard_core::MAX_TEXT_BYTES;

    use super::*;
    use crate::broker::HEARTBEAT_TIMEOUT;

    #[test]
    fn write_timeout_grows_with_size_but_stays_below_the_heartbeat() {
        assert_eq!(write_timeout(0), WRITE_TIMEOUT);
        assert_eq!(write_timeout(1), WRITE_TIMEOUT + WRITE_TIMEOUT_PER_CHUNK);
        let largest = write_timeout(MAX_TEXT_BYTES + 64);
        assert!(largest > write_timeout(MAX_TEXT_BYTES / 2));
        assert!(largest < HEARTBEAT_TIMEOUT);
    }

    #[test]
    fn shutdown_reaps_an_agent_that_ignores_closed_stdin() {
        let executable =
            std::env::temp_dir().join(format!("clipboard-stalled-agent-{}", std::process::id()));
        std::fs::write(&executable, "#!/bin/sh\nexec sleep 60\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let event_loop = calloop::EventLoop::<BrokerState>::try_new().unwrap();
        let handle = event_loop.handle();
        let agent = AgentProcess::spawn(&executable, false, &handle).unwrap();
        let pid = agent.child.id();
        agent.shutdown(&handle);
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
        std::fs::remove_file(executable).unwrap();
    }
}
