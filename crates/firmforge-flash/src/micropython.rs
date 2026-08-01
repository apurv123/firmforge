//! Talking to MicroPython over the serial console.
//!
//! MicroPython exposes two REPLs on the same wire. The *friendly* REPL is what
//! a human sees: a `>>>` prompt that echoes, wraps lines and reformats
//! tracebacks. It is pleasant to type at and miserable to automate against,
//! because output and echo are interleaved with no way to tell them apart.
//!
//! The *raw* REPL exists for exactly this. Entering it with Ctrl-A turns off
//! echo and framing; a command is terminated with Ctrl-D and the board replies
//! with `OK`, then stdout, then `\x04`, then stderr, then `\x04`, then a `>`
//! prompt. That framing is what makes it possible to know whether something
//! worked, and to separate a traceback from a `print`.
//!
//! This is the same protocol `mpremote`, `ampy` and Thonny use, for the same
//! reason.

use crate::console::Console;
use crate::transport::TransportError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Interrupt a running program.
const CTRL_C: &[u8] = b"\x03";
/// Enter the raw REPL.
const CTRL_A: &[u8] = b"\x01";
/// Leave the raw REPL, back to the `>>>` prompt.
const CTRL_B: &[u8] = b"\x02";
/// End of a raw-REPL command, and the terminator of each output stream.
const CTRL_D: &[u8] = b"\x04";

const RAW_BANNER: &[u8] = b"raw REPL; CTRL-B to exit\r\n>";
const HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(2_000);
const EXEC_TIMEOUT: Duration = Duration::from_secs(15);

/// Bytes per chunk when writing a file.
///
/// Each chunk is a separate round trip, so this trades latency against RAM on
/// the board: base64 inflates by a third, and the decoded result has to fit
/// alongside the encoded string on a device that may only have tens of
/// kilobytes free.
const CHUNK: usize = 256;

/// What a board said in response to a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecOutput {
    pub stdout: String,
    /// A traceback, if the code raised. Empty when it succeeded.
    pub stderr: String,
}

impl ExecOutput {
    pub fn ok(&self) -> bool {
        self.stderr.trim().is_empty()
    }

    fn into_result(self) -> Result<Self, TransportError> {
        if self.ok() {
            Ok(self)
        } else {
            Err(TransportError::Io(tidy_traceback(&self.stderr)))
        }
    }
}

/// What was found on the other end of the port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    /// True when a MicroPython REPL answered.
    pub micropython: bool,
    /// e.g. `MicroPython v1.25.0 on 2025-04-15`, when it could be read.
    pub version: Option<String>,
    /// e.g. `ESP32S3 module with ESP32S3`.
    pub machine: Option<String>,
    /// Free bytes on the board's filesystem, when it has one.
    pub free_bytes: Option<u64>,
}

/// Interrupt whatever is running and get back to a known state.
///
/// Two Ctrl-Cs, because the first is often swallowed by a `time.sleep` or a
/// tight loop that only checks for an interrupt between iterations.
pub fn interrupt(console: &mut Console) -> Result<(), TransportError> {
    console.write(b"\r")?;
    console.write(CTRL_C)?;
    std::thread::sleep(Duration::from_millis(60));
    console.write(CTRL_C)?;
    std::thread::sleep(Duration::from_millis(120));
    Ok(())
}

/// Soft reboot: re-runs `boot.py` and `main.py` without power-cycling.
pub fn soft_reboot(console: &mut Console) -> Result<(), TransportError> {
    console.write(b"\r")?;
    console.write(CTRL_D)?;
    Ok(())
}

/// Enter the raw REPL, interrupting anything already running.
fn enter_raw(console: &mut Console) -> Result<(), TransportError> {
    console.set_quiet(true);
    console.drain();

    interrupt(console)?;
    console.drain();
    console.write(b"\r")?;
    console.write(CTRL_A)?;

    console
        .wait_for(RAW_BANNER, HANDSHAKE_TIMEOUT)
        .map_err(|_| {
            // Leaving `quiet` set would silence the terminal for good, and the
            // most likely reason to land here is a board that is not running
            // MicroPython at all — whose output the user still wants to see.
            console.set_quiet(false);
            TransportError::Io(
                "the board did not answer as MicroPython. Is MicroPython installed on it, and \
             is anything else — Thonny, a serial monitor — holding the port?"
                    .into(),
            )
        })?;
    Ok(())
}

/// Leave the raw REPL. Best-effort: the user gets their prompt back either way.
fn exit_raw(console: &mut Console) {
    let _ = console.write(b"\r");
    let _ = console.write(CTRL_B);
    std::thread::sleep(Duration::from_millis(60));
    console.drain();
    console.set_quiet(false);
}

