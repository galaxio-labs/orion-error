# 错误处理最佳实践

> 历史设计说明：
> 本页保存的是早期最佳实践草稿，包含大量概念性示例和非当前源码接口的伪代码，不应直接视为 `orion-error 0.6.x / V1 API` 的使用手册。
> 文中的自定义 `ContextualError`、扩展 trait、上下文模型和包装方式，很多并不是当前 crate 的公开 API。
> 当前推荐路径请以 [docs/README.md](../README.md)、[使用教程](../tutorial.md)、[V1 Migration Checklist](../v1-migration-checklist.md) 和 [V1 修复与评审基线](../v1-fix-and-review-plan.md) 为准。

## 当前 V1 最小实践

如果你要按当前 `orion-error 0.6.x / V1 API` 写新代码，先记住这几条：

- 普通错误第一次进入结构化体系：优先 `into_as(...)`
- 已结构化错误跨层传播：优先 `err_conv()` 或 `wrap_as(...)`
- 普通 source：`with_std_source(...)`
- 结构化 source：`with_struct_source(...)`
- 上下文命名优先 `doing(...)` / `at(...)`
- `owe(...)`、`err_wrap(...)`、`want(...)`、`with_source(...)` 只作为 compat 路径；`owe_*()` 已移除

下面的大段代码和类型定义是历史草稿与伪代码，不是当前 crate 已提供的 API，也不建议直接复制到新实现里。

use std::collections::HashMap;
use serde_json::Value;

pub trait ErrorContextExt<T, E> {
    fn with_detail(self, detail: String) -> Result<T, ContextualError<E>>;
    fn with_tag(self, tag: &str) -> Result<T, ContextualError<E>>;
    fn with_metric(self, metric_name: &str, value: f64) -> Result<T, ContextualError<E>>;
    fn with_user_context(self, user_id: String, user_info: HashMap<String, String>) -> Result<T, ContextualError<E>>;
    fn with_request_context(self, request_id: String, trace_id: String) -> Result<T, ContextualError<E>>;
    fn with_system_context(self, system_info: HashMap<String, Value>) -> Result<T, ContextualError<E>>;
}

impl<T, E: std::fmt::Debug> ErrorContextExt<T, E> for Result<T, E> {
    fn with_detail(self, detail: String) -> Result<T, ContextualError<E>> {
        self.map_err(|e| ContextualError::new(e).with_detail(detail))
    }
    
    fn with_tag(self, tag: &str) -> Result<T, ContextualError<E>> {
        self.map_err(|e| ContextualError::new(e).with_tag(tag))
    }
    
    fn with_metric(self, metric_name: &str, value: f64) -> Result<T, ContextualError<E>> {
        self.map_err(|e| ContextualError::new(e).with_metric(metric_name, value))
    }
    
    fn with_user_context(self, user_id: String, user_info: HashMap<String, String>) -> Result<T, ContextualError<E>> {
        self.map_err(|e| ContextualError::new(e).with_user_context(user_id, user_info))
    }
    
    fn with_request_context(self, request_id: String, trace_id: String) -> Result<T, ContextualError<E>> {
        self.map_err(|e| ContextualError::new(e).with_request_context(request_id, trace_id))
    }
    
    fn with_system_context(self, system_info: HashMap<String, Value>) -> Result<T, ContextualError<E>> {
        self.map_err(|e| ContextualError::new(e).with_system_context(system_info))
    }
}

#[derive(Debug, thiserror::Error)]
pub struct ContextualError<E> {
    #[source]
    source: E,
    detail: Option<String>,
    tags: Vec<String>,
    metrics: HashMap<String, f64>,
    user_context: Option<UserContext>,
    request_context: Option<RequestContext>,
    system_context: Option<HashMap<String, Value>>,
    timestamp: SystemTime,
}

impl<E: std::fmt::Debug> ContextualError<E> {
    pub fn new(source: E) -> Self {
        Self {
            source,
            detail: None,
            tags: Vec::new(),
            metrics: HashMap::new(),
            user_context: None,
            request_context: None,
            system_context: None,
            timestamp: SystemTime::now(),
        }
    }
    
    pub fn with_detail(mut self, detail: String) -> Self {
        self.detail = Some(detail);
        self
    }
    
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }
    
    pub fn with_metric(mut self, metric_name: &str, value: f64) -> Self {
        self.metrics.insert(metric_name.to_string(), value);
        self
    }
    
    pub fn with_user_context(mut self, user_id: String, user_info: HashMap<String, String>) -> Self {
        self.user_context = Some(UserContext {
            user_id,
            info: user_info,
        });
        self
    }
    
    pub fn with_request_context(mut self, request_id: String, trace_id: String) -> Self {
        self.request_context = Some(RequestContext {
            request_id,
            trace_id,
        });
        self
    }
    
    pub fn with_system_context(mut self, system_info: HashMap<String, Value>) -> Self {
        self.system_context = Some(system_info);
        self
    }
}

#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub info: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub trace_id: String,
}
```

#### 使用示例

以下示例是历史上下文包装草稿，不对应当前 V1 推荐写法：

```rust
pub fn process_order(order: &Order) -> Result<OrderResponse, BusinessError> {
    validate_order(order)
        .with_detail(format!("订单ID: {}, 金额: {}", order.id, order.amount))
        .with_tag("order_validation")
        .with_metric("validation_attempt", 1.0)
        .with_user_context(
            order.user_id.clone(),
            HashMap::from([
                ("user_type".to_string(), "premium".to_string()),
                ("tier".to_string(), "gold".to_string()),
            ])
        )
        .with_request_context(
            generate_request_id(),
            generate_trace_id(),
        )?;

    let inventory_result = check_inventory(&order.items)
        .with_detail("检查商品库存可用性")
        .with_tag("inventory_check")
        .with_metric("inventory_check_count", 1.0)?;

    let payment_result = process_payment(&order.payment_info)
        .with_detail(format!("处理支付，金额: {}", order.amount))
        .with_tag("payment_processing")
        .with_metric("payment_amount", order.amount)
        .with_system_context(HashMap::from([
            ("payment_gateway".to_string(), Value::String("stripe".to_string())),
            ("environment".to_string(), Value::String("production".to_string())),
        ]))?;

    Ok(OrderResponse::new(order.id, "success"))
}
```

### 上下文信息层次

本节的 `ErrorContext`、`BusinessContext`、`TechnicalContext` 等类型是历史建模示意，不是当前 crate 的公开结构。

#### 推荐的上下文层次结构
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    // 核心识别信息
    pub trace_id: String,
    pub span_id: String,
    pub request_id: String,
    
    // 时间信息
    pub timestamp: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    
    // 业务上下文
    pub business_context: BusinessContext,
    
    // 技术上下文
    pub technical_context: TechnicalContext,
    
    // 用户上下文
    pub user_context: UserContext,
    
    // 环境上下文
    pub environment_context: EnvironmentContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessContext {
    pub operation: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub business_rules: Vec<String>,
    pub custom_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalContext {
    pub function_name: String,
    pub line_number: Option<u32>,
    pub module_path: String,
    pub thread_id: String,
    pub memory_usage: Option<u64>,
    pub cpu_usage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: Option<String>,
    pub user_type: Option<String>,
    pub session_id: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentContext {
    pub service_name: String,
    pub service_version: String,
    pub environment: String,
    pub host_name: String,
    pub region: String,
    pub deployment_version: String,
}
```

