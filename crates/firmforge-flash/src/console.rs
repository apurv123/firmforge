//! A live serial connection to a running board.
//!
//! Flashing and talking to a board are different problems. Flashing drives the
//! ROM bootloader, finishes, and lets go. A console stays open for as long as
//! the user wants, streams whatever the board says, and sends whatever they
//! type. That needs a background reader, because a board can talk at any time
//! and a UI that only reads when asked would drop output.
//!
//! Two details here are less obvious than they look:
//!
//! * **UTF-8 is decoded incrementally.** Serial reads chop the stream at
//!   arbitrary byte boundaries, so a multi-byte character routinely straddles
//!   two reads. Decoding each read on its own would turn those into replacement
//!   characters — visible corruption in ordinary output.
//! * **DTR and RTS are held low on open.** On most ESP32 boards those lines are
//!   wired to EN and GPIO0, so asserting them resets the chip or drops it into
//!   the bootloader. Connecting a terminal should not restart the program you
//!   are trying to observe, so resetting is something the user asks for.
//!
//! Not compiled on Android or iOS, where a host serial port cannot be opened.

use crate::transport::TransportError;
use serialport::SerialPort;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The REPL baud rate for every ESP32 MicroPython build.
pub const DEFAULT_BAUD: u32 = 115_200;

/// How long a read blocks before the reader thread loops and re-checks whether
/// it has been asked to stop. Short enough that disconnecting feels immediate.
const READ_TIMEOUT: Duration = Duration::from_millis(50);

/// Shared between the reader thread and whoever is driving the connection.
struct Shared {
    /// Everything received and not yet consumed by a scripted exchange.
    buf: Mutex<Vec<u8>>,
    /// While set, output is being consumed by a scripted exchange (a raw-REPL
    /// command, a file upload) and must not also be echoed to the user. Without
    /// this the terminal would fill with protocol chatter during an upload.
    quiet: AtomicBool,
}

/// An open serial connection.
pub struct Console {
    writer: Box<dyn SerialPort>,
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
    port_name: String,
    baud: u32,
}

