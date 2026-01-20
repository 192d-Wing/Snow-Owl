//! Integration tests for Snow Owl SFTP
//!
//! These tests verify RFC compliance and basic functionality

use snow_owl_sftp::{Config, Server};
use std::path::PathBuf;

#[tokio::test]
async fn test_server_creation() {
    let mut config = Config::default();
    config.root_dir = PathBuf::from("/tmp/sftp_test");

    // Create test directory
    std::fs::create_dir_all(&config.root_dir).unwrap();

    let result = Server::new(config).await;
    assert!(result.is_ok(), "Server should be created successfully");

    // Cleanup
    std::fs::remove_dir_all("/tmp/sftp_test").ok();
}

#[test]
fn test_config_validation() {
    let mut config = Config::default();
    config.root_dir = PathBuf::from("/nonexistent/directory");

    let result = config.validate();
    assert!(result.is_err(), "Config validation should fail for non-existent directory");
}

#[test]
fn test_config_packet_size_validation() {
    let mut config = Config::default();
    config.root_dir = PathBuf::from("/tmp");
    config.max_packet_size = 1024; // Too small per RFC 4254

    let result = config.validate();
    assert!(result.is_err(), "Config validation should fail for packet size < 32768");
}

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.port, 2222);
    assert_eq!(config.bind_address, "0.0.0.0");
    assert!(config.max_packet_size >= 32768, "Default packet size should meet RFC 4254 minimum");
}

mod protocol_tests {
    use snow_owl_sftp::protocol::{FileAttrs, MessageType, OpenFlags, StatusCode};

    #[test]
    fn test_message_type_conversion() {
        assert_eq!(MessageType::try_from(1).unwrap(), MessageType::Init);
        assert_eq!(MessageType::try_from(2).unwrap(), MessageType::Version);
        assert_eq!(MessageType::try_from(3).unwrap(), MessageType::Open);
        assert_eq!(MessageType::try_from(101).unwrap(), MessageType::Status);
    }

    #[test]
    fn test_invalid_message_type() {
        let result = MessageType::try_from(255);
        assert!(result.is_err(), "Invalid message type should return error");
    }

    #[test]
    fn test_open_flags() {
        let flags = OpenFlags(OpenFlags::READ | OpenFlags::WRITE);
        assert!(flags.has_read());
        assert!(flags.has_write());
        assert!(!flags.has_append());
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(u32::from(StatusCode::Ok), 0);
        assert_eq!(u32::from(StatusCode::Eof), 1);
        assert_eq!(u32::from(StatusCode::NoSuchFile), 2);
        assert_eq!(u32::from(StatusCode::PermissionDenied), 3);
    }

    #[test]
    fn test_file_attrs_encode_decode() {
        let attrs = FileAttrs {
            size: Some(1024),
            uid: Some(1000),
            gid: Some(1000),
            permissions: Some(0o644),
            atime: Some(1234567890),
            mtime: Some(1234567890),
        };

        let encoded = attrs.encode();
        let mut buf = &encoded[..];
        let decoded = FileAttrs::decode(&mut buf).unwrap();

        assert_eq!(decoded.size, Some(1024));
        assert_eq!(decoded.uid, Some(1000));
        assert_eq!(decoded.gid, Some(1000));
        assert_eq!(decoded.permissions, Some(0o644));
    }

    #[test]
    fn test_codec_string_encoding() {
        use snow_owl_sftp::protocol::codec;
        use bytes::BytesMut;

        let test_string = "Hello, SFTP!";
        let mut buf = BytesMut::new();
        codec::put_string(&mut buf, test_string);

        let mut read_buf = &buf[..];
        let decoded = codec::get_string(&mut read_buf).unwrap();

        assert_eq!(decoded, test_string);
    }

    #[test]
    fn test_codec_bytes_encoding() {
        use snow_owl_sftp::protocol::codec;
        use bytes::BytesMut;

        let test_data = vec![1, 2, 3, 4, 5];
        let mut buf = BytesMut::new();
        codec::put_bytes(&mut buf, &test_data);

        let mut read_buf = &buf[..];
        let decoded = codec::get_bytes(&mut read_buf).unwrap();

        assert_eq!(decoded, test_data);
    }
}
