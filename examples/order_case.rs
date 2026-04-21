// 创造一个客户订单的用例,展示orion-error。
// 用例包括: order_service, order_store, order_txt 相关概念。
// 产生错语的原因可能： order_txt 格式错语， 客户帐户资金不足、 order_store 的空间不足等

use derive_more::From;
use orion_error::{
    print_error, ContextRecord, ErrorCode, ErrorConv, ErrorWith, OperationContext, RedactPolicy,
    RenderMode, StructError, UvsReason,
};
use std::sync::atomic::Ordering;
use thiserror::Error;

// ========== 领域错误定义 ==========
#[derive(Debug, PartialEq, Clone, Error, From)]
pub enum OrderReason {
    #[error("format error")]
    FormatError,
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("storage full")]
    StorageFull,
    #[error("user not found")]
    UserNotFound,
    #[error("{0}")]
    Uvs(UvsReason),
}
impl ErrorCode for OrderReason {
    fn error_code(&self) -> i32 {
        match self {
            Self::Uvs(uvs_reason) => uvs_reason.error_code(),
            _ => 500,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Error, From)]
pub enum StoreReason {
    #[error("storage full")]
    StorageFull,
    #[error("{0}")]
    Uvs(UvsReason),
}
impl ErrorCode for StoreReason {
    fn error_code(&self) -> i32 {
        match self {
            Self::Uvs(uvs_reason) => uvs_reason.error_code(),
            _ => 500,
        }
    }
}

#[derive(Debug, Error, PartialEq, Clone, From)]
pub enum ParseReason {
    #[error("format error")]
    FormatError,
    #[error("{0}")]
    Uvs(UvsReason),
}
impl ErrorCode for ParseReason {
    fn error_code(&self) -> i32 {
        match self {
            ParseReason::FormatError => 400,
            ParseReason::Uvs(uvs_reason) => uvs_reason.error_code(),
        }
    }
}

impl From<ParseReason> for OrderReason {
    fn from(value: ParseReason) -> Self {
        match value {
            ParseReason::FormatError => Self::FormatError,
            ParseReason::Uvs(uvs_reason) => Self::Uvs(uvs_reason),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Error, From)]
pub enum UserReason {
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl From<UserReason> for OrderReason {
    fn from(value: UserReason) -> Self {
        match value {
            UserReason::NotFound => Self::UserNotFound,
            UserReason::Uvs(uvs_reason) => Self::Uvs(uvs_reason),
        }
    }
}

impl From<StoreReason> for OrderReason {
    fn from(value: StoreReason) -> Self {
        match value {
            StoreReason::StorageFull => Self::StorageFull,
            StoreReason::Uvs(uvs_reason) => Self::Uvs(uvs_reason),
        }
    }
}
pub type OrderError = StructError<OrderReason>;
pub type StoreError = StructError<StoreReason>;
pub type ParseError = StructError<ParseReason>;
pub type UserError = StructError<UserReason>;

struct ExampleRedactPolicy;

impl RedactPolicy for ExampleRedactPolicy {
    fn redact_key(&self, key: &str) -> bool {
        matches!(key, "order" | "config.secret")
    }

    fn redact_value(&self, _key: Option<&str>, _value: &str) -> Option<String> {
        Some("<redacted>".to_string())
    }
}

// ========== 数据层 ==========
pub mod storage {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };

    use super::*;

    #[derive(Clone)]
    pub struct Order {
        pub user_id: u32,
        pub amount: f64,
    }

    pub static STORAGE_CAPACITY: AtomicUsize = AtomicUsize::new(2);
    static ORDERS: Mutex<Vec<Order>> = Mutex::new(Vec::new());
    pub fn save(order: Order) -> Result<(), StoreError> {
        save_db_impl(order).map_err(|e| match e.kind() {
            std::io::ErrorKind::OutOfMemory => StructError::from(StoreReason::StorageFull)
                .with_detail("storage capacity exceeded")
                .with_std_source(e),
            _ => StructError::from(StoreReason::from(UvsReason::system_error()))
                .with_detail("persist order failed")
                .with_std_source(e),
        })
    }