impl Console {
    /// Open `port_name` and start streaming. `on_text` is called from the
    /// reader thread with decoded output as it arrives.
    pub fn open<F>(port_name: &str, baud: u32, on_text: F) -> Result<Self, TransportError>
    where
        F: Fn(String) + Send + 'static,
    {
        let mut writer = serialport::new(port_name, baud)
            .timeout(READ_TIMEOUT)
            .open()
            .map_err(|e| {
                TransportError::Io(format!(
                    "could not open {port_name}: {e}. Another program may be holding it — \
                     Thonny, the Arduino IDE, or another serial monitor."
                ))
            })?;

        // Held low so that merely connecting does not reset the board.
        let _ = writer.write_data_terminal_ready(false);
        let _ = writer.write_request_to_send(false);

        let mut reader_port = writer
            .try_clone()
            .map_err(|e| TransportError::Io(format!("could not listen on {port_name}: {e}")))?;

        let shared = Arc::new(Shared {
            buf: Mutex::new(Vec::new()),
            quiet: AtomicBool::new(false),
        });
        let stop = Arc::new(AtomicBool::new(false));

        let thread_shared = Arc::clone(&shared);
        let thread_stop = Arc::clone(&stop);
        let reader = std::thread::Builder::new()
            .name(format!("firmforge-console-{port_name}"))
            .spawn(move || {
                let mut chunk = [0u8; 1024];
                let mut undecoded: Vec<u8> = Vec::new();

                while !thread_stop.load(Ordering::Relaxed) {
                    match reader_port.read(&mut chunk) {
                        Ok(0) => continue,
                        Ok(n) => {
                            let bytes = &chunk[..n];
                            if let Ok(mut buf) = thread_shared.buf.lock() {
                                buf.extend_from_slice(bytes);
                            }
                            undecoded.extend_from_slice(bytes);
                            let text = take_decodable(&mut undecoded);
                            if !text.is_empty() && !thread_shared.quiet.load(Ordering::Relaxed) {
                                on_text(text);
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                        // The board was unplugged, or the port went away. There
                        // is nothing to recover; stop cleanly.
                        Err(_) => break,
                    }
                }
            })
            .map_err(|e| TransportError::Io(format!("could not start the reader: {e}")))?;

        Ok(Console {
            writer,
            shared,
            stop,
            reader: Some(reader),
            port_name: port_name.to_string(),
            baud,
        })
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    pub fn baud(&self) -> u32 {
        self.baud
    }

    /// Send raw bytes to the board.
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
        self.writer.write_all(bytes).map_err(|e| {
            TransportError::Io(format!("could not write to {}: {e}", self.port_name))
        })?;
        self.writer
            .flush()
            .map_err(|e| TransportError::Io(format!("could not write to {}: {e}", self.port_name)))
    }

    /// Suppress user-visible echo while a scripted exchange runs.
    pub(crate) fn set_quiet(&self, quiet: bool) {
        self.shared.quiet.store(quiet, Ordering::Relaxed);
    }

    /// Throw away anything received but not yet consumed.
    pub(crate) fn drain(&self) {
        if let Ok(mut buf) = self.shared.buf.lock() {
            buf.clear();
        }
    }

    /// Wait until `needle` has been received, returning everything before it.
    ///
    /// Both the needle and the bytes before it are consumed, so consecutive
    /// waits walk through a protocol exchange in order.
    pub(crate) fn wait_for(
        &self,
        needle: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, TransportError> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(mut buf) = self.shared.buf.lock() {
                if let Some(at) = find(&buf, needle) {
                    let before = buf[..at].to_vec();
                    buf.drain(..at + needle.len());
                    return Ok(before);
                }
            }
            if Instant::now() >= deadline {
                return Err(TransportError::Io(format!(
                    "the board did not respond as expected within {}s",
                    timeout.as_secs()
                )));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Pulse RTS, which is wired to EN on boards with a USB-to-UART bridge.
    ///
    /// Boards using the ESP32's native USB have no such wire, so this does
    /// nothing on them — a soft reboot through the REPL is the way there.
    pub fn hardware_reset(&mut self) -> Result<(), TransportError> {
        self.writer
            .write_request_to_send(true)
            .map_err(|e| TransportError::Io(e.to_string()))?;
        std::thread::sleep(Duration::from_millis(120));
        self.writer
            .write_request_to_send(false)
            .map_err(|e| TransportError::Io(e.to_string()))?;
        Ok(())
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
    }
}

/// Split off everything that forms complete UTF-8, leaving any trailing partial
/// character in `pending` for the next read to complete.
fn take_decodable(pending: &mut Vec<u8>) -> String {
    match std::str::from_utf8(pending) {
        Ok(s) => {
            let out = s.to_string();
            pending.clear();
            out
        }
        Err(e) => {
            let good = e.valid_up_to();
            let out = String::from_utf8_lossy(&pending[..good]).into_owned();
            match e.error_len() {
                // Genuinely invalid bytes: emit a replacement character for
                // them rather than stalling the stream forever.
                Some(bad) => {
                    pending.drain(..good + bad);
                    format!("{out}\u{FFFD}")
                }
                // Truncated at the end — keep it for the next read.
                None => {
                    pending.drain(..good);
                    out
                }
            }
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A multi-byte character split across two reads must survive. Decoding
    /// each read independently would show a replacement character instead.
    #[test]
    fn characters_split_across_reads_are_not_corrupted() {
        let text = "температура 21°C ✓";
        let bytes = text.as_bytes();
        let mut pending = Vec::new();
        let mut out = String::new();

        // Three bytes at a time, which lands mid-character repeatedly.
        for chunk in bytes.chunks(3) {
            pending.extend_from_slice(chunk);
            out.push_str(&take_decodable(&mut pending));
        }

        assert_eq!(out, text);
        assert!(pending.is_empty(), "nothing should be left over");
    }

    #[test]
    fn plain_ascii_passes_straight_through() {
        let mut pending = b">>> print(1)\r\n1\r\n".to_vec();
        assert_eq!(take_decodable(&mut pending), ">>> print(1)\r\n1\r\n");
        assert!(pending.is_empty());
    }

    /// A truncated character is held back rather than emitted broken.
    #[test]
    fn an_incomplete_character_is_held_back() {
        let mut pending = vec![b'o', b'k', 0xE2, 0x9C]; // "ok" + 2 of 3 bytes of ✓
        assert_eq!(take_decodable(&mut pending), "ok");
        assert_eq!(pending, vec![0xE2, 0x9C]);

        pending.push(0x93);
        assert_eq!(take_decodable(&mut pending), "✓");
    }

    /// Line noise must not stall the stream. A board being reset mid-sentence
    /// emits garbage, and the terminal has to keep going.
    #[test]
    fn invalid_bytes_do_not_stall_the_stream() {
        let mut pending = vec![b'a', 0xFF, b'b'];
        let first = take_decodable(&mut pending);
        assert_eq!(first, "a\u{FFFD}");
        assert_eq!(take_decodable(&mut pending), "b");
        assert!(pending.is_empty());
    }

    #[test]
    fn finds_needles_and_reports_absence() {
        assert_eq!(find(b"hello>world", b">"), Some(5));
        assert_eq!(find(b"raw REPL; CTRL-B to exit\r\n>", b"\r\n>"), Some(24));
        assert_eq!(find(b"nothing here", b"\x04"), None);
        assert_eq!(find(b"", b">"), None);
        assert_eq!(find(b">", b""), None, "an empty needle matches nothing");
    }
}
