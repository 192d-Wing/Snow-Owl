//! Comprehensive protocol encoding/decoding tests
//!
//! NIST 800-53: SI-11 (Error Handling), SI-10 (Input Validation)
//! STIG: V-222566, V-222396
//! Implementation: Tests for RFC-compliant protocol implementation

use bytes::BytesMut;
use snow_owl_sftp::protocol::{codec, FileAttrs, MessageType, OpenFlags, StatusCode, SFTP_VERSION};

/// NIST 800-53: SI-11 - Test protocol message type conversions
#[test]
fn test_all_message_type_conversions() {
    // Request types
    assert_eq!(MessageType::try_from(1).unwrap(), MessageType::Init);
    assert_eq!(MessageType::try_from(3).unwrap(), MessageType::Open);
    assert_eq!(MessageType::try_from(4).unwrap(), MessageType::Close);
    assert_eq!(MessageType::try_from(5).unwrap(), MessageType::Read);
    assert_eq!(MessageType::try_from(6).unwrap(), MessageType::Write);
    assert_eq!(MessageType::try_from(7).unwrap(), MessageType::Lstat);
    assert_eq!(MessageType::try_from(8).unwrap(), MessageType::Fstat);
    assert_eq!(MessageType::try_from(9).unwrap(), MessageType::Setstat);
    assert_eq!(MessageType::try_from(10).unwrap(), MessageType::Fsetstat);
    assert_eq!(MessageType::try_from(11).unwrap(), MessageType::Opendir);
    assert_eq!(MessageType::try_from(12).unwrap(), MessageType::Readdir);
    assert_eq!(MessageType::try_from(13).unwrap(), MessageType::Remove);
    assert_eq!(MessageType::try_from(14).unwrap(), MessageType::Mkdir);
    assert_eq!(MessageType::try_from(15).unwrap(), MessageType::Rmdir);
    assert_eq!(MessageType::try_from(16).unwrap(), MessageType::Realpath);
    assert_eq!(MessageType::try_from(17).unwrap(), MessageType::Stat);
    assert_eq!(MessageType::try_from(18).unwrap(), MessageType::Rename);

    // Response types
    assert_eq!(MessageType::try_from(2).unwrap(), MessageType::Version);
    assert_eq!(MessageType::try_from(101).unwrap(), MessageType::Status);
    assert_eq!(MessageType::try_from(102).unwrap(), MessageType::Handle);
    assert_eq!(MessageType::try_from(103).unwrap(), MessageType::Data);
    assert_eq!(MessageType::try_from(104).unwrap(), MessageType::Name);
    assert_eq!(MessageType::try_from(105).unwrap(), MessageType::Attrs);
}

/// NIST 800-53: SI-10 - Test invalid message type handling
#[test]
fn test_invalid_message_types() {
    // Test various invalid values
    assert!(MessageType::try_from(0).is_err());
    assert!(MessageType::try_from(21).is_err());
    assert!(MessageType::try_from(100).is_err());
    assert!(MessageType::try_from(106).is_err());
    assert!(MessageType::try_from(255).is_err());
}

/// NIST 800-53: SI-11 - Test all status codes
#[test]
fn test_all_status_codes() {
    assert_eq!(u32::from(StatusCode::Ok), 0);
    assert_eq!(u32::from(StatusCode::Eof), 1);
    assert_eq!(u32::from(StatusCode::NoSuchFile), 2);
    assert_eq!(u32::from(StatusCode::PermissionDenied), 3);
    assert_eq!(u32::from(StatusCode::Failure), 4);
    assert_eq!(u32::from(StatusCode::BadMessage), 5);
    assert_eq!(u32::from(StatusCode::NoConnection), 6);
    assert_eq!(u32::from(StatusCode::ConnectionLost), 7);
    assert_eq!(u32::from(StatusCode::OpUnsupported), 8);
}