#### 上下文收集策略
```rust
pub struct ContextCollector {
    pub business_context: BusinessContext,
    pub technical_context: TechnicalContext,
    pub user_context: UserContext,
    pub environment_context: EnvironmentContext,
}

impl ContextCollector {
    pub fn new(operation: &str, entity_type: &str) -> Self {
        Self {
            business_context: BusinessContext {
                operation: operation.to_string(),
                entity_type: entity_type.to_string(),
                entity_id: None,
                business_rules: Vec::new(),
                custom_fields: HashMap::new(),
            },
            technical_context: TechnicalContext {
                function_name: String::new(),
                line_number: None,
                module_path: module_path!().to_string(),
                thread_id: thread_id().to_string(),
                memory_usage: None,
                cpu_usage: None,
            },
            user_context: UserContext {
                user_id: None,
                user_type: None,
                session_id: None,
                client_ip: None,
                user_agent: None,
                permissions: Vec::new(),
            },
            environment_context: EnvironmentContext {
                service_name: env!("CARGO_PKG_NAME").to_string(),
                service_version: env!("CARGO_PKG_VERSION").to_string(),
                environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
                host_name: hostname::get().unwrap_or_else(|_| "unknown".into()).to_string_lossy().to_string(),
                region: env::var("REGION").unwrap_or_else(|_| "local".to_string()),
                deployment_version: env::var("DEPLOYMENT_VERSION").unwrap_or_else(|_| "dev".to_string()),
            },
        }
    }
    
    pub fn with_entity_id(mut self, entity_id: String) -> Self {
        self.business_context.entity_id = Some(entity_id);
        self
    }
    
    pub fn with_business_rule(mut self, rule: String) -> Self {
        self.business_context.business_rules.push(rule);
        self
    }
    
    pub fn with_function_info(mut self, function: &str, line: Option<u32>) -> Self {
        self.technical_context.function_name = function.to_string();
        self.technical_context.line_number = line;
        self
    }
    
    pub fn with_user_info(mut self, user_id: String, user_type: String) -> Self {
        self.user_context.user_id = Some(user_id);
        self.user_context.user_type = Some(user_type);
        self
    }
    
    pub fn with_request_info(mut self, session_id: String, client_ip: String, user_agent: String) -> Self {
        self.user_context.session_id = Some(session_id);
        self.user_context.client_ip = Some(client_ip);
        self.user_context.user_agent = Some(user_agent);
        self
    }
    
    pub fn collect_system_info(mut self) -> Self {
        // 收集内存使用情况
        if let Ok(memory) = sysinfo::System::new_all().total_memory() {
            self.technical_context.memory_usage = Some(memory);
        }
        
        // 收集CPU使用情况
        if let Ok(cpu_usage) = sysinfo::System::new_all().global_cpu_usage() {
            self.technical_context.cpu_usage = Some(cpu_usage as f64);
        }
        
        self
    }
    
    pub fn build_error_context(self, trace_id: String, span_id: String, request_id: String) -> ErrorContext {
        ErrorContext {
            trace_id,
            span_id,
            request_id,
            timestamp: Utc::now(),
            duration_ms: None,
            business_context: self.business_context,
            technical_context: self.technical_context,
            user_context: self.user_context,
            environment_context: self.environment_context,
        }
    }
}
```

## 错误转换规范

### 转换原则

#### 语义保持原则
错误转换过程中必须保持原始错误的语义信息，确保转换后的错误类型能够准确反映问题的本质。

```rust
// Good: 保留原始错误信息
#[derive(Debug, thiserror::Error)]
pub enum OrderError {
    #[error("订单处理失败: {source}")]
    ProcessingFailed { source: DatabaseError },
    
    #[error("支付处理失败: {source}")]
    PaymentFailed { source: PaymentGatewayError },
    
    #[error("库存检查失败: {source}")]
    InventoryCheckFailed { source: InventoryServiceError },
}

impl From<DatabaseError> for OrderError {
    fn from(error: DatabaseError) -> Self {
        OrderError::ProcessingFailed { source: error }
    }
}

// Bad: 丢失错误上下文
impl From<DatabaseError> for OrderError {
    fn from(_error: DatabaseError) -> Self {
        OrderError::ProcessingFailed {
            // 丢失了原始错误信息
            source: DatabaseError::Unknown("unknown error".to_string()),
        }
    }
}
```

#### 信息完整原则
转换过程中不应丢失任何重要的错误信息，包括错误类型、错误代码、错误消息等。

```rust
// Good: 保留完整错误信息
pub struct EnhancedError<E> {
    source: E,
    context: ErrorContext,
    timestamp: SystemTime,
    retry_count: u32,
    custom_fields: HashMap<String, Value>,
}

impl<E> EnhancedError<E> {
    pub fn new(source: E, context: ErrorContext) -> Self {
        Self {
            source,
            context,
            timestamp: SystemTime::now(),
            retry_count: 0,
            custom_fields: HashMap::new(),
        }
    }
    
    pub fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
    
    pub fn with_custom_field(mut self, key: String, value: Value) -> Self {
        self.custom_fields.insert(key, value);
        self
    }
}

impl From<DatabaseError> for EnhancedError<OrderError> {
    fn from(db_error: DatabaseError) -> Self {
        let context = ErrorContext::new("order_processing", "database");
        let order_error = match db_error {
            DatabaseError::ConnectionFailed => OrderError::ServiceUnavailable {
                service: "database".to_string(),
                message: "无法连接到数据库".to_string(),
            },
            DatabaseError::Timeout => OrderError::Timeout {
                operation: "database_query".to_string(),
                timeout_ms: 5000,
            },
            DatabaseError::RecordNotFound { table, id } => OrderError::NotFound {
                resource: format!("{}:{}", table, id),
            },
            DatabaseError::ConstraintViolation { constraint, table } => OrderError::ValidationFailed {
                field: constraint,
                message: format!("{}表数据约束违反", table),
            },
            DatabaseError::Unknown(message) => OrderError::InternalError {
                message: format!("数据库错误: {}", message),
            },
        };
        
        EnhancedError::new(order_error, context)
            .with_custom_field("database_error_type".to_string(), Value::String(format!("{:?}", db_error)))
            .with_custom_field("original_error_message".to_string(), Value::String(db_error.to_string()))
    }
}

// Bad: 信息不完整
impl From<DatabaseError> for OrderError {
    fn from(db_error: DatabaseError) -> Self {
        match db_error {
            DatabaseError::ConnectionFailed => OrderError::InternalError {
                message: "服务暂时不可用".to_string(), // 失去了具体的数据库连接失败信息
            },
            DatabaseError::Timeout => OrderError::InternalError {
                message: "请求超时".to_string(), // 失去了具体是哪个操作超时
            },
            // 其他转换...
        }
    }
}
```

