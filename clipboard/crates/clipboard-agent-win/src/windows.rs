use std::ffi::c_void;
use std::io::{self, BufReader, BufWriter};
use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use clipboard_core::protocol::{
    Frame, HELLO_HAS_TEXT, HELLO_READ_ERROR, MessageKind, TEXT_SENSITIVE,
};
use clipboard_core::text::{utf8_to_utf16, utf16_to_utf8, validate_utf8};
use clipboard_core::{MAX_TEXT_BYTES, PROTOCOL_VERSION};
use thiserror::Error;
use windows_sys::Win32::Foundation::{
    GetLastError, GlobalFree, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows_sys::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, RemoveClipboardFormatListener, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_MESSAGE, KillTimer, MSG,
    PostMessageW, RegisterClassW, SetTimer, TranslateMessage, WM_APP, WM_CLIPBOARDUPDATE, WM_TIMER,
    WNDCLASSW,
};

const CF_UNICODETEXT: u32 = 13;
const WM_AGENT_COMMAND: u32 = WM_APP + 1;
// The broker closing its end of the pipe is the only shutdown signal.
const WM_AGENT_QUIT: u32 = WM_APP + 2;
const CLIPBOARD_RETRIES: usize = 8;
// Another process can hold the clipboard open for longer than the retries above
// cover; the update is read once more after this pause before it is given up on.
const READ_RETRY_TIMER: usize = 1;
const READ_RETRY_DELAY_MS: u32 = 250;
const MAX_WINDOWS_TEXT_BYTES: usize = (MAX_TEXT_BYTES + 1) * size_of::<u16>();
// Source digest embedded by build.rs; `--probe` prints it so that a payload can
// be matched against the tree it was built from.
const SOURCE_DIGEST: &str = env!("ARCH_SWAY_WSLG_SOURCE_DIGEST");

/// The registered formats Windows documents for keeping a text out of clipboard
/// monitors, the clipboard history and cloud synchronization. Password managers
/// set them; nothing else about a text is taken as a sign of sensitivity.
struct HintFormats {
    exclude_from_monitors: u32,
    viewer_ignore: u32,
    history: u32,
    cloud: u32,
}

impl HintFormats {
    fn register() -> Result<Self, AgentError> {
        Ok(Self {
            exclude_from_monitors: register_format("ExcludeClipboardContentFromMonitorProcessing")?,
            viewer_ignore: register_format("Clipboard Viewer Ignore")?,
            history: register_format("CanIncludeInClipboardHistory")?,
            cloud: register_format("CanUploadToCloudClipboard")?,
        })
    }
}