/// NIST 800-53: SI-11 - Test OpenFlags operations
#[test]
fn test_open_flags_all_combinations() {
    // Test individual flags
    let read_only = OpenFlags(OpenFlags::READ);
    assert!(read_only.has_read());
    assert!(!read_only.has_write());
    assert!(!read_only.has_append());
    assert!(!read_only.has_creat());
    assert!(!read_only.has_trunc());
    assert!(!read_only.has_excl());

    let write_only = OpenFlags(OpenFlags::WRITE);
    assert!(!write_only.has_read());
    assert!(write_only.has_write());

    let append = OpenFlags(OpenFlags::APPEND);
    assert!(append.has_append());

    let create = OpenFlags(OpenFlags::CREAT);
    assert!(create.has_creat());

    let truncate = OpenFlags(OpenFlags::TRUNC);
    assert!(truncate.has_trunc());

    let exclusive = OpenFlags(OpenFlags::EXCL);
    assert!(exclusive.has_excl());

    // Test combinations
    let read_write = OpenFlags(OpenFlags::READ | OpenFlags::WRITE);
    assert!(read_write.has_read());
    assert!(read_write.has_write());

    let create_write = OpenFlags(OpenFlags::WRITE | OpenFlags::CREAT | OpenFlags::TRUNC);
    assert!(create_write.has_write());
    assert!(create_write.has_creat());
    assert!(create_write.has_trunc());
    assert!(!create_write.has_read());
}

/// NIST 800-53: SI-11 - Test FileAttrs encoding with all fields
#[test]
fn test_file_attrs_complete_encoding() {
    let attrs = FileAttrs {
        size: Some(1024 * 1024), // 1MB
        uid: Some(1000),
        gid: Some(1000),
        permissions: Some(0o755),
        atime: Some(1234567890),
        mtime: Some(1234567900),
    };

    let encoded = attrs.encode();
    let mut buf = &encoded[..];
    let decoded = FileAttrs::decode(&mut buf).unwrap();

    assert_eq!(decoded.size, Some(1024 * 1024));
    assert_eq!(decoded.uid, Some(1000));
    assert_eq!(decoded.gid, Some(1000));
    assert_eq!(decoded.permissions, Some(0o755));
    assert_eq!(decoded.atime, Some(1234567890));
    assert_eq!(decoded.mtime, Some(1234567900));
}

/// NIST 800-53: SI-11 - Test FileAttrs with partial fields
#[test]
fn test_file_attrs_partial_fields() {
    // Only size
    let attrs = FileAttrs {
        size: Some(2048),
        uid: None,
        gid: None,
        permissions: None,
        atime: None,
        mtime: None,
    };

    let encoded = attrs.encode();
    let mut buf = &encoded[..];
    let decoded = FileAttrs::decode(&mut buf).unwrap();

    assert_eq!(decoded.size, Some(2048));
    assert_eq!(decoded.uid, None);
    assert_eq!(decoded.gid, None);
    assert_eq!(decoded.permissions, None);
}

/// NIST 800-53: SI-11 - Test FileAttrs with no fields
#[test]
fn test_file_attrs_empty() {
    let attrs = FileAttrs::default();

    let encoded = attrs.encode();
    let mut buf = &encoded[..];
    let decoded = FileAttrs::decode(&mut buf).unwrap();

    assert_eq!(decoded.size, None);
    assert_eq!(decoded.uid, None);
    assert_eq!(decoded.gid, None);
    assert_eq!(decoded.permissions, None);
    assert_eq!(decoded.atime, None);
    assert_eq!(decoded.mtime, None);
}

/// NIST 800-53: SI-10 - Test string codec with various lengths
#[test]
fn test_codec_string_various_lengths() {
    let test_cases = vec![
        "",
        "a",
        "Hello",
        "Hello, SFTP!",
        &"x".repeat(100),
        &"y".repeat(1000),
        &"long string with unicode: 你好世界 🚀",
    ];

    for test_string in test_cases {
        let mut buf = BytesMut::new();
        codec::put_string(&mut buf, test_string);

        let mut read_buf = &buf[..];
        let decoded = codec::get_string(&mut read_buf).unwrap();

        assert_eq!(decoded, test_string, "Failed for: {}", test_string);
    }
}