### 转换模式

#### 直接映射模式
```rust
// 适用于错误类型语义相近的情况
impl From<ValidationError> for OrderError {
    fn from(validation_error: ValidationError) -> Self {
        match validation_error.field.as_str() {
            "email" => OrderError::InvalidEmail {
                email: validation_error.value,
                reason: validation_error.reason,
            },
            "phone" => OrderError::InvalidPhone {
                phone: validation_error.value,
                reason: validation_error.reason,
            },
            "amount" => OrderError::InvalidAmount {
                amount: validation_error.value.parse().unwrap_or(0.0),
                reason: validation_error.reason,
            },
            _ => OrderError::InvalidField {
                field: validation_error.field,
                value: validation_error.value,
                reason: validation_error.reason,
            },
        }
    }
}
```

#### 包装转换模式
```rust
// 适用于需要保留原始错误的情况
#[derive(Debug, thiserror::Error)]
pub enum OrderError {
    #[error("数据库操作失败: {source}")]
    DatabaseError { source: DatabaseError, operation: String },
    
    #[error("外部服务调用失败: {source}")]
    ExternalServiceError { source: ExternalServiceError, service_name: String },
    
    #[error("支付处理失败: {source}")]
    PaymentError { source: PaymentGatewayError, payment_method: String },
}

impl From<DatabaseError> for OrderError {
    fn from(db_error: DatabaseError) -> Self {
        OrderError::DatabaseError {
            source: db_error,
            operation: "unknown".to_string(),
        }
    }
}

// 使用时可以指定操作
impl OrderRepository {
    pub fn save_order(&self, order: &Order) -> Result<(), OrderError> {
        self.connection.execute("INSERT INTO orders ...", &[&order.id, &order.user_id])
            .map_err(|db_error| OrderError::DatabaseError {
                source: db_error,
                operation: "save_order".to_string(),
            })
            .map(|_| ())
    }
}
```

#### 丰富转换模式
```rust
// 适用于需要添加额外上下文的情况
impl From<PaymentGatewayError> for OrderError {
    fn from(gateway_error: PaymentGatewayError) -> Self {
        let (order_error, additional_context) = match gateway_error {
            PaymentGatewayError::InsufficientFunds { required, available } => {
                let order_error = OrderError::InsufficientBalance {
                    required,
                    available,
                };
                let context = HashMap::from([
                    ("payment_gateway".to_string(), Value::String("stripe".to_string())),
                    ("currency".to_string(), Value::String("USD".to_string())),
                    ("user_action_required".to_string(), Value::Bool(true)),
                ]);
                (order_error, context)
            },
            PaymentGatewayError::CardDeclined { reason, card_last_four } => {
                let order_error = OrderError::PaymentDeclined {
                    reason: reason.clone(),
                    card_last_four: card_last_four.clone(),
                };
                let context = HashMap::from([
                    ("decline_reason".to_string(), Value::String(reason)),
                    ("card_masked".to_string(), Value::String(format!("****-****-****-{}", card_last_four))),
                    ("retry_suggested".to_string(), Value::Bool(matches!(reason.as_str(), "temporary_decline"))),
                ]);
                (order_error, context)
            },
            PaymentGatewayError::NetworkError { message } => {
                let order_error = OrderError::ServiceUnavailable {
                    service: "payment_gateway".to_string(),
                    message: format!("支付网关网络错误: {}", message),
                };
                let context = HashMap::from([
                    ("network_error_details".to_string(), Value::String(message)),
                    ("retry_recommended".to_string(), Value::Bool(true)),
                    ("fallback_available".to_string(), Value::Bool(true)),
                ]);
                (order_error, context)
            },
        };
        
        // 使用装饰器添加上下文
        Err(order_error).with_additional_context(additional_context).unwrap_err()
    }
}
```

### 转换链管理

#### 转换链定义
```rust
#[derive(Debug, Clone)]
pub struct ErrorConversionChain {
    pub original_error: Box<dyn std::error::Error + Send + Sync>,
    pub conversion_steps: Vec<ConversionStep>,
    pub current_error: Box<dyn std::error::Error + Send + Sync>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct ConversionStep {
    pub from_type: String,
    pub to_type: String,
    pub timestamp: SystemTime,
    pub context: HashMap<String, String>,
    pub transformer: String, // 转换函数名称
}

impl ErrorConversionChain {
    pub fn new<E: std::error::Error + Send + Sync + 'static>(original_error: E) -> Self {
        Self {
            original_error: Box::new(original_error),
            conversion_steps: Vec::new(),
            current_error: Box::new(original_error),
            metadata: HashMap::new(),
        }
    }
    
    pub fn add_conversion<E1, E2, F>(&mut self, converter: F) -> &mut Self
    where
        E1: std::error::Error + Send + Sync + 'static,
        E2: std::error::Error + Send + Sync + 'static,
        F: FnOnce(E1) -> E2,
    {
        let current_error = self.current_error.as_ref();
        if let Some(error) = current_error.downcast_ref::<E1>() {
            let new_error = converter(error.clone());
            let step = ConversionStep {
                from_type: std::any::type_name::<E1>().to_string(),
                to_type: std::any::type_name::<E2>().to_string(),
                timestamp: SystemTime::now(),
                context: HashMap::new(),
                transformer: std::any::type_name::<F>().to_string(),
            };
            
            self.conversion_steps.push(step);
            self.current_error = Box::new(new_error);
        }
        
        self
    }
    
    pub fn with_metadata(mut self, key: String, value: Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    pub fn build(self) -> Box<dyn std::error::Error + Send + Sync> {
        self.current_error
    }
}

// 使用示例
pub fn process_order_with_chain(order: &Order) -> Result<(), OrderError> {
    let result = database_service.save_order(order);
    
    let chained_error = result.map_err(|db_error| {
        let mut chain = ErrorConversionChain::new(db_error)
            .with_metadata("operation".to_string(), Value::String("save_order".to_string()))
            .with_metadata("order_id".to_string(), Value::String(order.id.clone()));
        
        chain.add_conversion(|e: DatabaseError| -> BusinessError {
            BusinessError::from_database_error(e)
        });
        
        chain.add_conversion(|e: BusinessError| -> OrderError {
            OrderError::from_business_error(e)
        });
        
        chain.build()
    })?;
    
    Ok(())
}
```