/// Clipboard text in UTF-8, and whether Windows marked it as sensitive.
struct ClipboardText {
    text: Vec<u8>,
    sensitive: bool,
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Win32 call failed: {operation} ({code})")]
    Win32 { operation: &'static str, code: u32 },
    #[error("clipboard data was malformed")]
    MalformedClipboard,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Protocol(#[from] clipboard_core::protocol::ProtocolError),
    #[error(transparent)]
    Text(#[from] clipboard_core::text::TextError),
}

pub fn run() -> Result<(), AgentError> {
    let mut args = std::env::args_os();
    let _program = args.next();
    let argument = args.next();
    if matches!(argument.as_deref(), Some(value) if value == "--probe") {
        println!(
            "arch-sway-wslg-clipboard-agent protocol={PROTOCOL_VERSION} source={SOURCE_DIGEST} arch=x86_64"
        );
        return Ok(());
    }
    let write_only = matches!(argument.as_deref(), Some(value) if value == "--write-only");

    let formats = HintFormats::register()?;
    let (sender, receiver) = mpsc::channel();
    let window = create_message_window()?;
    let reader_window = window as usize;
    thread::spawn(move || read_commands(reader_window as HWND, sender));

    if !write_only {
        unsafe {
            if AddClipboardFormatListener(window) == 0 {
                return Err(last_error("AddClipboardFormatListener"));
            }
        }
    }

    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    let (initial_sequence, initial_text, initial_read_error) = if write_only {
        (unsafe { GetClipboardSequenceNumber() }, None, false)
    } else {
        match clipboard_snapshot(window, &formats) {
            Ok((sequence, text)) => (sequence, text, false),
            Err(_) => (unsafe { GetClipboardSequenceNumber() }, None, true),
        }
    };
    let mut hello = Frame::new(MessageKind::Hello);
    hello.request_id = u64::from(unsafe { GetCurrentProcessId() });
    hello.sequence = initial_sequence;
    if let Some(text) = initial_text {
        hello.flags |= HELLO_HAS_TEXT;
        if text.sensitive {
            hello.flags |= TEXT_SENSITIVE;
        }
        hello.payload = text.text;
    }
    if initial_read_error {
        hello.flags |= HELLO_READ_ERROR;
    }
    hello.write_to(&mut writer)?;

    let result = message_loop(window, &formats, receiver, &mut writer, initial_sequence);
    if !write_only {
        unsafe {
            RemoveClipboardFormatListener(window);
        }
    }
    result
}

fn read_commands(window: HWND, sender: Sender<Frame>) {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    loop {
        let frame = match Frame::read_from(&mut reader) {
            Ok(frame) => frame,
            Err(_) => {
                unsafe { PostMessageW(window, WM_AGENT_QUIT, 0, 0) };
                return;
            }
        };
        if sender.send(frame).is_err() {
            return;
        }
        unsafe { PostMessageW(window, WM_AGENT_COMMAND, 0, 0) };
    }
}

fn message_loop(
    window: HWND,
    formats: &HintFormats,
    receiver: Receiver<Frame>,
    writer: &mut BufWriter<impl io::Write>,
    mut last_sequence: u32,
) -> Result<(), AgentError> {
    let mut message = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result <= 0 {
            return if result == 0 {
                Ok(())
            } else {
                Err(last_error("GetMessageW"))
            };
        }

        match message.message {
            WM_CLIPBOARDUPDATE => {
                if unsafe { GetClipboardSequenceNumber() } == last_sequence {
                    continue;
                }
                match clipboard_snapshot(window, formats) {
                    Ok((sequence, text)) => {
                        unsafe { KillTimer(window, READ_RETRY_TIMER) };
                        last_sequence = report_selection(writer, sequence, text)?;
                    }
                    // Whoever holds the clipboard usually lets go within moments;
                    // reporting the text unavailable now would lose it for good.
                    Err(_) => unsafe {
                        SetTimer(window, READ_RETRY_TIMER, READ_RETRY_DELAY_MS, None);
                    },
                }
            }
            WM_TIMER if message.wParam == READ_RETRY_TIMER => {
                unsafe { KillTimer(window, READ_RETRY_TIMER) };
                if unsafe { GetClipboardSequenceNumber() } == last_sequence {
                    continue;
                }
                // Unsupported or still unavailable clipboard data must not take down
                // the listener. Advance the sequence and let the broker treat this
                // selection as unavailable.
                let (sequence, text) = match clipboard_snapshot(window, formats) {
                    Ok(snapshot) => snapshot,
                    Err(_) => (unsafe { GetClipboardSequenceNumber() }, None),
                };
                last_sequence = report_selection(writer, sequence, text)?;
            }
            WM_AGENT_COMMAND => {
                while let Ok(frame) = receiver.try_recv() {
                    match frame.kind {
                        MessageKind::SetWindowsText => {
                            let sensitive = frame.flags & TEXT_SENSITIVE != 0;
                            let mut response = match write_clipboard_text(
                                window,
                                formats,
                                &frame.payload,
                                sensitive,
                            ) {
                                Ok(sequence) => {
                                    last_sequence = sequence;
                                    let mut response = Frame::new(MessageKind::SetWindowsOk);
                                    response.sequence = sequence;
                                    response
                                }
                                Err(error) => {
                                    let mut response = Frame::new(MessageKind::SetWindowsError);
                                    response.payload = error.to_string().into_bytes();
                                    response
                                }
                            };
                            response.request_id = frame.request_id;
                            response.write_to(writer)?;
                        }
                        MessageKind::Ping => {
                            let mut pong = Frame::new(MessageKind::Pong);
                            pong.request_id = frame.request_id;
                            pong.write_to(writer)?;
                        }
                        _ => {
                            let mut error = Frame::new(MessageKind::ProtocolError);
                            error.request_id = frame.request_id;
                            error.payload = b"unexpected command".to_vec();
                            error.write_to(writer)?;
                        }
                    }
                }
            }
            WM_AGENT_QUIT => return Ok(()),
            _ => unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            },
        }
    }
}

/// Tells the broker what the clipboard holds and returns the sequence number the
/// report describes.
fn report_selection(
    writer: &mut BufWriter<impl io::Write>,
    sequence: u32,
    text: Option<ClipboardText>,
) -> Result<u32, AgentError> {
    let mut frame = Frame::new(match text {
        Some(_) => MessageKind::WindowsText,
        None => MessageKind::WindowsUnavailable,
    });
    frame.sequence = sequence;
    if let Some(text) = text {
        if text.sensitive {
            frame.flags |= TEXT_SENSITIVE;
        }
        frame.payload = text.text;
    }
    frame.write_to(writer)?;
    Ok(sequence)
}

/// Reads the clipboard text together with the sequence number it belongs to; a
/// change between the two reads would otherwise pair a number with newer text.
fn clipboard_snapshot(
    window: HWND,
    formats: &HintFormats,
) -> Result<(u32, Option<ClipboardText>), AgentError> {
    for _ in 0..CLIPBOARD_RETRIES {
        let before = unsafe { GetClipboardSequenceNumber() };
        let text = read_clipboard_text(window, formats)?;
        let after = unsafe { GetClipboardSequenceNumber() };
        if before == after {
            return Ok((after, text));
        }
    }
    Err(AgentError::MalformedClipboard)
}

fn read_clipboard_text(
    window: HWND,
    formats: &HintFormats,
) -> Result<Option<ClipboardText>, AgentError> {
    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
            return Ok(None);
        }
    }
    let text = with_open_clipboard(window, || {
        let text = read_open_clipboard_text()?;
        Ok(text.map(|text| ClipboardText {
            text,
            sensitive: open_clipboard_is_sensitive(formats),
        }))
    })?;
    Ok(text)
}

