use std::env;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use calloop::timer::{TimeoutAction, Timer};
use calloop::{EventLoop, LoopHandle, LoopSignal};
use calloop_wayland_source::WaylandSource;
use clipboard_core::protocol::{Frame, HELLO_HAS_TEXT, HELLO_READ_ERROR, MessageKind};
use clipboard_core::state::{MirrorState, TextHash};
use clipboard_core::text::validate_utf8;
use thiserror::Error;
use wayland_client::{Connection, QueueHandle};

use crate::agent::{AgentProcess, RESTART_DELAY, RESTART_LIMIT, RESTART_RESET_AFTER};
use crate::status::{Health, StatusWriter};
use crate::wayland::{WaylandEvent, WaylandState};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);
// A first start of the unsigned agent can sit in a security scan for a while;
// counting that against the restart budget would take the clipboard down for good.
const HELLO_TIMEOUT: Duration = Duration::from_secs(45);
const WINDOWS_WRITE_RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardMode {
    Both,
    ToWindows,
}

impl ClipboardMode {
    pub fn from_env() -> Result<Self, BrokerError> {
        match env::var("ARCH_SWAY_WSLG_CLIPBOARD")
            .unwrap_or_else(|_| "both".into())
            .as_str()
        {
            "both" => Ok(Self::Both),
            "to-windows" => Ok(Self::ToWindows),
            value => Err(BrokerError::Configuration(format!(
                "invalid ARCH_SWAY_WSLG_CLIPBOARD value: {value}"
            ))),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::ToWindows => "to-windows",
        }
    }
}

pub struct BrokerConfig {
    pub mode: ClipboardMode,
    pub sync_sensitive: bool,
    pub runtime_dir: PathBuf,
    pub agent: PathBuf,
}

pub(crate) struct BrokerState {
    mode: ClipboardMode,
    mirror: MirrorState,
    pub(crate) wayland: WaylandState,
    qh: QueueHandle<BrokerState>,
    handle: LoopHandle<'static, BrokerState>,
    status: StatusWriter,
    agent_executable: PathBuf,
    agent: Option<AgentProcess>,
    agent_pid: Option<u32>,
    agent_ready: bool,
    agent_started: Instant,
    agent_restarts: usize,
    restart_scheduled: bool,
    last_agent_response: Instant,
    wayland_ready: bool,
    latest_wayland_generation: u64,
    ever_synced: bool,
    slots: SyncSlots,
    // The Sway text the agent is writing right now, kept until its ACK so that a
    // refused or interrupted write can be queued again. After a refusal it stays
    // here until the retry timer fires or something newer supersedes it.
    in_flight: Option<WindowsWrite>,
    retry_scheduled: bool,
    fatal: Option<BrokerError>,
    stop: LoopSignal,
}

/// A text and its digest, computed once when the text enters the broker.
#[derive(Debug, Eq, PartialEq)]
struct HashedText {
    text: Vec<u8>,
    hash: TextHash,
}

impl HashedText {
    fn new(text: Vec<u8>) -> Self {
        let hash = MirrorState::hash(&text);
        Self { text, hash }
    }
}

struct WindowsWrite {
    text: HashedText,
    retried: bool,
}

#[derive(Debug, Eq, PartialEq)]
enum WindowsSelection {
    Text(HashedText, u32),
    Unavailable(u32),
}

impl WindowsSelection {
    const fn sequence(&self) -> u32 {
        match self {
            Self::Text(_, sequence) | Self::Unavailable(sequence) => *sequence,
        }
    }
}

/// The newest selection of each side that has not reached the other side yet.
///
/// A slot fills while the peer is not ready or a Windows write is in flight and
/// is drained as soon as both directions are idle. When both slots hold something
/// the Windows selection wins: its sequence number proves it postdates everything
/// the broker committed, whereas the Sway text's order relative to it is unknown.
#[derive(Debug, Default)]
struct SyncSlots {
    to_windows: Option<HashedText>,
    to_wayland: Option<WindowsSelection>,
}

#[derive(Debug, Eq, PartialEq)]
enum Delivery {
    ToWindows(HashedText),
    ToWayland(WindowsSelection),
}

#[derive(Debug, Eq, PartialEq)]
enum WaylandTextDisposition {
    Duplicate,
    WindowsEcho,
    Forwarded,
}

impl SyncSlots {
    fn wayland_text(&mut self, text: HashedText) {
        self.to_windows = Some(text);
    }

