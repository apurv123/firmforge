//! firmforge core.
//!
//! Platform-agnostic logic shared by the desktop and mobile applications:
//! the firmware manifest format, device identity, compatibility matching and
//! artifact verification.
//!
//! See `plan/spec/firmforge-spec.md` §6 for the data model this implements.

pub mod device;
pub mod error;
pub mod manifest;
pub mod matching;
pub mod source;
pub mod verify;

pub use device::{Device, DeviceIdentity, SerialType};
pub use error::{Error, Result};
pub use manifest::{Build, Channel, Manifest, Part};
pub use matching::{Compatibility, Incompatibility};
pub use source::{resolve, resolve_all, Origin, ResolvedPart};
pub use verify::verify_sha256;