## 敏感信息过滤

### 过滤原则

#### 隐私保护原则
在错误日志和错误信息中，必须保护用户的敏感信息，包括但不限于密码、信用卡号、身份证号、手机号、邮箱等。

```rust
// Good: 敏感信息过滤
impl Display for PaymentError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            PaymentError::InvalidCard { card_number, reason } => {
                // 屏蔽信用卡号，只显示最后4位
                let masked_card = if card_number.len() > 4 {
                    format!("****-****-****-{}", &card_number[card_number.len()-4..])
                } else {
                    "****".to_string()
                };
                write!(f, "无效的信用卡 {}: {}", masked_card, reason)
            },
            PaymentError::InsufficientFunds { card_number, required, available } => {
                let masked_card = if card_number.len() > 4 {
                    format!("****-****-****-{}", &card_number[card_number.len()-4..])
                } else {
                    "****".to_string()
                };
                write!(f, "信用卡 {} 余额不足，需要: {:.2}, 可用: {:.2}", masked_card, required, available)
            },
            PaymentError::AuthenticationFailed { user_id } => {
                write!(f, "用户 {} 认证失败", user_id) // 用户ID可能也需要屏蔽
            },
        }
    }
}

// Bad: 敏感信息泄露
impl Display for PaymentError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            PaymentError::InvalidCard { card_number, reason } => {
                write!(f, "无效的信用卡 {}: {}", card_number, reason) // 完整卡号泄露
            },
            PaymentError::AuthenticationFailed { user_id, password } => {
                write!(f, "用户 {} 认证失败，密码: {}", user_id, password) // 密码泄露
            },
        }
    }
}
```

#### 最小信息原则
只记录必要的错误信息，避免记录过多可能包含敏感信息的上下文。

```rust
// Good: 最小化信息记录
pub fn log_user_operation(operation: &UserOperation) {
    let masked_user_id = mask_user_id(&operation.user_id);
    let masked_phone = mask_phone_number(&operation.phone);
    let masked_email = mask_email(&operation.email);
    
    info!(
        "用户操作: 用户={}, 操作类型={}, 手机={}, 邮箱={}",
        masked_user_id,
        operation.operation_type,
        masked_phone,
        masked_email
    );
}

// Bad: 过度信息记录
pub fn log_user_operation(operation: &UserOperation) {
    info!(
        "用户操作详情: 用户ID={}, 用户名={}, 手机={}, 邮箱={}, 地址={}, 银行卡={}, \
         操作类型={}, 操作时间={}, IP地址={}, 设备信息={}, 浏览器信息={}",
        operation.user_id,
        operation.username,
        operation.phone,
        operation.email,
        operation.address,
        operation.bank_card,
        operation.operation_type,
        operation.timestamp,
        operation.ip_address,
        operation.device_info,
        operation.browser_info
    ); // 大量敏感信息泄露
}
```

### 过滤实现

#### 敏感信息检测器
```rust
use regex::Regex;
use std::collections::HashSet;

pub struct SensitiveDataDetector {
    patterns: Vec<SensitivePattern>,
    keywords: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct SensitivePattern {
    pub name: String,
    pub pattern: Regex,
    pub replacement: String,
    pub confidence: f64,
}

impl SensitiveDataDetector {
    pub fn new() -> Self {
        let patterns = vec![
            SensitivePattern {
                name: "credit_card".to_string(),
                pattern: Regex::new(r"\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b").unwrap(),
                replacement: "****-****-****-****".to_string(),
                confidence: 0.95,
            },
            SensitivePattern {
                name: "phone_number".to_string(),
                pattern: Regex::new(r"\b1[3-9]\d{9}\b").unwrap(),
                replacement: "***********".to_string(),
                confidence: 0.90,
            },
            SensitivePattern {
                name: "email".to_string(),
                pattern: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(),
                replacement: "***@***.***".to_string(),
                confidence: 0.85,
            },
            SensitivePattern {
                name: "id_card".to_string(),
                pattern: Regex::new(r"\b\d{17}[\dXx]\b").unwrap(),
                replacement: "***************".to_string(),
                confidence: 0.95,
            },
            SensitivePattern {
                name: "password".to_string(),
                pattern: Regex::new(r"(?i)password[:\s=]+[^\s]+").unwrap(),
                replacement: "password=****".to_string(),
                confidence: 0.80,
            },
            SensitivePattern {
                name: "api_key".to_string(),
                pattern: Regex::new(r"(?i)(api[_-]?key|secret[_-]?key|access[_-]?key)[:\s=]+[^\s]+").unwrap(),
                replacement: "api_key=****".to_string(),
                confidence: 0.90,
            },
        ];
        
        let keywords = HashSet::from([
            "password".to_string(),
            "secret".to_string(),
            "token".to_string(),
            "key".to_string(),
            "auth".to_string(),
            "credential".to_string(),
        ]);
        
        Self { patterns, keywords }
    }
    
    pub fn detect_and_filter(&self, text: &str) -> String {
        let mut filtered_text = text.to_string();
        
        for pattern in &self.patterns {
            filtered_text = pattern.pattern.replace_all(&filtered_text, &pattern.replacement).to_string();
        }
        
        // 额外的关键字检查
        for keyword in &self.keywords {
            let re = Regex::new(&format!(r"(?i){}[:\s=]+[^\s]+", regex::escape(keyword))).unwrap();
            filtered_text = re.replace_all(&filtered_text, &format!("{}=****", keyword)).to_string();
        }
        
        filtered_text
    }
    
    pub fn is_sensitive(&self, text: &str) -> bool {
        for pattern in &self.patterns {
            if pattern.pattern.is_match(text) {
                return true;
            }
        }
        
        for keyword in &self.keywords {
            let re = Regex::new(&format!(r"(?i){}[:\s=]+[^\s]+", regex::escape(keyword))).unwrap();
            if re.is_match(text) {
                return true;
            }
        }
        
        false
    }
    
    pub fn get_sensitive_patterns(&self, text: &str) -> Vec<&SensitivePattern> {
        self.patterns
            .iter()
            .filter(|pattern| pattern.pattern.is_match(text))
            .collect()
    }
}
```