    fn wayland_non_text(&mut self) {
        self.to_windows = None;
    }

    fn windows_selection(&mut self, selection: WindowsSelection) {
        self.to_wayland = Some(selection);
    }

    // The next Hello re-describes the Windows side; the Sway text waits for it.
    fn agent_lost(&mut self) {
        self.to_wayland = None;
    }

    // A write the agent never acknowledged goes back in line unless Sway moved on.
    fn requeue_windows_write(&mut self, text: HashedText) {
        if self.to_windows.is_none() {
            self.to_windows = Some(text);
        }
    }

    // A Windows selection observed before our own write landed is stale.
    fn windows_write_committed(&mut self, sequence: u32) {
        if self
            .to_wayland
            .as_ref()
            .is_some_and(|selection| !sequence_is_newer(selection.sequence(), sequence))
        {
            self.to_wayland = None;
        }
    }

    const fn is_empty(&self) -> bool {
        self.to_windows.is_none() && self.to_wayland.is_none()
    }

    fn take(&mut self) -> Option<Delivery> {
        if let Some(selection) = self.to_wayland.take() {
            self.to_windows = None;
            return Some(Delivery::ToWayland(selection));
        }
        self.to_windows.take().map(Delivery::ToWindows)
    }
}

fn observe_wayland_text(
    mirror: &mut MirrorState,
    slots: &mut SyncSlots,
    text: HashedText,
) -> WaylandTextDisposition {
    if mirror.is_wayland_echo(&text.hash) {
        return WaylandTextDisposition::Duplicate;
    }
    mirror.observe_wayland(text.hash);
    if mirror.is_windows_echo(&text.hash) {
        WaylandTextDisposition::WindowsEcho
    } else {
        slots.wayland_text(text);
        WaylandTextDisposition::Forwarded
    }
}

/// What to do with a Windows write the agent has just refused.
#[derive(Debug, Eq, PartialEq)]
enum WriteFailure {
    Retry,
    /// A newer selection on either side is about to replace the text anyway.
    Superseded,
    Exhausted,
}

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("Wayland connection failed: {0}")]
    Wayland(String),
    #[error("{0}")]
    Exhausted(String),
    #[error("event loop failed: {0}")]
    EventLoop(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl BrokerError {
    /// Exit status for a failure another start would only repeat; the launcher
    /// excludes it from `Restart=on-failure` so the clipboard stays down instead
    /// of cycling through the start limit for the rest of the session.
    pub const GAVE_UP: i32 = 3;

    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::Configuration(_) | Self::Wayland(_) | Self::Exhausted(_) => Self::GAVE_UP,
            Self::EventLoop(_) | Self::Io(_) => 1,
        }
    }
}

pub fn run(config: BrokerConfig) -> Result<(), BrokerError> {
    let mut status = StatusWriter::new(&config.runtime_dir, config.mode.as_str())?;
    status.write(Health::Starting, None, None)?;

    let connection =
        Connection::connect_to_env().map_err(|error| BrokerError::Wayland(error.to_string()))?;
    let mut event_queue = connection.new_event_queue::<BrokerState>();
    let qh = event_queue.handle();
    connection.display().get_registry(&qh, ());

    let mut event_loop: EventLoop<'static, BrokerState> =
        EventLoop::try_new().map_err(|error| BrokerError::EventLoop(error.to_string()))?;
    let handle = event_loop.handle();

    let mut state = BrokerState {
        mode: config.mode,
        mirror: MirrorState::default(),
        wayland: WaylandState::new(handle.clone(), config.sync_sensitive),
        qh,
        handle: handle.clone(),
        status,
        agent_executable: config.agent,
        agent: None,
        agent_pid: None,
        agent_ready: false,
        agent_started: Instant::now(),
        agent_restarts: 0,
        restart_scheduled: false,
        last_agent_response: Instant::now(),
        wayland_ready: false,
        latest_wayland_generation: 0,
        ever_synced: false,
        slots: SyncSlots::default(),
        in_flight: None,
        retry_scheduled: false,
        fatal: None,
        stop: event_loop.get_signal(),
    };

    // A compositor without the globals would otherwise leave the broker in
    // `starting` forever; fail here so systemd and `status` can tell.
    event_queue
        .roundtrip(&mut state)
        .map_err(|error| BrokerError::Wayland(error.to_string()))?;
    if let Some(missing) = state.wayland.missing_global() {
        return Err(BrokerError::Wayland(format!(
            "compositor does not provide {missing}"
        )));
    }

    WaylandSource::new(connection, event_queue)
        .insert(handle.clone())
        .map_err(|error| BrokerError::EventLoop(error.to_string()))?;
    handle
        .insert_source(Timer::from_duration(HEARTBEAT_INTERVAL), |_, _, state| {
            state.heartbeat();
            TimeoutAction::ToDuration(HEARTBEAT_INTERVAL)
        })
        .map_err(|error| BrokerError::EventLoop(error.error.to_string()))?;

    state.start_agent();
    let result = event_loop.run(None, &mut state, |_| {});
    // Reap the agent before the exit status tells systemd the broker is gone,
    // whichever way the loop ended.
    if let Some(agent) = state.agent.take() {
        agent.shutdown_now(&state.handle);
    }
    if let Some(error) = state.fatal.take() {
        return Err(error);
    }
    match result {
        Ok(()) => Ok(()),
        // Sway closing the socket is how the session ends; the scope stops the
        // broker moments later, so a restart would only add log noise.
        Err(error) if compositor_closed(&error) => {
            eprintln!("clipboard: compositor closed the connection");
            Ok(())
        }
        Err(error) => Err(BrokerError::EventLoop(error.to_string())),
    }
}