/// NIST 800-53: SI-10 - Test bytes codec with various lengths
#[test]
fn test_codec_bytes_various_lengths() {
    let test_cases: Vec<Vec<u8>> = vec![
        vec![],
        vec![0],
        vec![1, 2, 3, 4, 5],
        vec![255; 10],
        (0..=255).collect(),
        vec![0; 1000],
    ];

    for test_data in test_cases {
        let mut buf = BytesMut::new();
        codec::put_bytes(&mut buf, &test_data);

        let mut read_buf = &buf[..];
        let decoded = codec::get_bytes(&mut read_buf).unwrap();

        assert_eq!(decoded, test_data);
    }
}

/// NIST 800-53: SI-10 - Test codec with invalid data (insufficient length)
#[test]
fn test_codec_string_insufficient_data() {
    // Create buffer with length prefix but insufficient data
    let mut buf = BytesMut::new();
    buf.put_u32(100); // Says 100 bytes but no data follows

    let mut read_buf = &buf[..];
    let result = codec::get_string(&mut read_buf);

    assert!(result.is_err(), "Should fail with insufficient data");
}

/// NIST 800-53: SI-10 - Test codec with invalid UTF-8
#[test]
fn test_codec_string_invalid_utf8() {
    let mut buf = BytesMut::new();
    buf.put_u32(4); // Length
    buf.put_slice(&[0xFF, 0xFE, 0xFD, 0xFC]); // Invalid UTF-8

    let mut read_buf = &buf[..];
    let result = codec::get_string(&mut read_buf);

    assert!(result.is_err(), "Should fail with invalid UTF-8");
}

/// NIST 800-53: SI-10 - Test bytes codec with insufficient data
#[test]
fn test_codec_bytes_insufficient_data() {
    let mut buf = BytesMut::new();
    buf.put_u32(100); // Says 100 bytes but no data follows

    let mut read_buf = &buf[..];
    let result = codec::get_bytes(&mut read_buf);

    assert!(result.is_err(), "Should fail with insufficient data");
}

/// NIST 800-53: SI-11 - Test SFTP version constant
#[test]
fn test_sftp_version() {
    assert_eq!(SFTP_VERSION, 3, "SFTP version should be 3 per draft-ietf-secsh-filexfer-02");
}

/// NIST 800-53: SI-11 - Test FileAttrs with maximum values
#[test]
fn test_file_attrs_max_values() {
    let attrs = FileAttrs {
        size: Some(u64::MAX),
        uid: Some(u32::MAX),
        gid: Some(u32::MAX),
        permissions: Some(0o7777),
        atime: Some(u32::MAX),
        mtime: Some(u32::MAX),
    };

    let encoded = attrs.encode();
    let mut buf = &encoded[..];
    let decoded = FileAttrs::decode(&mut buf).unwrap();

    assert_eq!(decoded.size, Some(u64::MAX));
    assert_eq!(decoded.uid, Some(u32::MAX));
    assert_eq!(decoded.gid, Some(u32::MAX));
    assert_eq!(decoded.permissions, Some(0o7777));
    assert_eq!(decoded.atime, Some(u32::MAX));
    assert_eq!(decoded.mtime, Some(u32::MAX));
}

/// NIST 800-53: SI-11 - Test round-trip encoding of permissions
#[test]
fn test_permissions_encoding() {
    let test_permissions = vec![
        0o000, 0o400, 0o600, 0o644, 0o755, 0o777,
        0o1000, 0o2000, 0o4000, // setuid, setgid, sticky
    ];

    for perm in test_permissions {
        let attrs = FileAttrs {
            size: None,
            uid: None,
            gid: None,
            permissions: Some(perm),
            atime: None,
            mtime: None,
        };

        let encoded = attrs.encode();
        let mut buf = &encoded[..];
        let decoded = FileAttrs::decode(&mut buf).unwrap();

        assert_eq!(decoded.permissions, Some(perm), "Failed for permission: {:o}", perm);
    }
}