    fn save_db_impl(order: Order) -> Result<(), std::io::Error> {
        let capacity = STORAGE_CAPACITY.load(Ordering::Relaxed);
        let mut orders = ORDERS
            .lock()
            .map_err(|_| std::io::Error::other("Failed to lock orders mutex"))?;

        if orders.len() >= capacity {
            return Err(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "Storage capacity exceeded",
            ));
        }
        orders.push(order);
        Ok(())
    }
}

// ========== 业务逻辑层 ==========
struct OrderService;

impl OrderService {
    /// 创建订单完整流程
    pub fn place_order(
        user_id: u32,
        amount: f64,
        order_txt: &str,
    ) -> Result<storage::Order, OrderError> {
        let mut ctx = OperationContext::doing("place_order");
        ctx.record("order", order_txt);
        ctx.record_meta("component.name", "order_service");
        ctx.record_meta("config.secret", "/prod/orders/api-token");
        let order = Self::parse_order(order_txt, amount)
            .doing("解析订单")
            .with(&ctx)
            .err_conv()?;

        Self::validate_funds(user_id, order.amount)
            .doing("验证资金")
            .with(&ctx)?;

        let order = storage::Order { user_id, amount };
        Self::save_order(order).doing("保存订单").with(&ctx)
    }

    fn parse_order(txt: &str, amount: f64) -> Result<storage::Order, ParseError> {
        if txt.is_empty() {
            return Err(StructError::builder(ParseReason::FormatError)
                .detail("订单文本不能为空")
                .context(
                    OperationContext::doing("parse order text")
                        .with_meta("config.kind", "order_txt")
                        .with_meta("parse.field", "order_txt"),
                )
                .finish());
        }

        // 模拟解析逻辑 - 验证金额
        if amount <= 0.0 {
            return Err(StructError::builder(ParseReason::FormatError)
                .detail("订单金额必须大于零")
                .context(
                    OperationContext::doing("parse order amount")
                        .with_meta("parse.field", "amount"),
                )
                .finish());
        }

        Ok(storage::Order {
            user_id: 123,
            amount,
        })
    }

    fn validate_funds(user_id: u32, amount: f64) -> Result<(), OrderError> {
        //let balance = Self::get_balance(user_id).map_err(stc_err_conv)?;
        let balance = Self::get_balance(user_id).err_conv()?;

        if balance < amount {
            Err(StructError::builder(OrderReason::InsufficientFunds)
                .detail(format!("当前余额：{balance}，需要：{amount}"))
                .finish())
        } else {
            Ok(())
        }
    }

    fn get_balance(user_id: u32) -> Result<f64, UserError> {
        if user_id != 123 {
            Err(StructError::builder(UserReason::NotFound)
                .detail(format!("uid:{user_id}"))
                .finish())
        } else {
            Ok(500.0)
        }
    }

    fn save_order(order: storage::Order) -> Result<storage::Order, OrderError> {
        storage::save(order.clone()).err_conv()?;
        Ok(order)
    }
}

// ========== 展示错误处理 ==========

fn print_verbose_report(err: &OrderError) {
    println!("verbose report:\n{}", err.render(RenderMode::Verbose));
    println!(
        "redacted verbose report:\n{}",
        err.render_redacted(RenderMode::Verbose, &ExampleRedactPolicy)
    );
}

fn main() {
    // 测试用例 1: 空订单文本
    let case1 = OrderService::place_order(123, 200.0, "");
    if let Err(e) = case1 {
        print_error(&e);
        println!("root metadata: {:?}", e.context_metadata().as_map());
        let report = e.report();
        println!("report path: {:?}", report.path);
        print_verbose_report(&e);
    }

    // 测试用例 2: 用户不存在
    let case2 = OrderService::place_order(456, 200.0, "valid_order");
    if let Err(e) = case2 {
        print_error(&e);
    }

    // 测试用例 3: 余额不足
    let case3 = OrderService::place_order(123, 600.0, "valid_order");
    if let Err(e) = case3 {
        print_error(&e);
    }

    // 测试用例 4: 存储空间不足
    storage::STORAGE_CAPACITY.store(0, Ordering::Relaxed);
    let case4 = OrderService::place_order(123, 200.0, "valid_order");
    if let Err(e) = case4 {
        print_error(&e);
        if let Some(frame) = e.source_frames().first() {
            println!("first source metadata: {:?}", frame.metadata.as_map());
        }
    }

    // 测试用例 5: 金额验证失败
    let case5 = OrderService::place_order(123, 0.0, "negative_amount");
    if let Err(e) = case5 {
        print_error(&e);
        println!("root metadata: {:?}", e.context_metadata().as_map());
    }
}
