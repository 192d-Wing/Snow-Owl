//! Comprehensive error handling tests
//!
//! NIST 800-53: SI-11 (Error Handling)
//! STIG: V-222566
//! Implementation: Tests for error categorization, sanitization, and recovery

use snow_owl_sftp::Error;

/// NIST 800-53: SI-11 - Test error is_recoverable classification
#[test]
fn test_error_is_recoverable() {
    // Recoverable errors
    assert!(Error::Timeout("test".into()).is_recoverable());
    assert!(Error::Connection("test".into()).is_recoverable());
    assert!(Error::ChannelClosed("test".into()).is_recoverable());

    // Non-recoverable errors
    assert!(!Error::FileNotFound("test".into()).is_recoverable());
    assert!(!Error::PermissionDenied("test".into()).is_recoverable());
    assert!(!Error::InvalidPath("test".into()).is_recoverable());
    assert!(!Error::InvalidHandle("test".into()).is_recoverable());
    assert!(!Error::Authentication("test".into()).is_recoverable());
    assert!(!Error::Config("test".into()).is_recoverable());
    assert!(!Error::Protocol("test".into()).is_recoverable());
    assert!(!Error::NotSupported("test".into()).is_recoverable());
    assert!(!Error::ResourceExhaustion("test".into()).is_recoverable());
}

/// NIST 800-53: SI-11 - Test error is_client_error classification
#[test]
fn test_error_is_client_error() {
    // Client errors
    assert!(Error::InvalidPath("test".into()).is_client_error());
    assert!(Error::FileNotFound("test".into()).is_client_error());
    assert!(Error::PermissionDenied("test".into()).is_client_error());
    assert!(Error::InvalidHandle("test".into()).is_client_error());
    assert!(Error::NotSupported("test".into()).is_client_error());
    assert!(Error::Protocol("test".into()).is_client_error());

    // Server errors
    assert!(!Error::Connection("test".into()).is_client_error());
    assert!(!Error::Timeout("test".into()).is_client_error());
    assert!(!Error::ChannelClosed("test".into()).is_client_error());
    assert!(!Error::Config("test".into()).is_client_error());
    assert!(!Error::ResourceExhaustion("test".into()).is_client_error());
    assert!(!Error::Ssh("test".into()).is_client_error());
}

/// NIST 800-53: AU-2, SI-11 - Test error is_security_event classification
#[test]
fn test_error_is_security_event() {
    // Security events that should be audited
    assert!(Error::Authentication("test".into()).is_security_event());
    assert!(Error::PermissionDenied("test".into()).is_security_event());
    assert!(Error::InvalidPath("test".into()).is_security_event());

    // Non-security events
    assert!(!Error::FileNotFound("test".into()).is_security_event());
    assert!(!Error::Timeout("test".into()).is_security_event());
    assert!(!Error::Connection("test".into()).is_security_event());
    assert!(!Error::InvalidHandle("test".into()).is_security_event());
    assert!(!Error::Config("test".into()).is_security_event());
}

/// NIST 800-53: SI-11 - Test SFTP status code mapping
#[test]
fn test_error_to_status_code() {
    use snow_owl_sftp::protocol::StatusCode;

    assert_eq!(
        Error::FileNotFound("test".into()).to_status_code(),
        StatusCode::NoSuchFile as u32
    );
    assert_eq!(
        Error::PermissionDenied("test".into()).to_status_code(),
        StatusCode::PermissionDenied as u32
    );
    assert_eq!(
        Error::InvalidPath("test".into()).to_status_code(),
        StatusCode::BadMessage as u32
    );
    assert_eq!(
        Error::InvalidHandle("test".into()).to_status_code(),
        StatusCode::BadMessage as u32
    );
    assert_eq!(
        Error::NotSupported("test".into()).to_status_code(),
        StatusCode::OpUnsupported as u32
    );
    assert_eq!(
        Error::Timeout("test".into()).to_status_code(),
        StatusCode::Failure as u32
    );
    assert_eq!(
        Error::ChannelClosed("test".into()).to_status_code(),
        StatusCode::ConnectionLost as u32
    );
    assert_eq!(
        Error::Connection("test".into()).to_status_code(),
        StatusCode::ConnectionLost as u32
    );
}

