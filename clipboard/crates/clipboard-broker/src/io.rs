//! Bounded, non-blocking pipe helpers shared by the agent pipe and the Wayland
//! transfers. Every call here returns within the deadline it was given.

use std::io::{self, Read, Write};
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};

const CHUNK: usize = 8192;

pub fn set_nonblocking(fd: impl AsFd) -> io::Result<()> {
    let flags = fcntl_getfl(&fd)?;
    fcntl_setfl(&fd, flags | OFlags::NONBLOCK)?;
    Ok(())
}

/// Writes all of `input`, giving up once `timeout` has passed.
pub fn write_with_deadline(
    mut writer: impl AsFd + Write,
    mut input: &[u8],
    timeout: Duration,
) -> io::Result<()> {
    set_nonblocking(&writer)?;
    let deadline = Instant::now() + timeout;
    while !input.is_empty() {
        wait_for_fd(&writer, PollFlags::OUT, deadline)?;
        let count = match writer.write(input) {
            Ok(count) => count,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                continue;
            }
            Err(error) => return Err(error),
        };
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "pipe reader stopped accepting data",
            ));
        }
        input = &input[count..];
    }
    Ok(())
}

/// Moves everything a non-blocking pipe currently holds into `output`.
///
/// Returns `Ok(true)` once the writer has closed the pipe. Exceeding `limit` is
/// an `InvalidData` error so the caller never buffers more than one selection.
pub fn read_available(
    mut reader: impl Read,
    output: &mut Vec<u8>,
    limit: usize,
) -> io::Result<bool> {
    let mut buffer = [0_u8; CHUNK];
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(0) => return Ok(true),
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if output.len() + count > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("text exceeded the {limit} byte limit"),
            ));
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

fn wait_for_fd(fd: impl AsFd, flags: PollFlags, deadline: Instant) -> io::Result<()> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "pipe transfer timed out",
        ));
    }
    let timeout = Timespec::try_from(remaining).map_err(io::Error::other)?;
    let mut descriptors = [PollFd::new(&fd, flags)];
    if poll(&mut descriptors, Some(&timeout))? == 0 {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "pipe transfer timed out",
        ));
    }
    // HUP is left to the read or write itself: a closed read end still holds
    // data, and a closed write end surfaces as EPIPE.
    if descriptors[0]
        .revents()
        .intersects(PollFlags::ERR | PollFlags::NVAL)
    {
        return Err(io::Error::other("pipe reported an error"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::thread;

    use rustix::pipe::{PipeFlags, pipe_with};

    use super::*;

    fn pipe() -> (File, File) {
        let (reader, writer) = pipe_with(PipeFlags::CLOEXEC).unwrap();
        (File::from(reader), File::from(writer))
    }

    #[test]
    fn bounded_write_times_out_when_reader_stalls() {
        let (_reader, writer) = pipe();
        let input = vec![0_u8; 1024 * 1024];
        let error = write_with_deadline(&writer, &input, Duration::from_millis(10)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    }

    #[test]
    fn bounded_write_delivers_to_a_live_reader() {
        let (reader, writer) = pipe();
        let input = vec![7_u8; 256 * 1024];
        let expected = input.clone();
        let consumer = thread::spawn(move || {
            let mut output = Vec::new();
            set_nonblocking(&reader).unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            while !read_available(&reader, &mut output, usize::MAX).unwrap() {
                assert!(Instant::now() < deadline, "reader starved");
                wait_for_fd(&reader, PollFlags::IN, deadline).unwrap();
            }
            output
        });
        write_with_deadline(&writer, &input, Duration::from_secs(1)).unwrap();
        drop(writer);
        assert_eq!(consumer.join().unwrap(), expected);
    }

    #[test]
    fn read_available_reports_eof_and_pending_separately() {
        let (reader, writer) = pipe();
        set_nonblocking(&reader).unwrap();
        (&writer).write_all(b"hello").unwrap();
        let mut output = Vec::new();
        assert!(!read_available(&reader, &mut output, 1024).unwrap());
        assert_eq!(output, b"hello");
        drop(writer);
        assert!(read_available(&reader, &mut output, 1024).unwrap());
        assert_eq!(output, b"hello");
    }

    #[test]
    fn read_available_rejects_oversized_text() {
        let (reader, writer) = pipe();
        set_nonblocking(&reader).unwrap();
        (&writer).write_all(b"too large").unwrap();
        let mut output = Vec::new();
        let error = read_available(&reader, &mut output, 3).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