// calloop wraps a source's error once more, so the socket error sits a few
// levels down the chain.
fn compositor_closed(error: &calloop::Error) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(candidate) = current {
        if let Some(io_error) = candidate.downcast_ref::<io::Error>() {
            return matches!(
                io_error.kind(),
                io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset
            );
        }
        current = candidate.source();
    }
    false
}

impl BrokerState {
    pub(crate) fn handle_wayland_event(&mut self, event: WaylandEvent) {
        match event {
            WaylandEvent::SelectionStarted(generation) => {
                self.latest_wayland_generation = generation;
                self.mirror.invalidate_wayland();
                // A newer selection supersedes queued Sway text even while its
                // offer is still being read.
                self.slots.wayland_non_text();
            }
            WaylandEvent::Text { generation, text } => {
                if generation != self.latest_wayland_generation {
                    return;
                }
                match accept_wayland_text(text) {
                    Ok(text) => {
                        // Every offer is read, including the announcement of a
                        // selection this broker published. The text hash, not
                        // cross-client event order, identifies an echo.
                        if observe_wayland_text(&mut self.mirror, &mut self.slots, text)
                            == WaylandTextDisposition::WindowsEcho
                            && self.agent_ready
                        {
                            self.write_status(Health::Running, None);
                        }
                    }
                    Err(error) => self.withdraw_wayland_selection(error.as_deref()),
                }
                self.wayland_described();
            }
            WaylandEvent::Unusable { generation, error } => {
                if generation == self.latest_wayland_generation {
                    self.withdraw_wayland_selection(error.as_deref());
                    self.wayland_described();
                }
            }
            WaylandEvent::DeviceLost => self.fail(BrokerError::EventLoop(
                "ext-data-control device was removed".into(),
            )),
        }
    }

    // Sway has described its selection, so its side can be served from now on.
    fn wayland_described(&mut self) {
        if !self.wayland_ready {
            self.wayland_ready = true;
            if self.agent_ready {
                self.sides_ready();
            }
        }
        self.drain();
    }

    // Both sides have described their selection: the mirror is running.
    fn sides_ready(&mut self) {
        self.ever_synced = true;
        self.write_status(Health::Running, None);
    }

    fn withdraw_wayland_selection(&mut self, error: Option<&str>) {
        self.mirror.invalidate_wayland();
        self.slots.wayland_non_text();
        if let Some(error) = error {
            self.reject(error);
        }
    }

    // Agent process lifecycle

    fn start_agent(&mut self) {
        self.restart_scheduled = false;
        self.agent_started = Instant::now();
        self.last_agent_response = Instant::now();
        match AgentProcess::spawn(
            &self.agent_executable,
            self.mode == ClipboardMode::ToWindows,
            &self.handle,
        ) {
            Ok(agent) => self.agent = Some(agent),
            Err(error) => self.lose_agent(format!(
                "failed to start {}: {error}",
                self.agent_executable.display()
            )),
        }
    }