/// NIST 800-53: SI-11 - Test sanitized error messages
/// STIG: V-222566 - Error messages must not reveal sensitive information
#[test]
fn test_error_sanitized_messages() {
    // Authentication errors should not reveal why they failed
    let auth_err = Error::Authentication("Invalid key format XYZ".into());
    assert_eq!(auth_err.sanitized_message(), "Authentication failed");
    assert!(!auth_err.sanitized_message().contains("key"));
    assert!(!auth_err.sanitized_message().contains("XYZ"));

    // Permission errors should not reveal paths
    let perm_err = Error::PermissionDenied("/etc/shadow".into());
    assert_eq!(perm_err.sanitized_message(), "Permission denied");
    assert!(!perm_err.sanitized_message().contains("/etc/shadow"));

    // Invalid path should not reveal the path
    let path_err = Error::InvalidPath("../../../etc/passwd".into());
    assert_eq!(path_err.sanitized_message(), "Invalid path");
    assert!(!path_err.sanitized_message().contains("passwd"));

    // Config errors should not reveal configuration details
    let config_err = Error::Config("Missing host_key at /secure/path/key.pem".into());
    assert_eq!(config_err.sanitized_message(), "Server configuration error");
    assert!(!config_err.sanitized_message().contains("/secure"));
    assert!(!config_err.sanitized_message().contains("key.pem"));
}

/// NIST 800-53: SI-11 - Test error constructor helpers
#[test]
fn test_error_constructors() {
    let timeout_err = Error::timeout("Operation timed out");
    assert!(matches!(timeout_err, Error::Timeout(_)));
    assert!(timeout_err.is_recoverable());

    let channel_err = Error::channel_closed("Channel was closed");
    assert!(matches!(channel_err, Error::ChannelClosed(_)));
    assert!(channel_err.is_recoverable());

    let handle_err = Error::invalid_handle("Handle not found");
    assert!(matches!(handle_err, Error::InvalidHandle(_)));
    assert!(handle_err.is_client_error());

    let resource_err = Error::resource_exhaustion("Too many handles");
    assert!(matches!(resource_err, Error::ResourceExhaustion(_)));
}

/// NIST 800-53: SI-11 - Test error display messages
#[test]
fn test_error_display() {
    let err = Error::FileNotFound("test.txt".into());
    assert!(format!("{}", err).contains("File not found"));
    assert!(format!("{}", err).contains("test.txt"));

    let err = Error::Timeout("Read operation".into());
    assert!(format!("{}", err).contains("timed out"));

    let err = Error::InvalidHandle("123".into());
    assert!(format!("{}", err).contains("Invalid file handle"));
}

/// NIST 800-53: SI-11 - Test IO error conversion
#[test]
fn test_io_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let sftp_err: Error = io_err.into();

    assert!(matches!(sftp_err, Error::Io(_)));
}

/// NIST 800-53: SI-11 - Test error combinations for retry logic
#[test]
fn test_retry_logic_classification() {
    let recoverable_errors = vec![
        Error::Timeout("test".into()),
        Error::Connection("test".into()),
        Error::ChannelClosed("test".into()),
    ];

    for err in recoverable_errors {
        assert!(
            err.is_recoverable() && !err.is_client_error(),
            "Recoverable errors should not be client errors: {:?}",
            err
        );
    }

    let permanent_errors = vec![
        Error::FileNotFound("test".into()),
        Error::PermissionDenied("test".into()),
        Error::InvalidPath("test".into()),
    ];

    for err in permanent_errors {
        assert!(
            !err.is_recoverable(),
            "Client errors should not be recoverable: {:?}",
            err
        );
    }
}

/// NIST 800-53: AU-2, SI-11 - Test security event logging classification
#[test]
fn test_security_events_for_audit() {
    let security_events = vec![
        Error::Authentication("test".into()),
        Error::PermissionDenied("test".into()),
        Error::InvalidPath("test".into()),
    ];

    for err in security_events {
        assert!(
            err.is_security_event(),
            "Should be classified as security event: {:?}",
            err
        );
    }

    let non_security_events = vec![
        Error::FileNotFound("test".into()),
        Error::Timeout("test".into()),
        Error::Config("test".into()),
    ];

    for err in non_security_events {
        assert!(
            !err.is_security_event(),
            "Should not be classified as security event: {:?}",
            err
        );
    }
}

/// NIST 800-53: SI-11 - Test comprehensive error type coverage
#[test]
fn test_all_error_types() {
    let all_errors = vec![
        Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "test")),
        Error::Ssh("test".into()),
        Error::Protocol("test".into()),
        Error::Authentication("test".into()),
        Error::FileNotFound("test".into()),
        Error::PermissionDenied("test".into()),
        Error::InvalidPath("test".into()),
        Error::Config("test".into()),
        Error::Connection("test".into()),
        Error::Timeout("test".into()),
        Error::InvalidHandle("test".into()),
        Error::ResourceExhaustion("test".into()),
        Error::NotSupported("test".into()),
        Error::ChannelClosed("test".into()),
        Error::Other("test".into()),
    ];

    // Ensure all errors can be created and have valid status codes
    for err in all_errors {
        let status_code = err.to_status_code();
        assert!(
            status_code <= 8,
            "Status code should be valid SFTP code: {}",
            status_code
        );

        let _ = err.sanitized_message();
        let _ = err.is_recoverable();
        let _ = err.is_client_error();
        let _ = err.is_security_event();
    }
}