/// Must run with the clipboard open, so the hint belongs to the text read.
fn open_clipboard_is_sensitive(formats: &HintFormats) -> bool {
    let excluded = unsafe {
        IsClipboardFormatAvailable(formats.exclude_from_monitors) != 0
            || IsClipboardFormatAvailable(formats.viewer_ignore) != 0
    };
    excluded || open_clipboard_dword(formats.history) == Some(0)
}

fn open_clipboard_dword(format: u32) -> Option<u32> {
    unsafe {
        let handle = GetClipboardData(format);
        if handle.is_null() || GlobalSize(handle as HANDLE) < size_of::<u32>() {
            return None;
        }
        let pointer = GlobalLock(handle as HANDLE) as *const u32;
        if pointer.is_null() {
            return None;
        }
        let value = pointer.read_unaligned();
        GlobalUnlock(handle as HANDLE);
        Some(value)
    }
}

fn read_open_clipboard_text() -> Result<Option<Vec<u8>>, AgentError> {
    unsafe {
        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.is_null() {
            return Err(last_error("GetClipboardData"));
        }
        let size = GlobalSize(handle as HANDLE);
        if size < size_of::<u16>()
            || size > MAX_WINDOWS_TEXT_BYTES
            || !size.is_multiple_of(size_of::<u16>())
        {
            return Err(AgentError::MalformedClipboard);
        }
        let pointer = GlobalLock(handle as HANDLE) as *const u16;
        if pointer.is_null() {
            return Err(last_error("GlobalLock"));
        }
        let units = std::slice::from_raw_parts(pointer, size / size_of::<u16>());
        let Some(end) = units.iter().position(|unit| *unit == 0) else {
            GlobalUnlock(handle as HANDLE);
            return Err(AgentError::MalformedClipboard);
        };
        // An empty string is not a selection worth clearing the other side for.
        let result = if end == 0 {
            Ok(None)
        } else {
            utf16_to_utf8(&units[..end]).map(Some)
        };
        GlobalUnlock(handle as HANDLE);
        result.map_err(AgentError::from)
    }
}