    pub(crate) fn read_agent_output(&mut self, output: &File) {
        let Some(result) = self.agent.as_mut().map(|agent| agent.fill(output)) else {
            return;
        };
        match result {
            Ok(0) => {
                self.lose_agent("Windows clipboard agent closed its pipe".into());
                return;
            }
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                ) =>
            {
                return;
            }
            Err(error) => {
                self.lose_agent(format!("Windows clipboard agent read failed: {error}"));
                return;
            }
        }
        // A frame handler may replace the agent, which ends this process's stream.
        loop {
            match self.agent.as_mut().map(AgentProcess::next_frame) {
                Some(Ok(Some(frame))) => self.handle_agent_frame(frame),
                Some(Ok(None)) | None => return,
                Some(Err(error)) => {
                    self.lose_agent(format!("Windows clipboard agent protocol failed: {error}"));
                    return;
                }
            }
        }
    }

    fn lose_agent(&mut self, reason: String) {
        if let Some(agent) = self.agent.take() {
            agent.shutdown(&self.handle);
        }
        self.agent_pid = None;
        self.agent_ready = false;
        self.slots.agent_lost();
        self.mirror.reset_windows_transport();
        if let Some(write) = self.in_flight.take() {
            self.slots.requeue_windows_write(write.text);
        }
        self.degrade(&reason);
        if self.restart_scheduled {
            return;
        }
        if self.agent_started.elapsed() >= RESTART_RESET_AFTER {
            self.agent_restarts = 0;
        }
        self.agent_restarts += 1;
        if self.agent_restarts > RESTART_LIMIT {
            self.fail(BrokerError::Exhausted(format!(
                "Windows clipboard agent restart budget exhausted: {reason}"
            )));
            return;
        }
        self.restart_scheduled = true;
        let restart =
            self.handle
                .insert_source(Timer::from_duration(RESTART_DELAY), |_, _, state| {
                    state.start_agent();
                    TimeoutAction::Drop
                });
        if let Err(error) = restart {
            self.fail(BrokerError::EventLoop(format!(
                "cannot schedule Windows clipboard agent restart: {error}"
            )));
        }
    }

    fn send_frame(&mut self, frame: &Frame) -> bool {
        let Some(agent) = self.agent.as_mut() else {
            return false;
        };
        match agent.send(frame) {
            Ok(()) => true,
            Err(error) => {
                self.lose_agent(format!("Windows clipboard agent write failed: {error}"));
                false
            }
        }
    }

    fn heartbeat(&mut self) {
        if self.agent.is_none() {
            return;
        }
        let silence = self.last_agent_response.elapsed();
        if self.agent_ready {
            if silence >= HEARTBEAT_TIMEOUT {
                self.lose_agent("Windows clipboard agent heartbeat timed out".into());
            } else {
                self.send_frame(&Frame::new(MessageKind::Ping));
            }
        } else if silence >= HELLO_TIMEOUT {
            self.lose_agent("Windows clipboard agent did not report its startup state".into());
        }
    }

    // Agent frames

    fn handle_agent_frame(&mut self, frame: Frame) {
        self.last_agent_response = Instant::now();
        match frame.kind {
            MessageKind::Hello => self.handle_hello(frame),
            MessageKind::WindowsText | MessageKind::WindowsUnavailable
                if self.mode == ClipboardMode::Both =>
            {
                let selection =
                    if frame.kind == MessageKind::WindowsText && !frame.payload.is_empty() {
                        WindowsSelection::Text(HashedText::new(frame.payload), frame.sequence)
                    } else {
                        WindowsSelection::Unavailable(frame.sequence)
                    };
                self.slots.windows_selection(selection);
                self.drain();
            }
            MessageKind::SetWindowsOk => {
                if self
                    .mirror
                    .commit_windows_write(frame.request_id, frame.sequence)
                {
                    self.in_flight = None;
                    self.slots.windows_write_committed(frame.sequence);
                    self.write_status(Health::Running, None);
                    self.drain();
                }
            }
            MessageKind::SetWindowsError => {
                if self.mirror.fail_windows_write(frame.request_id) {
                    self.windows_write_failed(&String::from_utf8_lossy(&frame.payload));
                    self.drain();
                }
            }
            MessageKind::Pong
            | MessageKind::ProtocolError
            | MessageKind::WindowsText
            | MessageKind::WindowsUnavailable => {}
            MessageKind::Ping | MessageKind::SetWindowsText => {
                self.lose_agent("Windows clipboard agent sent an unexpected frame".into());
            }
        }
    }

    fn handle_hello(&mut self, frame: Frame) {
        let has_text = frame.flags & HELLO_HAS_TEXT != 0;
        let read_error = frame.flags & HELLO_READ_ERROR != 0;
        let pid = u32::try_from(frame.request_id).ok();
        if pid.is_none()
            || self.agent_ready
            || (read_error && has_text)
            || (!has_text && !frame.payload.is_empty())
        {
            self.lose_agent("Windows clipboard agent sent an inconsistent startup state".into());
            return;
        }
        self.agent_pid = pid;
        self.agent_ready = true;
        let sequence = frame.sequence;
        let text = (has_text && !frame.payload.is_empty()).then(|| HashedText::new(frame.payload));
        let windows_changed = self.mirror.windows_changed_since(sequence)
            || text
                .as_ref()
                .is_some_and(|text| !self.mirror.is_windows_echo(&text.hash));
        match &text {
            Some(text) => self.mirror.observe_windows(text.hash, sequence),
            None => self.mirror.invalidate_windows(sequence),
        }
        if self.mode == ClipboardMode::Both
            && let Some(selection) = startup_windows_selection(
                self.ever_synced,
                windows_changed,
                read_error,
                text,
                sequence,
            )
        {
            self.slots.windows_selection(selection);
        }
        if self.wayland_ready {
            self.sides_ready();
        }
        self.drain();
    }

    // Delivery

    fn drain(&mut self) {
        if !self.wayland_ready || !self.agent_ready || self.mirror.has_pending_windows_write() {
            return;
        }
        let Some(delivery) = self.slots.take() else {
            return;
        };
        // Whatever leaves now postdates a refused write still waiting for its retry.
        self.in_flight = None;
        match delivery {
            Delivery::ToWayland(WindowsSelection::Text(text, sequence)) => {
                self.publish_windows_text(text, sequence);
            }
            Delivery::ToWayland(WindowsSelection::Unavailable(sequence)) => {
                self.mirror.invalidate_windows(sequence);
            }
            Delivery::ToWindows(text) => self.forward_wayland_text(text, false),
        }
    }

    fn publish_windows_text(&mut self, text: HashedText, sequence: u32) {
        if let Err(error) = validate_utf8(&text.text) {
            self.mirror.invalidate_windows(sequence);
            self.reject(&error.to_string());
            return;
        }
        let wayland_echo = self.mirror.is_wayland_echo(&text.hash);
        self.mirror.observe_windows(text.hash, sequence);
        if wayland_echo {
            return;
        }
        // The compositor serves the selection back through the ordinary offer
        // path, whose text hash commits it without relying on event order.
        if let Err(error) = self.wayland.publish_text(text.text, &self.qh) {
            self.degrade(&error);
        }
    }

    fn forward_wayland_text(&mut self, text: HashedText, retried: bool) {
        if self.mirror.is_windows_echo(&text.hash) {
            return;
        }
        let pending = self.mirror.begin_windows_write(text.hash);
        let mut frame = Frame::new(MessageKind::SetWindowsText);
        frame.request_id = pending.request_id;
        frame.payload = text.text;
        let sent = self.send_frame(&frame);
        let text = HashedText {
            text: frame.payload,
            hash: text.hash,
        };
        if sent {
            self.in_flight = Some(WindowsWrite { text, retried });
        } else {
            self.slots.requeue_windows_write(text);
        }
    }

    // Win32 refuses the clipboard while another process holds it open, which is
    // routine rather than a transport failure: try once more before degrading,
    // and do not even count a refusal against a text that is obsolete already.
    fn windows_write_failed(&mut self, error: &str) {
        let Some(mut write) = self.in_flight.take() else {
            return;
        };
        match write_failure(write.retried, &self.slots) {
            WriteFailure::Retry => {
                write.retried = true;
                self.in_flight = Some(write);
                self.reject(&format!("{error}; retrying once"));
                self.schedule_retry();
            }
            WriteFailure::Superseded => self.reject(error),
            WriteFailure::Exhausted => self.degrade(error),
        }
    }

    fn schedule_retry(&mut self) {
        if self.retry_scheduled {
            return;
        }
        let retry = self.handle.insert_source(
            Timer::from_duration(WINDOWS_WRITE_RETRY_DELAY),
            |_, _, state| {
                state.retry_scheduled = false;
                state.retry_windows_write();
                TimeoutAction::Drop
            },
        );
        match retry {
            Ok(_) => self.retry_scheduled = true,
            Err(error) => self.degrade(&format!("cannot schedule clipboard write retry: {error}")),
        }
    }

    fn retry_windows_write(&mut self) {
        // A write in flight now is a newer one; the refused text is gone already.
        if self.mirror.has_pending_windows_write() {
            return;
        }
        if let Some(write) = self.in_flight.take()
            && self.agent_ready
            && self.slots.is_empty()
        {
            self.forward_wayland_text(write.text, true);
        }
        self.drain();
    }

    // Health

    // `main` reports the error once the loop has ended; only the status file
    // needs it here.
    fn fail(&mut self, error: BrokerError) {
        self.write_status(Health::Degraded, Some(&error.to_string()));
        self.fatal = Some(error);
        self.stop.stop();
    }

    // A refused selection is logged but is not a transport failure.
    fn reject(&self, error: &str) {
        eprintln!("clipboard: {error}");
    }

    fn degrade(&mut self, error: &str) {
        eprintln!("clipboard: {error}");
        self.write_status(Health::Degraded, Some(error));
    }

    fn write_status(&mut self, health: Health, error: Option<&str>) {
        if let Err(status_error) = self.status.write(health, self.agent_pid, error) {
            eprintln!("clipboard: failed to write status: {status_error}");
        }
    }
}