#### 日志过滤器
```rust
pub struct LogFilter {
    sensitive_detector: SensitiveDataDetector,
    enabled_filters: HashSet<FilterType>,
    custom_patterns: Vec<SensitivePattern>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FilterType {
    CreditCard,
    PhoneNumber,
    Email,
    IDCard,
    Password,
    APIKey,
    Custom(String),
}

impl LogFilter {
    pub fn new() -> Self {
        Self {
            sensitive_detector: SensitiveDataDetector::new(),
            enabled_filters: HashSet::from([
                FilterType::CreditCard,
                FilterType::PhoneNumber,
                FilterType::Email,
                FilterType::IDCard,
                FilterType::Password,
                FilterType::APIKey,
            ]),
            custom_patterns: Vec::new(),
        }
    }
    
    pub fn enable_filter(mut self, filter_type: FilterType) -> Self {
        self.enabled_filters.insert(filter_type);
        self
    }
    
    pub fn disable_filter(mut self, filter_type: FilterType) -> Self {
        self.enabled_filters.remove(&filter_type);
        self
    }
    
    pub fn add_custom_pattern(mut self, pattern: SensitivePattern) -> Self {
        self.custom_patterns.push(pattern);
        self
    }
    
    pub fn filter_log_entry(&self, log_entry: &str) -> String {
        let mut filtered = log_entry.to_string();
        
        // 应用启用的过滤器
        if self.enabled_filters.contains(&FilterType::CreditCard) {
            filtered = self.filter_credit_cards(&filtered);
        }
        
        if self.enabled_filters.contains(&FilterType::PhoneNumber) {
            filtered = self.filter_phone_numbers(&filtered);
        }
        
        if self.enabled_filters.contains(&FilterType::Email) {
            filtered = self.filter_emails(&filtered);
        }
        
        if self.enabled_filters.contains(&FilterType::IDCard) {
            filtered = self.filter_id_cards(&filtered);
        }
        
        if self.enabled_filters.contains(&FilterType::Password) {
            filtered = self.filter_passwords(&filtered);
        }
        
        if self.enabled_filters.contains(&FilterType::APIKey) {
            filtered = self.filter_api_keys(&filtered);
        }
        
        // 应用自定义模式
        for pattern in &self.custom_patterns {
            filtered = pattern.pattern.replace_all(&filtered, &pattern.replacement).to_string();
        }
        
        filtered
    }
    
    fn filter_credit_cards(&self, text: &str) -> String {
        let re = Regex::new(r"\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b").unwrap();
        re.replace_all(text, "****-****-****-****").to_string()
    }
    
    fn filter_phone_numbers(&self, text: &str) -> String {
        let re = Regex::new(r"\b1[3-9]\d{9}\b").unwrap();
        re.replace_all(text, "***********").to_string()
    }
    
    fn filter_emails(&self, text: &str) -> String {
        let re = Regex::new(r"\b([A-Za-z0-9._%+-]+)@([A-Za-z0-9.-]+)\.([A-Z|a-z]{2,})\b").unwrap();
        re.replace_all(text, "***@***.***").to_string()
    }
    
    fn filter_id_cards(&self, text: &str) -> String {
        let re = Regex::new(r"\b\d{17}[\dXx]\b").unwrap();
        re.replace_all(text, "***************").to_string()
    }
    
    fn filter_passwords(&self, text: &str) -> String {
        let re = Regex::new(r"(?i)(password|passwd|pwd)[\s=:]+[^\s]+").unwrap();
        re.replace_all(text, "$1=****").to_string()
    }
    
    fn filter_api_keys(&self, text: &str) -> String {
        let re = Regex::new(r"(?i)(api[_-]?key|secret[_-]?key|access[_-]?key)[\s=:]+[^\s]+").unwrap();
        re.replace_all(text, "$1=****").to_string()
    }
    
    pub fn is_sensitive_content(&self, text: &str) -> bool {
        self.sensitive_detector.is_sensitive(text)
    }
}

// 使用示例
pub fn secure_log_operation(operation: &Operation) {
    let log_filter = LogFilter::new()
        .enable_filter(FilterType::CreditCard)
        .enable_filter(FilterType::PhoneNumber)
        .add_custom_pattern(SensitivePattern {
            name: "bank_account".to_string(),
            pattern: Regex::new(r"\b\d{16,19}\b").unwrap(),
            replacement: "***************".to_string(),
            confidence: 0.85,
        });
    
    let log_message = format!(
        "操作执行: 用户={}, 卡号={}, 账户={}, 详情={}",
        operation.user_id,
        operation.card_number,
        operation.bank_account,
        operation.details
    );
    
    let filtered_message = log_filter.filter_log_entry(&log_message);
    
    if log_filter.is_sensitive_content(&log_message) {
        warn!("敏感信息已被过滤，原始日志包含敏感数据");
    }
    
    info!("安全日志: {}", filtered_message);
}
```

### 环境相关过滤

#### 环境感知过滤策略
```rust
use std::env;

pub struct EnvironmentAwareFilter {
    development_filter: LogFilter,
    production_filter: LogFilter,
    current_environment: String,
}

impl EnvironmentAwareFilter {
    pub fn new() -> Self {
        let current_environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        
        let development_filter = LogFilter::new()
            .disable_filter(FilterType::Email) // 开发环境保留邮箱信息
            .disable_filter(FilterType::PhoneNumber); // 开发环境保留手机号
        
        let production_filter = LogFilter::new()
            .enable_filter(FilterType::CreditCard)
            .enable_filter(FilterType::PhoneNumber)
            .enable_filter(FilterType::Email)
            .enable_filter(FilterType::IDCard)
            .enable_filter(FilterType::Password)
            .enable_filter(FilterType::APIKey);
        
        Self {
            development_filter,
            production_filter,
            current_environment,
        }
    }
    
    pub fn filter_log(&self, log_entry: &str) -> String {
        match self.current_environment.as_str() {
            "development" | "test" => {
                // 开发环境：部分过滤，保留调试信息
                self.development_filter.filter_log_entry(log_entry)
            },
            "staging" => {
                // 预发布环境：大部分过滤，保留部分调试信息
                let mut filter = LogFilter::new()
                    .enable_filter(FilterType::CreditCard)
                    .enable_filter(FilterType::Password)
                    .enable_filter(FilterType::APIKey);
                filter.filter_log_entry(log_entry)
            },
            "production" => {
                // 生产环境：完全过滤
                self.production_filter.filter_log_entry(log_entry)
            },
            _ => {
                // 默认情况：安全过滤
                self.production_filter.filter_log_entry(log_entry)
            }
        }
    }
    
    pub fn should_log_full_error(&self) -> bool {
        matches!(self.current_environment.as_str(), "development" | "test")
    }
    
    pub fn should_log_stack_trace(&self) -> bool {
        matches!(self.current_environment.as_str(), "development" | "test")
    }
}

// 全局日志过滤器
lazy_static! {
    static ref GLOBAL_LOG_FILTER: EnvironmentAwareFilter = EnvironmentAwareFilter::new();
}

pub fn secure_info(message: &str) {
    let filtered_message = GLOBAL_LOG_FILTER.filter_log(message);
    info!("{}", filtered_message);
}

pub fn secure_warn(message: &str) {
    let filtered_message = GLOBAL_LOG_FILTER.filter_log(message);
    warn!("{}", filtered_message);
}

pub fn secure_error(message: &str) {
    let filtered_message = GLOBAL_LOG_FILTER.filter_log(message);
    error!("{}", filtered_message);
}

// 带堆栈跟踪的安全错误日志
pub fn secure_error_with_trace(error: &dyn std::error::Error) {
    let error_message = error.to_string();
    let filtered_message = GLOBAL_LOG_FILTER.filter_log(&error_message);
    
    error!("错误: {}", filtered_message);
    
    if GLOBAL_LOG_FILTER.should_log_stack_trace() {
        let backtrace = std::backtrace::Backtrace::capture();
        error!("堆栈跟踪: {:?}", backtrace);
    }
}
```

