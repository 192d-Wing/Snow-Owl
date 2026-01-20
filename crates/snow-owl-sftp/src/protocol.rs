//! SFTP Protocol Implementation
//!
//! This module implements the SSH File Transfer Protocol as defined in
//! draft-ietf-secsh-filexfer-02 and later versions.
//!
//! The SFTP protocol runs over the SSH connection protocol (RFC 4254),
//! using the "sftp" subsystem.

use bytes::{Buf, BufMut, Bytes, BytesMut};

/// SFTP Protocol Version
pub const SFTP_VERSION: u32 = 3;

/// SFTP message types (as defined in the SFTP specification)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Initialize SFTP session
    Init = 1,
    /// Version response
    Version = 2,
    /// Open file
    Open = 3,
    /// Close file
    Close = 4,
    /// Read from file
    Read = 5,
    /// Write to file
    Write = 6,
    /// Get file attributes by path
    Lstat = 7,
    /// Get file attributes by handle
    Fstat = 8,
    /// Set file attributes by path
    Setstat = 9,
    /// Set file attributes by handle
    Fsetstat = 10,
    /// Open directory
    Opendir = 11,
    /// Read directory entries
    Readdir = 12,
    /// Remove file
    Remove = 13,
    /// Create directory
    Mkdir = 14,
    /// Remove directory
    Rmdir = 15,
    /// Get real path
    Realpath = 16,
    /// Get file attributes by path (follow symlinks)
    Stat = 17,
    /// Rename file or directory
    Rename = 18,
    /// Read symbolic link
    Readlink = 19,
    /// Create symbolic link
    Symlink = 20,
    /// Status response
    Status = 101,
    /// Handle response
    Handle = 102,
    /// Data response
    Data = 103,
    /// Name response (for directory listings)
    Name = 104,
    /// Attributes response
    Attrs = 105,
    /// Extended request
    Extended = 200,
    /// Extended reply
    ExtendedReply = 201,
}

impl TryFrom<u8> for MessageType {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::Init),
            2 => Ok(MessageType::Version),
            3 => Ok(MessageType::Open),
            4 => Ok(MessageType::Close),
            5 => Ok(MessageType::Read),
            6 => Ok(MessageType::Write),
            7 => Ok(MessageType::Lstat),
            8 => Ok(MessageType::Fstat),
            9 => Ok(MessageType::Setstat),
            10 => Ok(MessageType::Fsetstat),
            11 => Ok(MessageType::Opendir),
            12 => Ok(MessageType::Readdir),
            13 => Ok(MessageType::Remove),
            14 => Ok(MessageType::Mkdir),
            15 => Ok(MessageType::Rmdir),
            16 => Ok(MessageType::Realpath),
            17 => Ok(MessageType::Stat),
            18 => Ok(MessageType::Rename),
            19 => Ok(MessageType::Readlink),
            20 => Ok(MessageType::Symlink),
            101 => Ok(MessageType::Status),
            102 => Ok(MessageType::Handle),
            103 => Ok(MessageType::Data),
            104 => Ok(MessageType::Name),
            105 => Ok(MessageType::Attrs),
            200 => Ok(MessageType::Extended),
            201 => Ok(MessageType::ExtendedReply),
            _ => Err(crate::Error::Protocol(format!(
                "Unknown message type: {}",
                value
            ))),
        }
    }
}

/// SFTP Status codes (RFC draft-ietf-secsh-filexfer)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    /// Success
    Ok = 0,
    /// End of file
    Eof = 1,
    /// No such file or directory
    NoSuchFile = 2,
    /// Permission denied
    PermissionDenied = 3,
    /// General failure
    Failure = 4,
    /// Bad message
    BadMessage = 5,
    /// No connection
    NoConnection = 6,
    /// Connection lost
    ConnectionLost = 7,
    /// Operation not supported
    OpUnsupported = 8,
}

impl From<StatusCode> for u32 {
    fn from(code: StatusCode) -> u32 {
        code as u32
    }
}

/// File open flags (as defined in SFTP spec)
#[derive(Debug, Clone, Copy)]
pub struct OpenFlags(pub u32);

impl OpenFlags {
    pub const READ: u32 = 0x00000001;
    pub const WRITE: u32 = 0x00000002;
    pub const APPEND: u32 = 0x00000004;
    pub const CREAT: u32 = 0x00000008;
    pub const TRUNC: u32 = 0x00000010;
    pub const EXCL: u32 = 0x00000020;

    pub fn has_read(&self) -> bool {
        self.0 & Self::READ != 0
    }

    pub fn has_write(&self) -> bool {
        self.0 & Self::WRITE != 0
    }

    pub fn has_append(&self) -> bool {
        self.0 & Self::APPEND != 0
    }

    pub fn has_creat(&self) -> bool {
        self.0 & Self::CREAT != 0
    }

    pub fn has_trunc(&self) -> bool {
        self.0 & Self::TRUNC != 0
    }