/// Decides whether a Sway selection is text worth forwarding.
///
/// An empty read is what a crashed source, a source that closed its pipe without
/// writing, or a refused transfer looks like; forwarding it would wipe the
/// Windows clipboard, so it counts as "no text" rather than as text.
fn accept_wayland_text(text: Vec<u8>) -> Result<HashedText, Option<String>> {
    if text.is_empty() {
        return Err(None);
    }
    validate_utf8(&text).map_err(|error| Some(error.to_string()))?;
    Ok(HashedText::new(text))
}

fn startup_windows_selection(
    ever_synced: bool,
    windows_changed: bool,
    read_error: bool,
    text: Option<HashedText>,
    sequence: u32,
) -> Option<WindowsSelection> {
    let has_text = text.is_some();
    let windows_has_priority = (!ever_synced && has_text) || (ever_synced && windows_changed);
    if windows_has_priority {
        Some(match text {
            Some(text) => WindowsSelection::Text(text, sequence),
            None => WindowsSelection::Unavailable(sequence),
        })
    } else if read_error {
        Some(WindowsSelection::Unavailable(sequence))
    } else {
        None
    }
}

fn write_failure(retried: bool, slots: &SyncSlots) -> WriteFailure {
    if !slots.is_empty() {
        WriteFailure::Superseded
    } else if retried {
        WriteFailure::Exhausted
    } else {
        WriteFailure::Retry
    }
}

