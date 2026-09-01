use std::ffi::c_void;
use std::io::{self, BufReader, BufWriter};
use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::Duration;

use clipboard_core::protocol::{Frame, HELLO_HAS_TEXT, HELLO_READ_ERROR, MessageKind};
use clipboard_core::text::{utf8_to_utf16, utf16_to_utf8, validate_utf8};
use clipboard_core::{MAX_TEXT_BYTES, PROTOCOL_VERSION};
use thiserror::Error;
use windows_sys::Win32::Foundation::{
    GetLastError, GlobalFree, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows_sys::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
    RemoveClipboardFormatListener, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_MESSAGE, MSG,
    PostMessageW, RegisterClassW, TranslateMessage, WM_APP, WM_CLIPBOARDUPDATE, WNDCLASSW,
};

const CF_UNICODETEXT: u32 = 13;
const WM_AGENT_COMMAND: u32 = WM_APP + 1;
// The broker closing its end of the pipe is the only shutdown signal.
const WM_AGENT_QUIT: u32 = WM_APP + 2;
const CLIPBOARD_RETRIES: usize = 8;
const MAX_WINDOWS_TEXT_BYTES: usize = (MAX_TEXT_BYTES + 1) * size_of::<u16>();

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
            "arch-sway-wslg-clipboard-agent protocol={} arch=x86_64",
            PROTOCOL_VERSION
        );
        return Ok(());
    }
    let write_only = matches!(argument.as_deref(), Some(value) if value == "--write-only");

    let (sender, receiver) = mpsc::sync_channel(8);
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
        match clipboard_snapshot(window) {
            Ok((sequence, text)) => (sequence, text, false),
            Err(_) => (unsafe { GetClipboardSequenceNumber() }, None, true),
        }
    };
    let mut hello = Frame::new(MessageKind::Hello);
    hello.request_id = u64::from(unsafe { GetCurrentProcessId() });
    hello.sequence = initial_sequence;
    if let Some(text) = initial_text {
        hello.flags |= HELLO_HAS_TEXT;
        hello.payload = text;
    }
    if initial_read_error {
        hello.flags |= HELLO_READ_ERROR;
    }
    hello.write_to(&mut writer)?;

    let result = message_loop(window, receiver, &mut writer, initial_sequence);
    if !write_only {
        unsafe {
            RemoveClipboardFormatListener(window);
        }
    }
    result
}

fn read_commands(window: HWND, sender: SyncSender<Frame>) {
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
                // Unsupported or temporarily unavailable clipboard data must not take
                // down the listener. Advance the sequence and let the broker treat this
                // selection as unavailable.
                let (sequence, text) = match clipboard_snapshot(window) {
                    Ok(snapshot) => snapshot,
                    Err(_) => (unsafe { GetClipboardSequenceNumber() }, None),
                };
                last_sequence = sequence;
                let mut frame = Frame::new(match text {
                    Some(_) => MessageKind::WindowsText,
                    None => MessageKind::WindowsUnavailable,
                });
                frame.sequence = sequence;
                frame.payload = text.unwrap_or_default();
                frame.write_to(writer)?;
            }
            WM_AGENT_COMMAND => {
                while let Ok(frame) = receiver.try_recv() {
                    match frame.kind {
                        MessageKind::SetWindowsText => {
                            let mut response = match write_clipboard_text(window, &frame.payload) {
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

/// Reads the clipboard text together with the sequence number it belongs to; a
/// change between the two reads would otherwise pair a number with newer text.
fn clipboard_snapshot(window: HWND) -> Result<(u32, Option<Vec<u8>>), AgentError> {
    for _ in 0..CLIPBOARD_RETRIES {
        let before = unsafe { GetClipboardSequenceNumber() };
        let text = read_clipboard_text(window)?;
        let after = unsafe { GetClipboardSequenceNumber() };
        if before == after {
            return Ok((after, text));
        }
    }
    Err(AgentError::MalformedClipboard)
}

fn read_clipboard_text(window: HWND) -> Result<Option<Vec<u8>>, AgentError> {
    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
            return Ok(None);
        }
    }
    with_open_clipboard(window, || unsafe {
        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.is_null() {
            return Err(last_error("GetClipboardData"));
        }
        let size = GlobalSize(handle as HANDLE);
        if size < size_of::<u16>() || size > MAX_WINDOWS_TEXT_BYTES || size % size_of::<u16>() != 0
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
    })
}

fn write_clipboard_text(window: HWND, bytes: &[u8]) -> Result<u32, AgentError> {
    validate_utf8(bytes)?;
    let wide = utf8_to_utf16(bytes)?;
    let byte_len = wide.len() * size_of::<u16>();
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, byte_len) };
    if handle.is_null() {
        return Err(last_error("GlobalAlloc"));
    }
    let pointer = unsafe { GlobalLock(handle) as *mut u16 };
    if pointer.is_null() {
        unsafe { GlobalFree(handle) };
        return Err(last_error("GlobalLock"));
    }
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), pointer, wide.len());
        GlobalUnlock(handle);
    }

    let result = with_open_clipboard(window, || unsafe {
        if EmptyClipboard() == 0 {
            return Err(last_error("EmptyClipboard"));
        }
        if SetClipboardData(CF_UNICODETEXT, handle as HANDLE).is_null() {
            return Err(last_error("SetClipboardData"));
        }
        Ok(GetClipboardSequenceNumber())
    });
    if result.is_err() {
        unsafe { GlobalFree(handle) };
    }
    result
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
