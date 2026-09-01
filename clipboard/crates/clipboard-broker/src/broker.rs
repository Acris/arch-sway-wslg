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
use wayland_client::protocol::wl_callback;
use wayland_client::{Connection, Dispatch, QueueHandle};

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
    connection: Connection,
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
    initial_sync: bool,
    ever_synced: bool,
    slots: SyncSlots,
    // The Sway text the agent is writing right now, kept until its ACK so that a
    // refused or interrupted write can be queued again.
    in_flight: Option<WindowsWrite>,
    retry_scheduled: bool,
    fatal: Option<String>,
    stop: LoopSignal,
}

#[derive(Clone, Debug)]
pub(crate) struct PublishAck {
    hash: TextHash,
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
    // A Windows write that failed once; any newer selection on either side
    // makes it obsolete.
    retry_to_windows: Option<HashedText>,
}

#[derive(Debug, Eq, PartialEq)]
enum Delivery {
    ToWindows { text: HashedText, retry: bool },
    ToWayland(WindowsSelection),
}

impl SyncSlots {
    fn wayland_text(&mut self, text: HashedText) {
        self.to_windows = Some(text);
        self.retry_to_windows = None;
    }

    fn wayland_non_text(&mut self) {
        self.to_windows = None;
        self.retry_to_windows = None;
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

    fn retry_windows_write(&mut self, text: HashedText) -> bool {
        if self.to_windows.is_some() {
            return false;
        }
        self.retry_to_windows = Some(text);
        true
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

    fn take(&mut self, allow_retry: bool) -> Option<Delivery> {
        if let Some(selection) = self.to_wayland.take() {
            self.to_windows = None;
            self.retry_to_windows = None;
            return Some(Delivery::ToWayland(selection));
        }
        if let Some(text) = self.to_windows.take() {
            self.retry_to_windows = None;
            return Some(Delivery::ToWindows { text, retry: false });
        }
        if !allow_retry {
            return None;
        }
        self.retry_to_windows
            .take()
            .map(|text| Delivery::ToWindows { text, retry: true })
    }
}

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("Wayland connection failed: {0}")]
    Wayland(String),
    #[error("event loop failed: {0}")]
    EventLoop(String),
    #[error(transparent)]
    Io(#[from] io::Error),
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
        connection: connection.clone(),
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
        initial_sync: false,
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
    if let Some(error) = state.fatal.take() {
        return Err(BrokerError::EventLoop(error));
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
            }
            WaylandEvent::Text { generation, text } => {
                if generation != self.latest_wayland_generation {
                    return;
                }
                self.wayland_ready = true;
                match accept_wayland_text(text) {
                    Ok(text) => {
                        // An echo forwards nothing but still makes Sway ready.
                        if !self.mirror.is_wayland_echo(&text.hash) {
                            self.mirror.observe_wayland(text.hash);
                            self.slots.wayland_text(text);
                        }
                    }
                    Err(error) => self.withdraw_wayland_selection(error.as_deref()),
                }
                self.drain();
            }
            WaylandEvent::Unusable { generation, error } => {
                if generation == self.latest_wayland_generation {
                    self.wayland_ready = true;
                    self.withdraw_wayland_selection(error.as_deref());
                    self.drain();
                }
            }
            WaylandEvent::DeviceLost(error) => self.fail(error),
        }
    }

    fn withdraw_wayland_selection(&mut self, error: Option<&str>) {
        self.mirror.reject_wayland_selection();
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
        self.initial_sync = false;
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
            self.fail(format!(
                "Windows clipboard agent restart budget exhausted: {reason}"
            ));
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
            self.fail(format!(
                "cannot schedule Windows clipboard agent restart: {error}"
            ));
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
        self.drain();
    }

    // Delivery

    fn drain(&mut self) {
        if !self.wayland_ready || !self.agent_ready {
            return;
        }
        if !self.initial_sync {
            self.initial_sync = true;
            self.ever_synced = true;
            self.write_status(Health::Running, None);
        }
        if self.mirror.has_pending_windows_write() {
            return;
        }
        match self.slots.take(!self.retry_scheduled) {
            Some(Delivery::ToWayland(WindowsSelection::Text(text, sequence))) => {
                self.publish_windows_text(text, sequence);
            }
            Some(Delivery::ToWayland(WindowsSelection::Unavailable(sequence))) => {
                self.mirror.invalidate_windows(sequence);
            }
            Some(Delivery::ToWindows { text, retry }) => self.forward_wayland_text(text, retry),
            None => {}
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
        match self.wayland.publish_text(text.text, &self.qh) {
            Ok(()) => {
                self.mirror.begin_wayland_publish(text.hash, sequence);
                self.connection
                    .display()
                    .sync(&self.qh, PublishAck { hash: text.hash });
            }
            Err(error) => self.degrade(&error),
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
    // routine rather than a transport failure: try once more before degrading.
    fn windows_write_failed(&mut self, error: &str) {
        if let Some(write) = self.in_flight.take()
            && !write.retried
            && self.slots.retry_windows_write(write.text)
        {
            self.reject(&format!("{error}; retrying once"));
            self.schedule_retry();
        } else {
            self.degrade(error);
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
                state.drain();
                TimeoutAction::Drop
            },
        );
        match retry {
            Ok(_) => self.retry_scheduled = true,
            Err(error) => self.degrade(&format!("cannot schedule clipboard write retry: {error}")),
        }
    }

    // Health

    fn fail(&mut self, error: String) {
        self.degrade(&error);
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

fn sequence_is_newer(candidate: u32, current: u32) -> bool {
    candidate != current && candidate.wrapping_sub(current) < (1 << 31)
}

impl Dispatch<wl_callback::WlCallback, PublishAck> for BrokerState {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        ack: &PublishAck,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, wl_callback::Event::Done { .. })
            && state.mirror.commit_wayland_publish(&ack.hash)
        {
            state.write_status(Health::Running, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Delivery, HashedText, SyncSlots, WindowsSelection, accept_wayland_text, sequence_is_newer,
        startup_windows_selection,
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
            slots.take(true),
            Some(Delivery::ToWayland(windows_text(b"newer", 6)))
        );
        assert_eq!(slots.take(true), None);
    }

    #[test]
    fn sway_text_survives_agent_loss_until_windows_claims_priority() {
        let mut slots = SyncSlots::default();
        slots.windows_selection(windows_text(b"stale", 3));
        slots.wayland_text(text(b"copied while agent was down"));
        slots.agent_lost();
        assert_eq!(
            slots.take(true),
            Some(Delivery::ToWindows {
                text: text(b"copied while agent was down"),
                retry: false
            })
        );
    }

    #[test]
    fn interrupted_write_is_requeued_behind_newer_sway_text() {
        let mut slots = SyncSlots::default();
        slots.requeue_windows_write(text(b"in flight"));
        assert_eq!(
            slots.take(true),
            Some(Delivery::ToWindows {
                text: text(b"in flight"),
                retry: false
            })
        );

        slots.wayland_text(text(b"newer"));
        slots.requeue_windows_write(text(b"in flight"));
        assert_eq!(
            slots.take(true),
            Some(Delivery::ToWindows {
                text: text(b"newer"),
                retry: false
            })
        );
    }

    #[test]
    fn failed_write_retries_once_unless_superseded() {
        let mut slots = SyncSlots::default();
        assert!(slots.retry_windows_write(text(b"refused")));
        assert_eq!(slots.take(false), None);
        assert_eq!(
            slots.take(true),
            Some(Delivery::ToWindows {
                text: text(b"refused"),
                retry: true
            })
        );

        assert!(slots.retry_windows_write(text(b"refused")));
        slots.wayland_text(text(b"newer"));
        assert_eq!(
            slots.take(true),
            Some(Delivery::ToWindows {
                text: text(b"newer"),
                retry: false
            })
        );
        assert_eq!(slots.take(true), None);

        slots.wayland_text(text(b"queued"));
        assert!(!slots.retry_windows_write(text(b"refused")));

        let mut slots = SyncSlots::default();
        assert!(slots.retry_windows_write(text(b"refused")));
        slots.windows_selection(windows_text(b"windows moved on", 4));
        assert_eq!(
            slots.take(true),
            Some(Delivery::ToWayland(windows_text(b"windows moved on", 4)))
        );
        assert_eq!(slots.take(true), None);
    }

    #[test]
    fn windows_selection_older_than_committed_write_is_dropped() {
        let mut slots = SyncSlots::default();
        slots.windows_selection(windows_text(b"before our write", 9));
        slots.wayland_text(text(b"queued"));
        slots.windows_write_committed(10);
        assert_eq!(
            slots.take(true),
            Some(Delivery::ToWindows {
                text: text(b"queued"),
                retry: false
            })
        );

        slots.windows_selection(windows_text(b"after our write", 11));
        slots.windows_write_committed(10);
        assert_eq!(
            slots.take(true),
            Some(Delivery::ToWayland(windows_text(b"after our write", 11)))
        );
    }

    #[test]
    fn non_text_sway_selection_withdraws_queued_text() {
        let mut slots = SyncSlots::default();
        slots.wayland_text(text(b"old"));
        slots.wayland_non_text();
        assert_eq!(slots.take(true), None);
    }
}
