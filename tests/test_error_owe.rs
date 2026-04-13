use orion_error::ErrorCode;
use orion_error::ErrorOwe;
use orion_error::ErrorOweBase;
use orion_error::ErrorOweSource;
use orion_error::{StructError, UvsReason};
use std::error::Error as StdError;

#[test]
fn test_owe_basic_conversion() {
    let result: Result<i32, &str> = Err("test error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe(UvsReason::business_error());

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 101);
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("test error"));
}

#[test]
fn test_owe_biz() {
    let result: Result<i32, &str> = Err("business error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe_biz();

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 101);
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("business error"));
}

#[test]
fn test_owe_validation() {
    let result: Result<i32, &str> = Err("validation error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe_validation();

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 100);
    assert!(converted.as_ref().unwrap_err().detail().is_some());
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("validation error"));
}

#[test]
fn test_owe_data() {
    // Test owe_data for data errors
    let result: Result<Vec<i32>, &str> = Err("data corruption");

    let converted: Result<Vec<i32>, StructError<UvsReason>> = result.owe_data();
    assert!(converted.is_err());

    let error = converted.unwrap_err();
    assert_eq!(error.reason().error_code(), 200); // data error code
    assert!(error.detail().is_some());
    assert!(error.detail().as_ref().unwrap().contains("data corruption"));
}

#[test]
fn test_owe_conf() {
    // Test owe_conf for configuration errors
    let result: Result<bool, &str> = Err("config missing");

    let converted: Result<bool, StructError<UvsReason>> = result.owe_conf();
    assert!(converted.is_err());

    let error = converted.unwrap_err();
    assert_eq!(error.reason().error_code(), 300); // config error code
    assert!(error.detail().is_some());
    assert!(error.detail().as_ref().unwrap().contains("config missing"));
}

#[test]
fn test_owe_res() {
    // Test owe_res for resource errors
    let result: Result<(), &str> = Err("memory full");

    let converted: Result<(), StructError<UvsReason>> = result.owe_res();
    assert!(converted.is_err());

    let error = converted.unwrap_err();
    assert_eq!(error.reason().error_code(), 203); // resource error code
    assert!(error.detail().is_some());
    assert!(error.detail().as_ref().unwrap().contains("memory full"));
}

#[test]
fn test_owe_net() {
    // Test owe_net for network errors
    let result: Result<(), &str> = Err("connection failed");

    let converted: Result<(), StructError<UvsReason>> = result.owe_net();
    assert!(converted.is_err());

    let error = converted.unwrap_err();
    assert_eq!(error.reason().error_code(), 202); // network error code
    assert!(error.detail().is_some());
    assert!(error
        .detail()
        .as_ref()
        .unwrap()
        .contains("connection failed"));
}

#[test]
fn test_owe_sys() {
    // Test owe_sys for system errors
    let result: Result<(), &str> = Err("system crash");

    let converted: Result<(), StructError<UvsReason>> = result.owe_sys();
    assert!(converted.is_err());

    let error = converted.unwrap_err();
    assert_eq!(error.reason().error_code(), 201); // system error code
    assert!(error.detail().is_some());
    assert!(error.detail().as_ref().unwrap().contains("system crash"));
}

#[test]
fn test_owe_logic() {
    // Test owe_logic for logic errors
    let result: Result<(), &str> = Err("logic bug");

    let converted: Result<(), StructError<UvsReason>> = result.owe_logic();
    assert!(converted.is_err());

    let error = converted.unwrap_err();
    assert_eq!(error.reason().error_code(), 104); // logic error code
    assert!(error.detail().is_some());
    assert!(error.detail().as_ref().unwrap().contains("logic bug"));
}

#[test]
fn test_owe_success_case() {
    // Test that successful results are preserved
    let result: Result<i32, &str> = Ok(42);

    let converted: Result<i32, StructError<UvsReason>> = result.owe_biz();
    assert!(converted.is_ok());
    assert_eq!(converted.unwrap(), 42);
}

#[test]
fn test_owe_with_different_error_types() {
    // Test with different error types that implement Display
    let result: Result<(), std::io::Error> = Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));

    let converted: Result<(), StructError<UvsReason>> = result.owe_sys();
    assert!(converted.is_err());

    let error = converted.unwrap_err();
    assert_eq!(error.reason().error_code(), 201); // system error code
    assert!(error.detail().is_some());
    assert!(error.detail().as_ref().unwrap().contains("file not found"));
}

#[test]
fn test_owe_network() {
    let result: Result<i32, &str> = Err("network error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe_net();

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 202);
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("network error"));
}

#[test]
fn test_owe_resource() {
    let result: Result<i32, &str> = Err("resource error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe_res();

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 203);
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("resource error"));
}

#[test]
fn test_owe_timeout() {
    let result: Result<i32, &str> = Err("timeout error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe_timeout();

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 204);
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("timeout error"));
}

#[test]
fn test_owe_not_found() {
    let result: Result<i32, &str> = Err("not found error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe(UvsReason::not_found_error());

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 102);
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("not found error"));
}

#[test]
fn test_owe_permission() {
    let result: Result<i32, &str> = Err("permission error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe(UvsReason::permission_error());

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 103);
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("permission error"));
}

#[test]
fn test_owe_external() {
    let result: Result<i32, &str> = Err("external error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe(UvsReason::external_error());

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 301);
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("external error"));
}

#[test]
fn test_owe_system() {
    let result: Result<i32, &str> = Err("system error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe_sys();

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 201);
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("system error"));
}

#[test]
fn test_error_code_implementation() {
    let result: Result<i32, &str> = Err("test error");
    let converted: Result<i32, StructError<UvsReason>> = result.owe(UvsReason::business_error());

    assert_eq!(converted.as_ref().unwrap_err().error_code(), 101);
    assert!(converted
        .as_ref()
        .unwrap_err()
        .detail()
        .as_ref()
        .unwrap()
        .contains("test error"));
}

#[test]
fn test_owe_source_preserves_real_source() {
    let result: Result<(), std::io::Error> = Err(std::io::Error::other("disk offline"));

    let converted: Result<(), StructError<UvsReason>> = result.owe_sys_source();
    let error = converted.unwrap_err();

    assert_eq!(error.error_code(), 201);
    assert_eq!(
        StdError::source(&error).unwrap().to_string(),
        "disk offline"
    );
    assert_eq!(error.root_cause().unwrap().to_string(), "disk offline");
}