## 错误处理性能优化

### 内存管理

#### 错误对象池化
```rust
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::VecDeque;

pub struct ErrorPool<E> {
    pool: Arc<Mutex<VecDeque<E>>>,
    max_size: usize,
    constructor: Box<dyn Fn() -> E + Send + Sync>,
}

impl<E> ErrorPool<E> {
    pub fn new<F>(max_size: usize, constructor: F) -> Self
    where
        F: Fn() -> E + Send + Sync + 'static,
    {
        Self {
            pool: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
            constructor: Box::new(constructor),
        }
    }
    
    pub fn get(&self) -> PooledError<E> {
        let mut pool = self.pool.lock().unwrap();
        let error = pool.pop_front().unwrap_or_else(|| (self.constructor)());
        
        PooledError {
            error: Some(error),
            pool: self.pool.clone(),
        }
    }
    
    pub fn size(&self) -> usize {
        let pool = self.pool.lock().unwrap();
        pool.len()
    }
}

pub struct PooledError<E> {
    error: Option<E>,
    pool: Arc<Mutex<VecDeque<E>>>,
}

impl<E> std::ops::Deref for PooledError<E> {
    type Target = E;
    fn deref(&self) -> &E {
        self.error.as_ref().unwrap()
    }
}

impl<E> std::ops::DerefMut for PooledError<E> {
    fn deref_mut(&mut self) -> &mut E {
        self.error.as_mut().unwrap()
    }
}

impl<E> Drop for PooledError<E> {
    fn drop(&mut self) {
        if let Some(error) = self.error.take() {
            let mut pool = self.pool.lock().unwrap();
            if pool.len() < pool.capacity() {
                pool.push_back(error);
            }
        }
    }
}

// 使用示例
lazy_static! {
    static ref ORDER_ERROR_POOL: ErrorPool<OrderError> = ErrorPool::new(
        100,
        || OrderError::new("pooled_error".to_string())
    );
}

pub fn handle_order_with_pool(order: &Order) -> Result<OrderResponse, OrderError> {
    let mut pooled_error = ORDER_ERROR_POOL.get();
    
    // 使用池化错误对象
    *pooled_error = validate_order(order)?;
    
    // 处理成功，错误对象会被自动回收到池中
    Ok(OrderResponse::new("success".to_string()))
}
```

#### 字符串内存优化
```rust
use std::borrow::Cow;
use std::sync::Arc;

pub enum ErrorString {
    Static(&'static str),
    Shared(Arc<str>),
    Owned(String),
}

impl ErrorString {
    pub fn new<S: Into<String>>(s: S) -> Self {
        let string = s.into();
        
        // 对于短字符串，使用静态分配
        if string.len() <= 32 {
            Self::Shared(Arc::from(string))
        } else {
            Self::Owned(string)
        }
    }
    
    pub fn from_static(s: &'static str) -> Self {
        Self::Static(s)
    }
    
    pub fn as_str(&self) -> &str {
        match self {
            ErrorString::Static(s) => s,
            ErrorString::Shared(s) => s,
            ErrorString::Owned(s) => s,
        }
    }
    
    pub fn into_shared(self) -> Self {
        match self {
            ErrorString::Static(s) => Self::Shared(Arc::from(s)),
            ErrorString::Shared(s) => Self::Shared(s),
            ErrorString::Owned(s) => Self::Shared(Arc::from(s)),
        }
    }
}

impl std::fmt::Display for ErrorString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OptimizedError {
    #[error("业务错误: {message}")]
    Business { 
        message: ErrorString,
        code: u32,
    },
    
    #[error("系统错误: {message}")]
    System { 
        message: ErrorString,
        component: ErrorString,
    },
    
    #[error("验证错误: {field} - {message}")]
    Validation { 
        field: ErrorString,
        message: ErrorString,
    },
}

impl OptimizedError {
    pub fn business_message<S: Into<String>>(message: S, code: u32) -> Self {
        Self::Business {
            message: ErrorString::new(message),
            code,
        }
    }
    
    pub fn system_message<S1: Into<String>, S2: Into<String>>(message: S1, component: S2) -> Self {
        Self::System {
            message: ErrorString::new(message),
            component: ErrorString::new(component),
        }
    }
    
    pub fn validation_error<S1: Into<String>, S2: Into<String>>(field: S1, message: S2) -> Self {
        Self::Validation {
            field: ErrorString::new(field),
            message: ErrorString::new(message),
        }
    }
}
```

### 异步处理

