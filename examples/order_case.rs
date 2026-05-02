//! orion-error 三层错误治理示例
//! Three-layer error governance example
//!
//! 架构：Parse → User/Store → Order（下层 → 上层）
//! Arch: Parse → User/Store → Order（lower → upper layer）
//!
//! 1. 各层定义自己的 DomainReason
//!    Each layer defines its own DomainReason
//! 2. 下层错误通过 conv_err() 收敛到上层
//!    Lower errors converge to upper via conv_err()
//! 3. 边界输出使用 exposure_snapshot()
//!    Boundary output via exposure_snapshot()

use derive_more::From;
use orion_error::prelude::*;
use orion_error::protocol::DefaultExposurePolicy;
use orion_error::{
    cli::print_error, conversion::ToStructError, OperationContext, OrionError, StructError,
    UvsReason,
};
// conv_err 通过 prelude 可用 / conv_err is available via prelude

// ── 下层 Reason：解析层 / Lower layer: parsing ──
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

// ── 上层 Reason：服务层 / Upper layer: service ──
// 下层 Reason 通过 From 收敛到此层
// Lower-layer Reasons converge via From impls
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

// ── From 实现：下层 Reason → 上层 Reason / Lower → Upper ──
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

// ── 类型别名 / Type aliases ──
type ParseError = StructError<ParseReason>;
type UserError = StructError<UserReason>;
type StoreError = StructError<StoreReason>;
type OrderError = StructError<OrderReason>;

// ── 业务数据 / Business data ──
#[derive(Debug, Clone)]
struct OrderDraft {
    user_id: u32,
    amount: u64,
    item: String,
}

// ── 服务层：协调各下层，收敛错误类型 / Orchestration + error convergence ──
struct OrderService;

impl OrderService {
    /// 下单入口 / Place order entry point
    /// 逐层调用下层函数，用 conv_err() 收敛错误类型
    /// Calls lower-layer functions, converges errors via conv_err()
    fn place_order(user_id: u32, amount: u64, raw_order: &str) -> Result<OrderDraft, OrderError> {
        let ctx = OperationContext::doing("place_order")
            .with_field("user_id", user_id.to_string())
            .with_field("order.raw", raw_order)
            .with_meta("component.name", "order_service")
            .with_meta("trace.secret", "prod-token");

        // 调用解析层 / Call parsing layer → conv_err 收敛到 OrderReason
        let draft = Self::parse_order(user_id, amount, raw_order)
            .doing("parse order")
            .with_context(&ctx)
            .conv_err()?;

        Self::load_user(draft.user_id)
            .doing("load user")
            .with_context(&ctx)
            .conv_err()?;

        Self::ensure_balance(draft.amount)
            .doing("check balance")
            .with_context(&ctx)?;

        Self::save_order(&draft)
            .doing("save order")
            .with_context(&ctx)
            .conv_err()?;

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
        _ => StoreReason::system_error()
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

// ── 边界输出：打印 + protocol projection / Boundary output ──
fn print_protocol_views(err: &OrderError) {
    // 人类可读诊断 / Human-readable diagnostics
    print_error(err);
    println!();
    // 协议投影 / Protocol projection via DefaultExposurePolicy
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
    // 4 个测试场景：非法输入 / 用户不存在 / 余额不足 / 存储满
    // 4 test cases: invalid input / user missing / insufficient funds / storage full
    run_case("invalid input", 42, 100, "");
    run_case("user missing", 7, 100, "coffee");
    run_case("insufficient funds", 42, 500, "coffee");
    run_case("storage full", 42, 100, "overflow");
}