    pub fn has_excl(&self) -> bool {
        self.0 & Self::EXCL != 0
    }
}

/// File attributes (as defined in SFTP spec)
#[derive(Debug, Clone, Default)]
pub struct FileAttrs {
    pub size: Option<u64>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub permissions: Option<u32>,
    pub atime: Option<u32>,
    pub mtime: Option<u32>,
}

impl FileAttrs {
    const FLAG_SIZE: u32 = 0x00000001;
    const FLAG_UIDGID: u32 = 0x00000002;
    const FLAG_PERMISSIONS: u32 = 0x00000004;
    const FLAG_ACMODTIME: u32 = 0x00000008;

    /// Encode file attributes to bytes
    pub fn encode(&self) -> BytesMut {
        let mut buf = BytesMut::new();
        let mut flags = 0u32;

        if self.size.is_some() {
            flags |= Self::FLAG_SIZE;
        }
        if self.uid.is_some() && self.gid.is_some() {
            flags |= Self::FLAG_UIDGID;
        }
        if self.permissions.is_some() {
            flags |= Self::FLAG_PERMISSIONS;
        }
        if self.atime.is_some() && self.mtime.is_some() {
            flags |= Self::FLAG_ACMODTIME;
        }

        buf.put_u32(flags);

        if let Some(size) = self.size {
            buf.put_u64(size);
        }
        if let (Some(uid), Some(gid)) = (self.uid, self.gid) {
            buf.put_u32(uid);
            buf.put_u32(gid);
        }
        if let Some(permissions) = self.permissions {
            buf.put_u32(permissions);
        }
        if let (Some(atime), Some(mtime)) = (self.atime, self.mtime) {
            buf.put_u32(atime);
            buf.put_u32(mtime);
        }

        buf
    }

    /// Decode file attributes from bytes
    pub fn decode(buf: &mut &[u8]) -> crate::Result<Self> {
        if buf.remaining() < 4 {
            return Err(crate::Error::Protocol("Insufficient data for flags".into()));
        }

        let flags = buf.get_u32();
        let mut attrs = FileAttrs::default();

        if flags & Self::FLAG_SIZE != 0 {
            if buf.remaining() < 8 {
                return Err(crate::Error::Protocol("Insufficient data for size".into()));
            }
            attrs.size = Some(buf.get_u64());
        }

        if flags & Self::FLAG_UIDGID != 0 {
            if buf.remaining() < 8 {
                return Err(crate::Error::Protocol("Insufficient data for uid/gid".into()));
            }
            attrs.uid = Some(buf.get_u32());
            attrs.gid = Some(buf.get_u32());
        }

        if flags & Self::FLAG_PERMISSIONS != 0 {
            if buf.remaining() < 4 {
                return Err(crate::Error::Protocol(
                    "Insufficient data for permissions".into(),
                ));
            }
            attrs.permissions = Some(buf.get_u32());
        }

        if flags & Self::FLAG_ACMODTIME != 0 {
            if buf.remaining() < 8 {
                return Err(crate::Error::Protocol(
                    "Insufficient data for atime/mtime".into(),
                ));
            }
            attrs.atime = Some(buf.get_u32());
            attrs.mtime = Some(buf.get_u32());
        }

        Ok(attrs)
    }
}

/// Helper functions for encoding/decoding SFTP protocol strings
pub mod codec {
    use bytes::{Buf, BufMut, BytesMut};

    /// Encode a string as SFTP string (length + data)
    pub fn put_string(buf: &mut BytesMut, s: &str) {
        buf.put_u32(s.len() as u32);
        buf.put_slice(s.as_bytes());
    }

    /// Decode an SFTP string
    pub fn get_string(buf: &mut &[u8]) -> crate::Result<String> {
        if buf.remaining() < 4 {
            return Err(crate::Error::Protocol(
                "Insufficient data for string length".into(),
            ));
        }

        let len = buf.get_u32() as usize;
        if buf.remaining() < len {
            return Err(crate::Error::Protocol("Insufficient data for string".into()));
        }

        let bytes = &buf[..len];
        buf.advance(len);

        String::from_utf8(bytes.to_vec())
            .map_err(|e| crate::Error::Protocol(format!("Invalid UTF-8 string: {}", e)))
    }

    /// Encode bytes as SFTP string (length + data)
    pub fn put_bytes(buf: &mut BytesMut, data: &[u8]) {
        buf.put_u32(data.len() as u32);
        buf.put_slice(data);
    }

    /// Decode SFTP bytes
    pub fn get_bytes(buf: &mut &[u8]) -> crate::Result<Vec<u8>> {
        if buf.remaining() < 4 {
            return Err(crate::Error::Protocol(
                "Insufficient data for bytes length".into(),
            ));
        }

        let len = buf.get_u32() as usize;
        if buf.remaining() < len {
            return Err(crate::Error::Protocol("Insufficient data for bytes".into()));
        }

        let bytes = &buf[..len];
        buf.advance(len);

        Ok(bytes.to_vec())
    }
}