/// Writes the text and, for a sensitive one, the hints that keep it out of the
/// clipboard history, cloud synchronization and other clipboard monitors.
fn write_clipboard_text(
    window: HWND,
    formats: &HintFormats,
    bytes: &[u8],
    sensitive: bool,
) -> Result<u32, AgentError> {
    validate_utf8(bytes)?;
    let wide = utf8_to_utf16(bytes)?;
    let mut items: Vec<(u32, HANDLE)> = Vec::with_capacity(4);
    let mut transferred = 0;
    let result = prepare_items(&mut items, formats, &wide, sensitive).and_then(|()| {
        with_open_clipboard(window, || unsafe {
            if EmptyClipboard() == 0 {
                return Err(last_error("EmptyClipboard"));
            }
            for (format, handle) in &items {
                if SetClipboardData(*format, *handle).is_null() {
                    return Err(last_error("SetClipboardData"));
                }
                // The clipboard owns the memory from here on.
                transferred += 1;
            }
            Ok(GetClipboardSequenceNumber())
        })
    });
    for (_, handle) in &items[transferred..] {
        unsafe { GlobalFree(*handle) };
    }
    result
}

fn prepare_items(
    items: &mut Vec<(u32, HANDLE)>,
    formats: &HintFormats,
    wide: &[u16],
    sensitive: bool,
) -> Result<(), AgentError> {
    items.push((CF_UNICODETEXT, global_copy(wide)?));
    if sensitive {
        // Windows reads a DWORD 0 as "no" for the two Can* formats; the monitor
        // exclusion only has to be present.
        for format in [
            formats.exclude_from_monitors,
            formats.history,
            formats.cloud,
        ] {
            items.push((format, global_copy(&[0_u32])?));
        }
    }
    Ok(())
}

fn global_copy<T: Copy>(data: &[T]) -> Result<HANDLE, AgentError> {
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, size_of_val(data)) };
    if handle.is_null() {
        return Err(last_error("GlobalAlloc"));
    }
    let pointer = unsafe { GlobalLock(handle) as *mut T };
    if pointer.is_null() {
        let error = last_error("GlobalLock");
        unsafe { GlobalFree(handle) };
        return Err(error);
    }
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), pointer, data.len());
        GlobalUnlock(handle);
    }
    Ok(handle as HANDLE)
}

fn register_format(name: &str) -> Result<u32, AgentError> {
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    match unsafe { RegisterClipboardFormatW(wide.as_ptr()) } {
        0 => Err(last_error("RegisterClipboardFormatW")),
        format => Ok(format),
    }
}

fn with_open_clipboard<T>(
    window: HWND,
    operation: impl FnOnce() -> Result<T, AgentError>,
) -> Result<T, AgentError> {
    for attempt in 0..CLIPBOARD_RETRIES {
        if unsafe { OpenClipboard(window) } != 0 {
            let result = operation();
            unsafe { CloseClipboard() };
            return result;
        }
        thread::sleep(Duration::from_millis(5_u64 << attempt.min(6)));
    }
    Err(last_error("OpenClipboard"))
}

fn create_message_window() -> Result<HWND, AgentError> {
    let class_name: Vec<u16> = "ArchSwayWslgClipboardAgent\0".encode_utf16().collect();
    unsafe {
        let instance = GetModuleHandleW(null());
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(last_error("RegisterClassW"));
        }
        let window = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            instance,
            null::<c_void>(),
        );
        if window.is_null() {
            return Err(last_error("CreateWindowExW"));
        }
        Ok(window)
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

fn last_error(operation: &'static str) -> AgentError {
    AgentError::Win32 {
        operation,
        code: unsafe { GetLastError() },
    }
}
