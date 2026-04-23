use derive_more::From;
use orion_error::{
    DefaultErrorPolicy, ErrorCategory, ErrorCode, ErrorConv, ErrorIdentityProvider, ErrorWith,
    OperationContext, StructError, ToStructError, UvsReason,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error, From)]
enum ParseReason {
    #[error("invalid order text")]
    InvalidText,
    #[error("invalid amount")]
    InvalidAmount,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl ErrorCode for ParseReason {
    fn error_code(&self) -> i32 {
        match self {
            Self::InvalidText => 1001,
            Self::InvalidAmount => 1002,
            Self::Uvs(reason) => reason.error_code(),
        }
    }
}

impl ErrorIdentityProvider for ParseReason {
    fn stable_code(&self) -> &'static str {
        match self {
            Self::InvalidText => "biz.order_invalid_text",
            Self::InvalidAmount => "biz.order_invalid_amount",
            Self::Uvs(reason) => reason.stable_code(),
        }
    }

    fn error_category(&self) -> ErrorCategory {
        match self {
            Self::InvalidText | Self::InvalidAmount => ErrorCategory::Biz,
            Self::Uvs(reason) => reason.error_category(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Error, From)]
enum UserReason {
    #[error("user not found")]
    NotFound,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl ErrorCode for UserReason {
    fn error_code(&self) -> i32 {
        match self {
            Self::NotFound => 1101,
            Self::Uvs(reason) => reason.error_code(),
        }
    }
}

impl ErrorIdentityProvider for UserReason {
    fn stable_code(&self) -> &'static str {
        match self {
            Self::NotFound => "biz.user_not_found",
            Self::Uvs(reason) => reason.stable_code(),
        }
    }

    fn error_category(&self) -> ErrorCategory {
        match self {
            Self::NotFound => ErrorCategory::Biz,
            Self::Uvs(reason) => reason.error_category(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Error, From)]
enum StoreReason {
    #[error("storage full")]
    StorageFull,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl ErrorCode for StoreReason {
    fn error_code(&self) -> i32 {
        match self {
            Self::StorageFull => 2101,
            Self::Uvs(reason) => reason.error_code(),
        }
    }
}

impl ErrorIdentityProvider for StoreReason {
    fn stable_code(&self) -> &'static str {
        match self {
            Self::StorageFull => "sys.storage_full",
            Self::Uvs(reason) => reason.stable_code(),
        }
    }

    fn error_category(&self) -> ErrorCategory {
        match self {
            Self::StorageFull => ErrorCategory::Sys,
            Self::Uvs(reason) => reason.error_category(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Error, From)]
enum OrderReason {
    #[error("invalid order")]
    InvalidOrder,
    #[error("user not found")]
    UserNotFound,
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("storage full")]
    StorageFull,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl ErrorCode for OrderReason {
    fn error_code(&self) -> i32 {
        match self {
            Self::InvalidOrder => 3001,
            Self::UserNotFound => 3002,
            Self::InsufficientFunds => 3003,
            Self::StorageFull => 3004,
            Self::Uvs(reason) => reason.error_code(),
        }
    }
}

impl ErrorIdentityProvider for OrderReason {
    fn stable_code(&self) -> &'static str {
        match self {
            Self::InvalidOrder => "biz.order_invalid",
            Self::UserNotFound => "biz.user_not_found",
            Self::InsufficientFunds => "biz.insufficient_funds",
            Self::StorageFull => "sys.storage_full",
            Self::Uvs(reason) => reason.stable_code(),
        }
    }

    fn error_category(&self) -> ErrorCategory {
        match self {
            Self::InvalidOrder | Self::UserNotFound | Self::InsufficientFunds => ErrorCategory::Biz,
            Self::StorageFull => ErrorCategory::Sys,
            Self::Uvs(reason) => reason.error_category(),
        }
    }
}

impl From<ParseReason> for OrderReason {
    fn from(value: ParseReason) -> Self {
        match value {
            ParseReason::InvalidText | ParseReason::InvalidAmount => Self::InvalidOrder,
            ParseReason::Uvs(reason) => Self::Uvs(reason),
        }
    }
}

impl From<UserReason> for OrderReason {
    fn from(value: UserReason) -> Self {
        match value {
            UserReason::NotFound => Self::UserNotFound,
            UserReason::Uvs(reason) => Self::Uvs(reason),
        }
    }
}

impl From<StoreReason> for OrderReason {
    fn from(value: StoreReason) -> Self {
        match value {
            StoreReason::StorageFull => Self::StorageFull,
            StoreReason::Uvs(reason) => Self::Uvs(reason),
        }
    }
}

type ParseError = StructError<ParseReason>;
type UserError = StructError<UserReason>;
type StoreError = StructError<StoreReason>;
type OrderError = StructError<OrderReason>;

#[derive(Debug, Clone)]
struct OrderDraft {
    user_id: u32,
    amount: u64,
    item: String,
}

struct OrderService;

impl OrderService {
    fn place_order(user_id: u32, amount: u64, raw_order: &str) -> Result<OrderDraft, OrderError> {
        let mut ctx = OperationContext::doing("place_order");
        ctx.record_field("user_id", user_id.to_string());
        ctx.record_field("order.raw", raw_order);
        ctx.record_meta("component.name", "order_service");
        ctx.record_meta("trace.secret", "prod-token");

        let draft = Self::parse_order(user_id, amount, raw_order)
            .doing("parse order")
            .with_context(&ctx)
            .err_conv()?;

        Self::load_user(draft.user_id)
            .doing("load user")
            .with_context(&ctx)
            .err_conv()?;

        Self::ensure_balance(draft.amount)
            .doing("check balance")
            .with_context(&ctx)?;

        Self::save_order(&draft)
            .doing("save order")
            .with_context(&ctx)
            .err_conv()?;

        Ok(draft)
    }

    fn parse_order(user_id: u32, amount: u64, raw_order: &str) -> Result<OrderDraft, ParseError> {
        if raw_order.trim().is_empty() {
            return Err(ParseReason::InvalidText
                .to_err()
                .with_detail("order text must not be empty"));
        }

        if amount == 0 {
            return Err(ParseReason::InvalidAmount
                .to_err()
                .with_detail("amount must be greater than zero"));
        }

        Ok(OrderDraft {
            user_id,
            amount,
            item: raw_order.trim().to_string(),
        })
    }

    fn load_user(user_id: u32) -> Result<(), UserError> {
        if user_id == 42 {
            Ok(())
        } else {
            Err(UserReason::NotFound
                .to_err()
                .with_detail(format!("user {user_id} does not exist")))
        }
    }

    fn ensure_balance(amount: u64) -> Result<(), OrderError> {
        let balance = 300;
        if amount > balance {
            Err(OrderReason::InsufficientFunds
                .to_err()
                .with_detail(format!("balance={balance}, required={amount}")))
        } else {
            Ok(())
        }
    }

    fn save_order(draft: &OrderDraft) -> Result<(), StoreError> {
        persist_order(draft.item.as_str())
    }
}

fn persist_order(item: &str) -> Result<(), StoreError> {
    write_impl(item).map_err(|err| match err.kind() {
        std::io::ErrorKind::OutOfMemory => StoreReason::StorageFull
            .to_err()
            .with_detail("storage quota exceeded")
            .with_source(err),
        _ => StoreReason::from(UvsReason::system_error())
            .to_err()
            .with_detail("write order record failed")
            .with_source(err),
    })
}

fn write_impl(item: &str) -> Result<(), std::io::Error> {
    if item == "overflow" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::OutOfMemory,
            "storage full",
        ));
    }
    Ok(())
}

fn print_v3_views(err: &OrderError) {
    let policy = DefaultErrorPolicy;
    println!("{}", err.render_user_debug(&policy));
}

fn run_case(name: &str, user_id: u32, amount: u64, raw_order: &str) {
    println!("\n== {name} ==");
    match OrderService::place_order(user_id, amount, raw_order) {
        Ok(order) => println!(
            "created order: user={} amount={}",
            order.user_id, order.amount
        ),
        Err(err) => print_v3_views(&err),
    }
}

fn main() {
    run_case("invalid input", 42, 100, "");
    run_case("user missing", 7, 100, "coffee");
    run_case("insufficient funds", 42, 500, "coffee");
    run_case("storage full", 42, 100, "overflow");
}
