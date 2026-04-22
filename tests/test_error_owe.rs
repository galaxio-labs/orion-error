#![allow(deprecated)]

use orion_error::compat_prelude::ErrorOweBase;
use orion_error::conversion::IntoAs;
use orion_error::ErrorCode;
use orion_error::{StructError, UvsReason};

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
fn test_into_as_preserves_real_source() {
    let result: Result<(), std::io::Error> = Err(std::io::Error::other("disk offline"));

    let converted: Result<(), StructError<UvsReason>> =
        result.into_as(UvsReason::system_error(), "disk offline");
    let error = converted.unwrap_err();

    assert_eq!(error.error_code(), 201);
    assert_eq!(error.source_ref().unwrap().to_string(), "disk offline");
    assert_eq!(error.root_cause().unwrap().to_string(), "disk offline");
}