fn sequence_is_newer(candidate: u32, current: u32) -> bool {
    candidate != current && candidate.wrapping_sub(current) < (1 << 31)
}

#[cfg(test)]
mod tests {
    use clipboard_core::state::MirrorState;

    use super::{
        Delivery, HashedText, SyncSlots, WaylandTextDisposition, WindowsSelection, WriteFailure,
        accept_wayland_text, observe_wayland_text, sequence_is_newer, startup_windows_selection,
        write_failure,
    };

    fn text(bytes: &[u8]) -> HashedText {
        HashedText::new(bytes.to_vec())
    }

    fn windows_text(bytes: &[u8], sequence: u32) -> WindowsSelection {
        WindowsSelection::Text(text(bytes), sequence)
    }

    #[test]
    fn clipboard_sequence_comparison_handles_wraparound() {
        assert!(sequence_is_newer(11, 10));
        assert!(!sequence_is_newer(10, 10));
        assert!(!sequence_is_newer(10, 11));
        assert!(sequence_is_newer(0, u32::MAX));
    }

    #[test]
    fn empty_or_invalid_sway_selection_is_not_text() {
        assert_eq!(accept_wayland_text(Vec::new()), Err(None));
        assert!(matches!(
            accept_wayland_text(vec![0xff, 0xfe]),
            Err(Some(_))
        ));
        assert_eq!(accept_wayland_text(b"ok".to_vec()), Ok(text(b"ok")));
    }

    #[test]
    fn changed_non_text_windows_selection_blocks_offline_wayland_text() {
        assert_eq!(
            startup_windows_selection(true, true, false, None, 12),
            Some(WindowsSelection::Unavailable(12))
        );
    }