#### 异步错误处理
```rust
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct AsyncErrorHandler {
    error_sender: mpsc::UnboundedSender<AsyncErrorEvent>,
    _processor_handle: JoinHandle<()>,
}

pub struct AsyncErrorEvent {
    pub error: Box<dyn std::error::Error + Send + Sync>,
    pub context: ErrorContext,
    pub timestamp: SystemTime,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AsyncErrorHandler {
    pub fn new() -> Self {
        let (error_sender, mut error_receiver) = mpsc::unbounded_channel();
        
        let processor_handle = tokio::spawn(async move {
            while let Some(error_event) = error_receiver.recv().await {
                Self::process_error_event(error_event).await;
            }
        });
        
        Self {
            error_sender,
            _processor_handle: processor_handle,
        }
    }
    
    pub fn send_error(&self, error: impl std::error::Error + Send + Sync + 'static, 
                     context: ErrorContext, severity: ErrorSeverity) -> Result<(), Error> {
        let event = AsyncErrorEvent {
            error: Box::new(error),
            context,
            timestamp: SystemTime::now(),
            severity,
        };
        
        self.error_sender.send(event)
            .map_err(|_| Error::new("Failed to send error to async handler"))
    }
    
    async fn process_error_event(event: AsyncErrorEvent) {
        // 根据严重程度处理错误
        match event.severity {
            ErrorSeverity::Critical => {
                // 立即处理严重错误
                Self::handle_critical_error(event).await;
            },
            ErrorSeverity::High => {
                // 高优先级处理
                tokio::spawn(Self::handle_high_priority_error(event));
            },
            ErrorSeverity::Medium => {
                // 中优先级处理
                tokio::spawn(Self::handle_medium_priority_error(event));
            },
            ErrorSeverity::Low => {
                // 低优先级处理
                tokio::spawn(Self::handle_low_priority_error(event));
            },
        }
    }
    
    async fn handle_critical_error(event: AsyncErrorEvent) {
        // 立即记录日志
        error!(
            "严重错误: {}, 上下文: {}, 时间: {}",
            event.error,
            serde_json::to_string(&event.context).unwrap_or_default(),
            event.timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        
        // 发送告警
        if let Err(e) = Self::send_alert(&event).await {
            error!("发送告警失败: {}", e);
        }
        
        // 执行恢复操作
        if let Err(e) = Self::execute_recovery(&event).await {
            error!("恢复操作失败: {}", e);
        }
    }
    
    async fn handle_high_priority_error(event: AsyncErrorEvent) {
        // 高优先级错误处理
        warn!("高优先级错误: {}", event.error);
        
        // 批量处理逻辑
        Self::batch_process_errors(vec![event]).await;
    }
    
    async fn handle_medium_priority_error(event: AsyncErrorEvent) {
        // 中优先级错误处理
        info!("中优先级错误: {}", event.error);
        
        // 可以添加到批量处理队列
        Self::add_to_batch_queue(event).await;
    }
    
    async fn handle_low_priority_error(event: AsyncErrorEvent) {
        // 低优先级错误处理
        debug!("低优先级错误: {}", event.error);
        
        // 统计分析
        Self::analyze_error_pattern(&event).await;
    }
    
    async fn send_alert(event: &AsyncErrorEvent) -> Result<(), Error> {
        // 实现告警发送逻辑
        Ok(())
    }
    
    async fn execute_recovery(event: &AsyncErrorEvent) -> Result<(), Error> {
        // 实现恢复操作逻辑
        Ok(())
    }
    
    async fn batch_process_errors(events: Vec<AsyncErrorEvent>) {
        // 批量处理错误
        for event in events {
            info!("批量处理错误: {}", event.error);
        }
    }
    
    async fn add_to_batch_queue(event: AsyncErrorEvent) {
        // 添加到批量处理队列
        info!("添加到批量队列: {}", event.error);
    }
    
    async fn analyze_error_pattern(event: &AsyncErrorEvent) {
        // 分析错误模式
        debug!("分析错误模式: {}", event.error);
    }
}

// 全局异步错误处理器
lazy_static! {
    static ref GLOBAL_ASYNC_ERROR_HANDLER: AsyncErrorHandler = AsyncErrorHandler::new();
}

pub fn async_report_error(error: impl std::error::Error + Send + Sync + 'static, 
                          context: ErrorContext, severity: ErrorSeverity) {
    if let Err(e) = GLOBAL_ASYNC_ERROR_HANDLER.send_error(error, context, severity) {
        error!("异步错误报告失败: {}", e);
    }
}

// 使用示例
pub async fn process_order_async(order: Order) -> Result<OrderResponse, OrderError> {
    match validate_order_async(&order).await {
        Ok(_) => {
            // 验证成功
            Ok(process_valid_order(&order).await?)
        },
        Err(validation_error) => {
            // 异步报告错误
            let context = ErrorContext::new("order_validation", "order")
                .with_entity_id(order.id.clone());
            
            async_report_error(
                validation_error,
                context,
                ErrorSeverity::Medium
            );
            
            Err(OrderError::validation_failed("订单验证失败"))
        }
    }
}
```

## 测试策略

### 错误测试模式

#### 错误注入测试
```rust
pub trait ErrorInjector<T, E> {
    fn inject_error(&self, operation: impl FnOnce() -> Result<T, E>) -> Result<T, E>;
    fn inject_random_error(&self, operation: impl FnOnce() -> Result<T, E>) -> Result<T, E>;
}

pub struct ConfigurableErrorInjector {
    error_rate: f64,
    error_types: Vec<Box<dyn Fn() -> Box<dyn std::error::Error + Send + Sync>>>,
    random_generator: Box<dyn Fn() -> f64 + Send + Sync>,
}

impl ConfigurableErrorInjector {
    pub fn new(error_rate: f64) -> Self {
        Self {
            error_rate,
            error_types: Vec::new(),
            random_generator: Box::new(|| rand::random::<f64>()),
        }
    }
    
    pub fn add_error_type<E: std::error::Error + Send + Sync + 'static, F: Fn() -> E + Send + Sync + 'static>(
        mut self, 
        error_factory: F
    ) -> Self {
        self.error_types.push(Box::new(move || Box::new(error_factory())));
        self
    }
    
    pub fn with_random_generator<F: Fn() -> f64 + Send + Sync + 'static>(mut self, generator: F) -> Self {
        self.random_generator = Box::new(generator);
        self
    }
    
    fn should_inject_error(&self) -> bool {
        (self.random_generator)() < self.error_rate
    }
    
    fn get_random_error(&self) -> Option<Box<dyn std::error::Error + Send + Sync>> {
        if self.error_types.is_empty() {
            None
        } else {
            let index = (self.random_generator() * self.error_types.len() as f64) as usize;
            Some(self.error_types[index % self.error_types.len()]())
        }
    }
}

impl<T, E: std::error::Error + Send + Sync + 'static> ErrorInjector<T, E> for ConfigurableErrorInjector {
    fn inject_error(&self, operation: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
        if self.should_inject_error() {
            if let Some(injected_error) = self.get_random_error() {
                // 将注入的错误转换为目标错误类型
                let target_error: E = unsafe { std::mem::transmute_copy(&injected_error) };
                return Err(target_error);
            }
        }
        
        operation()
    }
    
    fn inject_random_error(&self, operation: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
        operation() // 随机错误注入逻辑类似
    }
}

// 测试使用示例
#[cfg(test)]
mod error_injection_tests {
    use super::*;
    
    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error("网络错误")]
        Network,
        #[error("数据库错误")]
        Database,
        #[error("超时错误")]
        Timeout,
    }
    
    #[test]
    fn test_error_injection() {
        let injector = ConfigurableErrorInjector::new(0.5) // 50%错误率
            .add_error_type(|| TestError::Network)
            .add_error_type(|| TestError::Database)
            .add_error_type(|| TestError::Timeout);
        
        let mut success_count = 0;
        let mut error_count = 0;
        
        for _ in 0..100 {
            let result = injector.inject_error(|| {
                // 模拟正常操作
                Ok::<i32, TestError>(42)
            });
            
            match result {
                Ok(_) => success_count += 1,
                Err(_) => error_count += 1,
            }
        }
        
        // 验证错误率接近预期
        let actual_error_rate = error_count as f64 / 100.0;
        assert!((actual_error_rate - 0.5).abs() < 0.1);
    }
}
```

