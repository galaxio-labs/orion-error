use derive_more::From;
use orion_error::prelude::*;
use orion_error::protocol::DefaultExposurePolicy;
use orion_error::{
    cli::print_error, conversion::ToStructError, OperationContext, OrionError, StructError,
    UvsReason,
};
use orion_error::conversion::Upcast;

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum ParseReason {
    #[orion_error(identity = "biz.order_invalid_text")]
    InvalidText,
    #[orion_error(identity = "biz.order_invalid_amount")]
    InvalidAmount,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum UserReason {
    #[orion_error(identity = "biz.user_not_found")]
    NotFound,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum StoreReason {
    #[orion_error(identity = "sys.storage_full")]
    StorageFull,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum OrderReason {
    #[orion_error(identity = "biz.order_invalid")]
    InvalidOrder,
    #[orion_error(identity = "biz.user_not_found")]
    UserNotFound,
    #[orion_error(identity = "biz.insufficient_funds")]
    InsufficientFunds,
    #[orion_error(identity = "sys.storage_full")]
    StorageFull,
    #[orion_error(transparent)]
    Uvs(UvsReason),
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
        let ctx = OperationContext::doing("place_order")
            .with_field("user_id", user_id.to_string())
            .with_field("order.raw", raw_order)
            .with_meta("component.name", "order_service")
            .with_meta("trace.secret", "prod-token");

        let draft = Self::parse_order(user_id, amount, raw_order)
            .doing("parse order")
            .with_context(&ctx)
            .upcast()?;

        Self::load_user(draft.user_id)
            .doing("load user")
            .with_context(&ctx)
            .upcast()?;

        Self::ensure_balance(draft.amount)
            .doing("check balance")
            .with_context(&ctx)?;

        Self::save_order(&draft)
            .doing("save order")
            .with_context(&ctx)
            .upcast()?;

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

fn print_protocol_views(err: &OrderError) {
    print_error(err);
    println!();
    let exposure_policy = DefaultExposurePolicy;
    println!(
        "{}",
        err.exposure_snapshot(&exposure_policy).render_user_debug()
    );
}

fn run_case(name: &str, user_id: u32, amount: u64, raw_order: &str) {
    println!("\n== {name} ==");
    match OrderService::place_order(user_id, amount, raw_order) {
        Ok(order) => println!(
            "created order: user={} amount={}",
            order.user_id, order.amount
        ),
        Err(err) => print_protocol_views(&err),
    }
}

fn main() {
    run_case("invalid input", 42, 100, "");
    run_case("user missing", 7, 100, "coffee");
    run_case("insufficient funds", 42, 500, "coffee");
    run_case("storage full", 42, 100, "overflow");
}