    #[test]
    fn initial_missing_text_still_allows_wayland_startup_selection() {
        assert_eq!(
            startup_windows_selection(false, true, false, None, 12),
            None
        );
    }

    #[test]
    fn initial_windows_text_has_priority() {
        assert_eq!(
            startup_windows_selection(false, false, false, Some(text(b"win")), 3),
            Some(windows_text(b"win", 3))
        );
    }

    #[test]
    fn unreadable_windows_selection_is_conservatively_unavailable() {
        assert_eq!(
            startup_windows_selection(false, false, true, None, 12),
            Some(WindowsSelection::Unavailable(12))
        );
    }

    #[test]
    fn windows_change_during_startup_replaces_hello_text_and_wins() {
        let mut slots = SyncSlots::default();
        slots.wayland_text(text(b"sway"));
        slots.windows_selection(windows_text(b"hello", 5));
        slots.windows_selection(windows_text(b"newer", 6));
        assert_eq!(
            slots.take(),
            Some(Delivery::ToWayland(windows_text(b"newer", 6)))
        );
        assert_eq!(slots.take(), None);
    }

    #[test]
    fn sway_text_survives_agent_loss_until_windows_claims_priority() {
        let mut slots = SyncSlots::default();
        slots.windows_selection(windows_text(b"stale", 3));
        slots.wayland_text(text(b"copied while agent was down"));
        slots.agent_lost();
        assert_eq!(
            slots.take(),
            Some(Delivery::ToWindows(text(b"copied while agent was down")))
        );
    }

    #[test]
    fn interrupted_write_is_requeued_behind_newer_sway_text() {
        let mut slots = SyncSlots::default();
        slots.requeue_windows_write(text(b"in flight"));
        assert_eq!(slots.take(), Some(Delivery::ToWindows(text(b"in flight"))));

        slots.wayland_text(text(b"newer"));
        slots.requeue_windows_write(text(b"in flight"));
        assert_eq!(slots.take(), Some(Delivery::ToWindows(text(b"newer"))));
    }

    #[test]
    fn failed_write_retries_once_unless_superseded() {
        let mut slots = SyncSlots::default();
        assert_eq!(write_failure(false, &slots), WriteFailure::Retry);
        assert_eq!(write_failure(true, &slots), WriteFailure::Exhausted);

        slots.wayland_text(text(b"newer"));
        assert_eq!(write_failure(false, &slots), WriteFailure::Superseded);
        assert_eq!(write_failure(true, &slots), WriteFailure::Superseded);

        let mut slots = SyncSlots::default();
        slots.windows_selection(windows_text(b"windows moved on", 4));
        assert_eq!(write_failure(false, &slots), WriteFailure::Superseded);
    }

    #[test]
    fn windows_selection_older_than_committed_write_is_dropped() {
        let mut slots = SyncSlots::default();
        slots.windows_selection(windows_text(b"before our write", 9));
        slots.wayland_text(text(b"queued"));
        slots.windows_write_committed(10);
        assert_eq!(slots.take(), Some(Delivery::ToWindows(text(b"queued"))));

        slots.windows_selection(windows_text(b"after our write", 11));
        slots.windows_write_committed(10);
        assert_eq!(
            slots.take(),
            Some(Delivery::ToWayland(windows_text(b"after our write", 11)))
        );
    }

    #[test]
    fn non_text_sway_selection_withdraws_queued_text() {
        let mut slots = SyncSlots::default();
        slots.wayland_text(text(b"old"));
        slots.wayland_non_text();
        assert_eq!(slots.take(), None);
    }

    #[test]
    fn wayland_offers_are_classified_by_hash_not_arrival_order() {
        let mut mirror = MirrorState::default();
        let mut slots = SyncSlots::default();
        let windows = text(b"published from Windows");
        mirror.observe_windows(windows.hash, 7);

        assert_eq!(
            observe_wayland_text(&mut mirror, &mut slots, text(b"external Sway text")),
            WaylandTextDisposition::Forwarded
        );
        // The later announcement supersedes the earlier queued selection before
        // its hash is classified.
        slots.wayland_non_text();
        mirror.invalidate_wayland();
        assert_eq!(
            observe_wayland_text(&mut mirror, &mut slots, windows),
            WaylandTextDisposition::WindowsEcho
        );
        assert_eq!(slots.take(), None);
    }
}