#### 错误场景测试
```rust
pub struct ErrorScenarioTester {
    scenarios: Vec<ErrorScenario>,
    test_runner: Box<dyn Fn(&ErrorScenario) -> TestResult + Send + Sync>,
}

pub struct ErrorScenario {
    pub name: String,
    pub description: String,
    pub error_injector: Box<dyn ErrorInjector<(), Box<dyn std::error::Error + Send + Sync>>>,
    pub expected_error_type: String,
    pub recovery_strategy: Box<dyn Fn() -> bool + Send + Sync>,
}

pub struct TestResult {
    pub scenario_name: String,
    pub success: bool,
    pub actual_error: Option<String>,
    pub recovery_successful: bool,
    pub execution_time_ms: u64,
}

impl ErrorScenarioTester {
    pub fn new() -> Self {
        Self {
            scenarios: Vec::new(),
            test_runner: Box::new(Self::default_test_runner),
        }
    }
    
    pub fn add_scenario(mut self, scenario: ErrorScenario) -> Self {
        self.scenarios.push(scenario);
        self
    }
    
    pub fn with_test_runner<F>(mut self, runner: F) -> Self
    where
        F: Fn(&ErrorScenario) -> TestResult + Send + Sync + 'static,
    {
        self.test_runner = Box::new(runner);
        self
    }
    
    pub fn run_all_tests(&self) -> Vec<TestResult> {
        let mut results = Vec::new();
        
        for scenario in &self.scenarios {
            let result = (self.test_runner)(scenario);
            results.push(result);
        }
        
        results
    }
    
    fn default_test_runner(scenario: &ErrorScenario) -> TestResult {
        let start_time = std::time::Instant::now();
        
        // 执行错误注入测试
        let result = scenario.error_injector.inject_error(|| Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()));
        let actual_error = result.err().map(|e| e.to_string());
        
        // 执行恢复策略
        let recovery_successful = (scenario.recovery_strategy)();
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        TestResult {
            scenario_name: scenario.name.clone(),
            success: actual_error.as_ref().map_or(false, |e| e.contains(&scenario.expected_error_type)),
            actual_error,
            recovery_successful,
            execution_time_ms,
        }
    }
}

// 使用示例
fn create_error_scenario_tester() -> ErrorScenarioTester {
    let network_error_injector = ConfigurableErrorInjector::new(1.0) // 100%错误率
        .add_error_type(|| Box::new(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection refused")) as Box<dyn std::error::Error + Send + Sync>);
    
    let database_error_injector = ConfigurableErrorInjector::new(1.0)
        .add_error_type(|| Box::new(DatabaseError::ConnectionFailed) as Box<dyn std::error::Error + Send + Sync>);
    
    ErrorScenarioTester::new()
        .add_scenario(ErrorScenario {
            name: "网络连接失败".to_string(),
            description: "测试网络连接失败时的错误处理".to_string(),
            error_injector: Box::new(network_error_injector),
            expected_error_type: "Connection refused".to_string(),
            recovery_strategy: Box::new(|| {
                // 模拟网络重连恢复
                true
            }),
        })
        .add_scenario(ErrorScenario {
            name: "数据库连接失败".to_string(),
            description: "测试数据库连接失败时的错误处理".to_string(),
            error_injector: Box::new(database_error_injector),
            expected_error_type: "ConnectionFailed".to_string(),
            recovery_strategy: Box::new(|| {
                // 模拟数据库重连恢复
                true
            }),
        })
}

#[test]
fn test_error_scenarios() {
    let tester = create_error_scenario_tester();
    let results = tester.run_all_tests();
    
    for result in results {
        assert!(result.success, "场景 {} 测试失败", result.scenario_name);
        assert!(result.recovery_successful, "场景 {} 恢复失败", result.scenario_name);
    }
}
```

## 结构化错误构建建议

- 在汇聚链路上优先使用 `StructError::builder`：可一次性设置 `detail()`、`position()` 与多条上下文，避免在 `Result` 链中多次克隆。
- 使用 `builder.context_ref(&ctx)` 复用既有 `OperationContext`，库内部会共享 `Arc<Vec<_>>`，不会重复分配。
- 与 `err_conv()`/`owe(...)` 组合时，可先构建领域级错误，再通过转换保持上下文连续。

```rust
let err = StructError::builder(OrderError::InsufficientInventory)
    .detail(format!("剩余库存 {remaining}, 请求 {want}"))
    .context_ref(&ctx)
    .finish();

return Err(err);
```

## 上下文作用域 Guard

- 对需要自动记录成功日志的流程，优先使用 `OperationContext::scoped_success()`；无须在所有返回分支显式 `mark_suc()`。
- `OperationScope` 默认在 Drop 时检查状态：`scope()` 保持失败，`scoped_success()` 自动标记成功，可调用 `mark_failure()` 或 `cancel()` 覆盖。
- Guard 实现 `Deref`/`DerefMut`，可直接调用 `record()/info()` 等方法。

```rust
let mut ctx = OperationContext::doing("billing").with_auto_log();
{
    let mut scope = ctx.scoped_success();
    scope.record("invoice_id", invoice_id.to_string());
    if !charge() {
        scope.mark_failure();
        return Err(fallback());
    }
}
```

## 相关文档

- [错误分类体系](./01-error-classification.md) - 不同类型错误的分类方法
- [错误处理策略](./02-handling-strategies.md) - 不同类型错误的处理策略
- [错误归集机制](./03-error-aggregation.md) - 跨层错误转换方法
- [错误处理层级](./04-handling-layers.md) - 分层处理模型
- [错误日志规范](./05-logging-standards.md) - 标准化错误日志字段格式
- [监控指标体系](./07-monitoring-metrics.md) - 关键指标定义和告警阈值
- [错误恢复模式](./08-recovery-patterns.md) - 重试机制和容错模式