/// Run one command in an already-entered raw REPL.
fn exec_raw(console: &mut Console, code: &str) -> Result<ExecOutput, TransportError> {
    console.write(code.as_bytes())?;
    console.write(CTRL_D)?;

    // The board acknowledges that it accepted the command before running it.
    console
        .wait_for(b"OK", HANDSHAKE_TIMEOUT)
        .map_err(|_| TransportError::Io("the board did not accept the command".into()))?;

    let stdout = console.wait_for(CTRL_D, EXEC_TIMEOUT)?;
    let stderr = console.wait_for(CTRL_D, HANDSHAKE_TIMEOUT)?;
    // Consume the prompt so the next command starts from a clean buffer.
    let _ = console.wait_for(b">", HANDSHAKE_TIMEOUT);

    Ok(ExecOutput {
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

/// Run a snippet and return what it printed, restoring the friendly REPL after.
pub fn exec(console: &mut Console, code: &str) -> Result<ExecOutput, TransportError> {
    enter_raw(console)?;
    let result = exec_raw(console, code);
    exit_raw(console);
    result
}

/// Ask the board what it is.
///
/// Never fails: a board that is not running MicroPython is a normal answer, not
/// an error, and the terminal is still useful for watching its boot log.
pub fn probe(console: &mut Console) -> Probe {
    let code = "import sys\n\
                print(sys.version)\n\
                try:\n \
                   import os\n \
                   print(os.uname().machine)\n\
                except Exception:\n \
                   print('')\n\
                try:\n \
                   import os\n \
                   s = os.statvfs('/')\n \
                   print(s[0] * s[3])\n\
                except Exception:\n \
                   print('')\n";

    match exec(console, code) {
        Ok(out) if out.ok() => {
            let mut lines = out.stdout.lines();
            let version = lines.next().map(str::trim).filter(|s| !s.is_empty());
            let machine = lines.next().map(str::trim).filter(|s| !s.is_empty());
            let free = lines.next().and_then(|l| l.trim().parse::<u64>().ok());
            Probe {
                micropython: true,
                version: version.map(|v| format!("MicroPython {v}")),
                machine: machine.map(str::to_string),
                free_bytes: free,
            }
        }
        _ => Probe {
            micropython: false,
            version: None,
            machine: None,
            free_bytes: None,
        },
    }
}

/// The result of putting a file on the board, checked afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Upload {
    pub path: String,
    pub bytes_sent: usize,
    /// The size the board reports for the file it now has.
    pub bytes_on_device: u64,
    /// True when the board's own SHA-256 of the stored file matched ours.
    pub hash_verified: bool,
    /// True when `bytes_on_device` matched what was sent.
    pub size_verified: bool,
}

impl Upload {
    /// Whether the file on the board is provably what was sent.
    pub fn verified(&self) -> bool {
        self.size_verified && self.hash_verified
    }
}

/// Write a file to the board's filesystem.
///
/// `on_progress` is called with bytes sent so far. Afterwards the board is
/// asked for the file's size and SHA-256 and both are checked, so a truncated
/// or corrupted transfer is reported rather than assumed fine — the same
/// principle the flasher applies to firmware.
pub fn upload<F>(
    console: &mut Console,
    path: &str,
    data: &[u8],
    mut on_progress: F,
) -> Result<Upload, TransportError>
where
    F: FnMut(usize),
{
    if path.trim().is_empty() {
        return Err(TransportError::Io("give the file a name first".into()));
    }

    enter_raw(console)?;
    let result = upload_raw(console, path, data, &mut on_progress);
    exit_raw(console);
    result
}

fn upload_raw(
    console: &mut Console,
    path: &str,
    data: &[u8],
    on_progress: &mut dyn FnMut(usize),
) -> Result<Upload, TransportError> {
    // Nested directories have to exist first; MicroPython's open() will not
    // create them. Failures are ignored because "it already exists" is the
    // common case and is not a problem.
    for parent in parents_of(path) {
        let _ = exec_raw(
            console,
            &format!(
                "import os\ntry:\n os.mkdir({})\nexcept Exception:\n pass\n",
                py_str(&parent)
            ),
        );
    }

    exec_raw(
        console,
        &format!("import binascii\n_f = open({}, 'wb')\n", py_str(path)),
    )?
    .into_result()?;

    let mut sent = 0usize;
    for chunk in data.chunks(CHUNK) {
        let encoded = base64(chunk);
        let outcome = exec_raw(
            console,
            &format!("_f.write(binascii.a2b_base64('{encoded}'))\n"),
        );
        match outcome {
            Ok(out) if out.ok() => {}
            Ok(out) => {
                let _ = exec_raw(console, "_f.close()\n");
                return Err(TransportError::Io(tidy_traceback(&out.stderr)));
            }
            Err(e) => {
                let _ = exec_raw(console, "_f.close()\n");
                return Err(e);
            }
        }
        sent += chunk.len();
        on_progress(sent);
    }

    exec_raw(console, "_f.close()\n")?.into_result()?;

    // Ask the board what it actually stored. hashlib is absent from some
    // builds, so a missing hash downgrades to a size check rather than failing.
    let check = exec_raw(
        console,
        &format!(
            "import os\n\
             _p = {}\n\
             print(os.stat(_p)[6])\n\
             try:\n \
                import hashlib, binascii\n \
                print(binascii.hexlify(hashlib.sha256(open(_p,'rb').read()).digest()).decode())\n\
             except Exception:\n \
                print('')\n",
            py_str(path)
        ),
    )?
    .into_result()?;

    let mut lines = check.stdout.lines();
    let bytes_on_device = lines
        .next()
        .and_then(|l| l.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let device_hash = lines.next().unwrap_or("").trim().to_lowercase();

    let expected = sha256_hex(data);
    Ok(Upload {
        path: path.to_string(),
        bytes_sent: sent,
        bytes_on_device,
        size_verified: bytes_on_device == data.len() as u64,
        hash_verified: !device_hash.is_empty() && device_hash == expected,
    })
}

/// Directory components that need to exist before `path` can be opened.
fn parents_of(path: &str) -> Vec<String> {
    let trimmed = path.trim_start_matches('/');
    let parts: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
    let leading = if path.starts_with('/') { "/" } else { "" };

    (1..parts.len())
        .map(|n| format!("{leading}{}", parts[..n].join("/")))
        .collect()
}

/// Render a Rust string as a Python string literal.
///
/// Paths come from the user, and a stray quote or backslash would otherwise be
/// a syntax error at best and an injected statement at worst.
fn py_str(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// Tracebacks arrive with the raw-REPL framing still attached and the most
/// useful line last. Show that line first.
fn tidy_traceback(stderr: &str) -> String {
    let cleaned = stderr.trim_matches(|c: char| c == '\u{4}' || c.is_whitespace());
    match cleaned.lines().next_back() {
        Some(last) if cleaned.lines().count() > 1 => {
            format!("{}\n\n{cleaned}", last.trim())
        }
        _ => cleaned.to_string(),
    }
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64, so the board can use `binascii.a2b_base64`.
///
/// Written out rather than pulled in as a dependency: it is fifteen lines, and
/// this crate is compiled for mobile targets where every dependency is weight.
fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for group in data.chunks(3) {
        let b = [
            group[0],
            *group.get(1).unwrap_or(&0),
            *group.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if group.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if group.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path with a quote in it would otherwise close the Python literal and
    /// let the rest of the name run as code.
    #[test]
    fn paths_cannot_break_out_of_a_python_literal() {
        assert_eq!(py_str("main.py"), "'main.py'");
        assert_eq!(py_str("it's.py"), r"'it\'s.py'");
        assert_eq!(py_str(r"a\b.py"), r"'a\\b.py'");
        assert_eq!(
            py_str("x'); import os; os.remove('boot.py"),
            r"'x\'); import os; os.remove(\'boot.py'"
        );
    }

    #[test]
    fn base64_matches_the_standard_alphabet_and_padding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    /// Binary content is why base64 is used at all — a .mpy file or an image
    /// would be mangled by any escaping scheme that assumes text.
    #[test]
    fn base64_survives_arbitrary_bytes() {
        let data: Vec<u8> = (0u8..=255).collect();
        let encoded = base64(&data);
        assert!(encoded
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='));
        assert_eq!(encoded.len(), 344);
    }

    #[test]
    fn nested_paths_list_the_directories_that_must_exist() {
        assert_eq!(parents_of("main.py"), Vec::<String>::new());
        assert_eq!(parents_of("lib/thing.py"), vec!["lib"]);
        assert_eq!(parents_of("a/b/c.py"), vec!["a", "a/b"]);
        assert_eq!(parents_of("/lib/x/y.py"), vec!["/lib", "/lib/x"]);
    }

    #[test]
    fn an_upload_is_only_verified_when_both_checks_pass() {
        let base = Upload {
            path: "main.py".into(),
            bytes_sent: 10,
            bytes_on_device: 10,
            hash_verified: true,
            size_verified: true,
        };
        assert!(base.verified());
        assert!(!Upload {
            hash_verified: false,
            ..base.clone()
        }
        .verified());
        assert!(!Upload {
            size_verified: false,
            ..base
        }
        .verified());
    }

    /// The last line of a traceback is the part that says what went wrong, and
    /// it is the part scrolled off the top of a small pane.
    #[test]
    fn tracebacks_lead_with_the_useful_line() {
        let raw = "Traceback (most recent call last):\r\n  File \"<stdin>\", line 1\r\nOSError: [Errno 28] ENOSPC\r\n\x04";
        let tidied = tidy_traceback(raw);
        assert!(tidied.starts_with("OSError: [Errno 28] ENOSPC"));
        assert!(tidied.contains("Traceback"), "full text is still there");
    }

    #[test]
    fn a_single_line_error_is_not_repeated() {
        assert_eq!(tidy_traceback("MemoryError\r\n\x04"), "MemoryError");
    }

    #[test]
    fn exec_output_distinguishes_success_from_a_traceback() {
        let good = ExecOutput {
            stdout: "42\r\n".into(),
            stderr: String::new(),
        };
        assert!(good.ok());
        assert!(good.into_result().is_ok());

        let bad = ExecOutput {
            stdout: String::new(),
            stderr: "NameError: name 'x' isn't defined".into(),
        };
        assert!(!bad.ok());
        assert!(bad.into_result().is_err());
    }

    #[test]
    fn hashes_match_the_reference_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
