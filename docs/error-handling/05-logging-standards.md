# 错误日志规范

> 历史设计说明：
> 本页保留的是日志字段与展示层面的设计草案，不是 `orion-error 0.6.x / V1 API` 的精确结构定义。
> 文中的 `StructError` 内部结构、`UvsReason` 枚举形态和日志字段示例可能早于当前实现，不能直接视为源码契约。
> 实际可依赖的输出字段、报告结构和渲染接口，请以当前源码中的 `ErrorReport`、`RenderMode` 及相关测试为准，并参考 [使用教程](../tutorial.md)。

## 当前 V1 对照

如果你的目标是“基于当前 crate 做日志/观测输出”，优先按下面的稳定入口理解：

- 结构化导出：`ErrorReport`
- 文本渲染：`render(RenderMode::Compact)` / `render(RenderMode::Verbose)`
- 脱敏输出：`render_redacted(...)`
- 机器可读 source 链：`source_frames`
- 上下文主语义：`want` / `path`
- 机器可读 metadata：`ErrorMetadata`

下面正文里出现的 `StructError` 内部字段、`UvsReason` 旧枚举形态、`OperationContext::with_target(...)` / `with_path(...)` 等代码，大多只是历史设计稿或示意结构，不是当前 `orion-error 0.6.x / V1 API` 的稳定契约。

## 概述

错误日志是错误处理系统的重要组成部分，标准化的日志格式和字段定义能够确保错误信息的一致性、可读性和可分析性。本规范基于 `StructError<T>` 和 `UvsReason` 的实际实现，定义了系统错误日志的标准格式、字段定义和最佳实践。

## 核心错误结构

以下结构体与枚举定义是历史结构示意，不应作为当前源码契约引用：

### StructError 结构
```rust
#[derive(Error, Debug, Clone, PartialEq, Getters)]
pub struct StructError<T: DomainReason> {
    imp: Box<StructErrorImpl<T>>,
}

#[derive(Error, Debug, Clone, PartialEq, Getters, Serialize)]
pub struct StructErrorImpl<T: DomainReason> {
    reason: T,                    // 错误原因
    detail: Option<String>,       // 技术详细信息
    position: Option<String>,     // 错误发生位置
    context: Vec<OperationContext>, // 操作上下文列表
}
```

### UvsReason 错误分类
```rust
#[derive(Debug, Error, PartialEq, Clone, Serialize)]
pub enum UvsReason {
    // === Business Layer Errors (100-199) ===
    /// 输入验证错误 (格式错误、参数校验失败等)
    #[error("validation error << {0}")]
    ValidationError(ErrorPayload),

    /// 业务逻辑规则违反 (业务规则违反、状态冲突等)
    #[error("business logic error << {0}")]
    BusinessError(ErrorPayload),

    /// 资源不存在 (查询的资源不存在)
    #[error("not found error << {0}")]
    NotFoundError(ErrorPayload),

    /// 权限和认证错误 (权限不足、认证失败)
    #[error("permission error << {0}")]
    PermissionError(ErrorPayload),

    // === Infrastructure Layer Errors (200-299) ===
    /// 数据库和数据处理错误 (数据库操作、数据格式错误)
    #[error("data error << {0}")]
    DataError(ErrorPayload, Option<usize>),

    /// 文件系统和操作系统错误 (文件系统、操作系统错误)
    #[error("system error << {0}")]
    SystemError(ErrorPayload),

    /// 网络连接和协议错误 (网络连接、HTTP请求错误)
    #[error("network error << {0}")]
    NetworkError(ErrorPayload),

    /// 资源耗尽 (内存不足、磁盘空间不足等)
    #[error("resource error << {0}")]
    ResourceError(ErrorPayload),

    /// 操作超时 (操作超时)
    #[error("timeout error << {0}")]
    TimeoutError(ErrorPayload),

    // === Configuration & External Layer Errors (300-399) ===
    /// 配置相关错误
    #[error("configuration error << {0}")]
    ConfigError(ConfErrReason),

    /// 第三方服务错误
    #[error("external service error << {0}")]
    ExternalError(ErrorPayload),

    /// 逻辑错误
    #[error("BUG :logic error << {0}")]
    LogicError(ErrorPayload),
}
```

### OperationContext 上下文结构
```rust
#[derive(Debug, Clone, Getters, Default, Serialize, Deserialize, PartialEq)]
pub struct OperationContext {
    target: Option<String>,    // 操作目标
    context: CallContext,      // 调用上下文
    is_suc: bool,             // 是否成功
    exit_log: bool,           // 是否退出时记录日志
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CallContext {
    items: Vec<(String, String)>, // 上下文键值对
}
```

> 提示：这里的 `OperationContext` 字段布局和部分方法名属于历史示意；当前用法请以源码和教程中的 `doing(...)`、`record(...)`、`with_auto_log()`、`scoped_success()` 等接口为准。

#### 用户上下文
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: Option<String>,        // 用户ID
    pub user_type: Option<String>,      // 用户类型
    pub user_role: Option<String>,      // 用户角色
    pub session_id: Option<String>,     // 会话ID
    pub client_ip: Option<String>,      // 客户端IP
    pub user_agent: Option<String>,     // 用户代理
    pub tenant_id: Option<String>,      // 租户ID
    pub department: Option<String>,     // 部门信息
}
```

#### 系统上下文
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub thread_id: String,              // 线程ID
    pub thread_name: Option<String>,    // 线程名称
    pub coroutine_id: Option<String>,   // 协程ID
    pub memory_usage: Option<u64>,      // 内存使用量(字节)
    pub cpu_usage: Option<f64>,         // CPU使用率
    pub disk_usage: Option<u64>,        // 磁盘使用量(字节)
    pub network_io: Option<NetworkIo>,  // 网络IO信息
    pub database_connections: Option<u32>, // 数据库连接数
}
```

## 日志格式规范

本节 JSON / 文本样例主要表达观测字段设计方向，不代表当前 crate 默认会直接输出同名字段，也不代表字段全集稳定承诺。

### JSON 格式标准

#### 标准格式
```json
{
  "@timestamp": "2024-01-15T10:30:45.123Z",
  "@version": "1",
  "message": "订单创建失败：余额不足",
  "level": "ERROR",
  "logger_name": "com.example.order.OrderService",
  "thread_name": "order-service-thread-1",
  "error": {
    "trace_id": "abc123-def456-ghi789",
    "span_id": "span001",
    "request_id": "req-001",
    "error_code": "ORDER.BUSINESS.INSUFFICIENT_BALANCE",
    "error_type": "BusinessRule",
    "severity": "Error",
    "category": "UserFacing",
    "detail": "用户 user_123 创建订单时余额不足，需要 100.00，可用 50.00",
    "root_cause": "数据库查询显示用户账户余额不足",
    "stack_trace": "com.example.order.OrderService.createOrder(OrderService.java:45)",
    "retry_count": 0,
    "handled": true,
    "recovery_action": "返回错误信息给用户"
  },
  "context": {
    "order_id": "order_001",
    "user_id": "user_123",
    "order_amount": 100.00,
    "currency": "CNY",
    "payment_method": "credit_card"
  },
  "user_context": {
    "user_id": "user_123",
    "user_type": "customer",
    "client_ip": "192.168.1.100",
    "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"
  },
  "system_context": {
    "thread_id": "47",
    "memory_usage": 104857600,
    "cpu_usage": 25.5,
    "database_connections": 5
  },
  "environment": {
    "service_name": "order-service",
    "service_version": "1.2.3",
    "environment": "production",
    "host_name": "order-service-pod-001",
    "region": "us-west-2"
  },
  "tags": ["order", "payment", "business"],
  "metrics": {
    "processing_time_ms": 45,
    "database_query_time_ms": 12
  }
}
```

#### 简化格式 (针对高频日志)
```json
{
  "@timestamp": "2024-01-15T10:30:45.123Z",
  "level": "ERROR",
  "trace_id": "abc123-def456-ghi789",
  "error_code": "ORDER.BUSINESS.INSUFFICIENT_BALANCE",
  "message": "订单创建失败：余额不足",
  "user_id": "user_123",
  "order_id": "order_001",
  "service": "order-service",
  "environment": "production"
}
```

### 文本格式标准

#### 结构化文本格式
```log
[2024-01-15T10:30:45.123Z] [ERROR] [order-service] [trace_id=abc123-def456-ghi789] [request_id=req-001] 
[error_code=ORDER.BUSINESS.INSUFFICIENT_BALANCE] [user_id=user_123] [order_id=order_001] 
订单创建失败：余额不足 | detail: 用户 user_123 创建订单时余额不足，需要 100.00，可用 50.00 | 
context: {\"order_amount\":100.00,\"currency\":\"CNY\"} | 
system: {\"memory_usage\":104857600,\"cpu_usage\":25.5}
```

#### 简化文本格式
```log
2024-01-15T10:30:45.123Z ERROR order-service trace_id=abc123-def456-ghi789 error_code=ORDER.BUSINESS.INSUFFICIENT_BALANCE message=订单创建失败：余额不足 user_id=user_123
```

### 错误处理策略

#### ErrStrategy 枚举
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrStrategy {
    // === 基础策略 ===
    Ignore,                         // 忽略错误
    Log,                            // 仅记录日志
    Panic,                          // 触发panic
    
    // === 重试策略 ===
    Retry(RetryConfig),             // 重试策略
    RetryWithBackoff(RetryConfig),  // 带退避的重试策略
    
    // === 降级策略 ===
    Degrade(DegradeConfig),         // 降级策略
    Fallback(FallbackConfig),       // 回退策略
    
    // === 熔断策略 ===
    CircuitBreaker(CircuitConfig),  // 熔断策略
    
    // === 自定义策略 ===
    Custom(Box<dyn Fn(&StructError<UvsReason>) -> Result<(), StructError<UvsReason>>>),
}
```

### 日志记录标准

#### StructError 日志格式

基于StructError结构体，我们定义了以下日志记录标准：

```rust
impl<T: DomainReason> Display for StructError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\n{}\n", self.reason)?;
        if let Some(pos) = &self.position {
            write!(f, "  position: {}\n", pos)?;
        }
        if let Some(detail) = &self.detail {
            write!(f, "  detail: {}\n", detail)?;
        }
        if let Some(ctx) = &self.context {
            write!(f, "  context: {}\n", ctx)?;
        }
        Ok(())
    }
}
```

#### OperationContext 日志方法

OperationContext提供了多种日志记录方法：

```rust
impl OperationContext {
    // 记录信息级别日志
    pub fn info(&self, msg: &str) {
        if self.exit_log {
            info!("{}: {}", self, msg);
        }
    }
    
    // 记录调试级别日志
    pub fn debug(&self, msg: &str) {
        if self.exit_log {
            debug!("{}: {}", self, msg);
        }
    }
    
    // 记录警告级别日志
    pub fn warn(&self, msg: &str) {
        if self.exit_log {
            warn!("{}: {}", self, msg);
        }
    }
    
    // 记录错误级别日志
    pub fn error(&self, msg: &str) {
        if self.exit_log {
            error!("{}: {}", self, msg);
        }
    }
}
```

## 日志级别使用指南

### 错误级别 (Error)

**使用场景：**
- 系统无法继续运行的严重错误
- 数据丢失或损坏
- 安全漏洞
- 关键业务流程失败

**示例：**
```rust
// 数据库连接失败
let result = database_connection.execute(query);
match result {
    Ok(_) => {},
    Err(e) => {
        let ctx = OperationContext::default()
            .with_target("database_connection")
            .with("query", query.to_string());
        ctx.error(&format!("Database connection failed: {}", e));
        return Err(StructError::new(
            UvsReason::DataError(ErrorPayload::new("database connection failed"), None),
            Some("database_connection".to_string()),
        ));
    }
}
```

### 警告级别 (Warn)

**使用场景：**
- 可能导致问题的异常情况
- 降级服务
- 性能问题
- 配置问题

**示例：**
```rust
// 缓存命中率低
if cache_hit_rate < 0.5 {
    let ctx = OperationContext::default()
        .with_target("cache_service")
        .with("hit_rate", cache_hit_rate.to_string());
    ctx.warn(&format!("Low cache hit rate: {}", cache_hit_rate));
}

// 降级服务
if primary_service.is_unavailable() {
    let ctx = OperationContext::default()
        .with_target("service_fallback")
        .with("primary_status", "unavailable".to_string());
    ctx.warn("Primary service unavailable, using fallback");
    use_fallback_service();
}
```

### 信息级别 (Info)

**使用场景：**
- 关键业务流程开始/结束
- 重要的状态变更
- 用户操作
- 系统启动/关闭

**示例：**
```rust
// 用户登录
let ctx = OperationContext::default()
    .with_target("user_auth")
    .with("user_id", user_id.to_string())
    .with("ip", ip_address.to_string());
ctx.info(&format!("User logged in: user_id={}, ip={}", user_id, ip_address));

// 订单创建
let ctx = OperationContext::default()
    .with_target("order_service")
    .with("order_id", order_id.to_string())
    .with("amount", amount.to_string());
ctx.info(&format!("Order created: order_id={}, amount={}", order_id, amount));
```

### 调试级别 (Debug)

**使用场景：**
- 函数调用跟踪
- 中间计算结果
- 条件分支执行
- 开发调试信息

**示例：**
```rust
// 函数调用跟踪
let ctx = OperationContext::default()
    .with_target("request_handler")
    .with("method", method.to_string())
    .with("path", path.to_string());
debug!("Processing request: method={}, path={}", method, path);

// 中间计算结果
let ctx = OperationContext::default()
    .with_target("calculation_service")
    .with("step", "intermediate_result".to_string());
debug!("Calculated result: {}", intermediate_result);
```

## 日志记录最佳实践

### 1. 使用OperationContext进行结构化日志

使用OperationContext提供结构化的日志记录：

```rust
// 好的做法
let ctx = OperationContext::default()
    .with_target("payment_service")
    .with("user_id", user_id.to_string())
    .with("order_id", order_id.to_string())
    .with("amount", amount.to_string())
    .with("error_code", "PAYMENT_FAILED".to_string());
ctx.error("Payment processing failed");

// 避免的做法
error!("Payment failed for user {} with order {} amount {}", user_id, order_id, amount);
```

## 结构化日志实现

### 日志记录器配置
```rust
use serde_json::json;
use tracing::{error, warn, info, debug, trace, Span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

pub fn setup_logging() {
    // 标准输出层
    let stdout_layer = tracing_subscriber::fmt::layer()
        .json() // JSON格式输出
        .with_current_span(true)
        .with_span_list(true)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true);

    // 文件输出层
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(std::io::BufWriter::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/application.log")
                .expect("Failed to open log file"),
        ));

    // 错误专用文件层
    let error_file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(std::io::BufWriter::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/error.log")
                .expect("Failed to open error log file"),
        ))
        .with_filter(tracing_subscriber::filter::LevelFilter::ERROR);

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .with(error_file_layer)
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}
```

### 错误日志宏封装
```rust
#[macro_export]
macro_rules! log_error {
    (error_code = $error_code:expr, $($field:tt = $value:expr),* $(,)?) => {
        {
            tracing::error!(
                error_code = $error_code,
                $($field = $value,)*
                error_type = tracing::field::Empty,
                severity = "ERROR",
                handled = false,
                recovery_action = tracing::field::Empty,
            );
        }
    };
}

#[macro_export]
macro_rules! log_warn {
    (error_code = $error_code:expr, $($field:tt = $value:expr),* $(,)?) => {
        {
            tracing::warn!(
                error_code = $error_code,
                $($field = $value,)*
                error_type = tracing::field::Empty,
                severity = "WARN",
                handled = false,
                recovery_action = tracing::field::Empty,
            );
        }
    };
}

#[macro_export]
macro_rules! log_info {
    (error_code = $error_code:expr, $($field:tt = $value:expr),* $(,)?) => {
        {
            tracing::info!(
                error_code = $error_code,
                $($field = $value,)*
                error_type = tracing::field::Empty,
                severity = "INFO",
                handled = false,
                recovery_action = tracing::field::Empty,
            );
        }
    };
}
```

### 上下文追踪实现
```rust
use tracing::{Span, span, Level};

pub struct ErrorContext {
    pub trace_id: String,
    pub span_id: String,
    pub request_id: String,
    pub user_id: Option<String>,
    pub service_name: String,
    pub environment: String,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            trace_id: generate_trace_id(),
            span_id: generate_span_id(),
            request_id: generate_request_id(),
            user_id: None,
            service_name: env!("CARGO_PKG_NAME").to_string(),
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        }
    }
    
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    pub fn create_span(&self, operation: &str) -> Span {
        span!(
            Level::INFO,
            "operation",
            trace_id = %self.trace_id,
            span_id = %self.span_id,
            request_id = %self.request_id,
            operation = operation,
            service_name = %self.service_name,
            environment = %self.environment,
            user_id = ?self.user_id,
        )
    }
}

// 使用示例
pub fn process_order(ctx: &ErrorContext, order_request: &OrderRequest) -> Result<OrderResponse, Error> {
    let span = ctx.create_span("process_order");
    let _guard = span.enter();
    
    log_info!(
        error_code = "ORDER.PROCESS.START",
        order_id = %order_request.id,
        user_id = %order_request.user_id,
        "开始处理订单"
    );
    
    // 业务逻辑处理
    match validate_order(order_request) {
        Ok(_) => {
            log_info!(
                error_code = "ORDER.PROCESS.SUCCESS",
                order_id = %order_request.id,
                "订单处理成功"
            );
            Ok(create_order_response(order_request))
        }
        Err(e) => {
            log_error!(
                error_code = "ORDER.PROCESS.FAILED",
                order_id = %order_request.id,
                error_detail = %e,
                "订单处理失败"
            );
            Err(e)
        }
    }
}
```

## 日志关联与追踪

### 全链路追踪实现
```rust
use opentelemetry::{
    trace::{TraceContextExt, TraceId, SpanId},
    Context,
};

pub struct TraceContext {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub baggage: std::collections::HashMap<String, String>,
}

impl TraceContext {
    pub fn current() -> Self {
        let context = Context::current();
        let span_context = context.span().span_context();
        
        Self {
            trace_id: span_context.trace_id(),
            span_id: span_context.span_id(),
            parent_span_id: None, // 父级Span需要从Context中获取
            baggage: std::collections::HashMap::new(),
        }
    }
    
    pub fn from_headers(headers: &http::HeaderMap) -> Option<Self> {
        // 从HTTP头中提取追踪信息
        headers.get("traceparent")
            .and_then(|value| value.to_str().ok())
            .and_then(|trace_parent| Self::from_traceparent(trace_parent))
    }
    
    fn from_traceparent(trace_parent: &str) -> Option<Self> {
        // 解析W3C Trace-Parent格式
        let parts: Vec<&str> = trace_parent.split('-').collect();
        if parts.len() != 4 {
            return None;
        }
        
        let trace_id = TraceId::from_hex(parts[1]).ok()?;
        let span_id = SpanId::from_hex(parts[2]).ok()?;
        
        Some(Self {
            trace_id,
            span_id,
            parent_span_id: None,
            baggage: std::collections::HashMap::new(),
        })
    }
    
    pub fn to_traceparent(&self) -> String {
        format!("00-{:032x}-{:016x}-{:02x}", 
            self.trace_id, 
            self.span_id, 
            0x01 // sampled
        )
    }
    
    pub fn inject_to_headers(&self, headers: &mut http::HeaderMap) {
        headers.insert("traceparent", 
            self.to_traceparent().parse().unwrap());
    }
}
```

### 日志关联查询实现
```rust
use serde_json::Value;
use std::collections::HashMap;

pub struct LogCorrelator {
    pub trace_id: String,
    pub related_logs: Vec<Value>,
    pub error_chain: Vec<String>,
}

impl LogCorrelator {
    pub fn new(trace_id: String) -> Self {
        Self {
            trace_id,
            related_logs: Vec::new(),
            error_chain: Vec::new(),
        }
    }
    
    pub fn add_log(&mut self, log_entry: Value) {
        if let Some(log_trace_id) = log_entry.get("trace_id")
            .and_then(|v| v.as_str()) 
        {
            if log_trace_id == self.trace_id {
                self.related_logs.push(log_entry);
            }
        }
    }
    
    pub fn build_error_chain(&mut self) {
        // 按时间戳排序日志
        self.related_logs.sort_by(|a, b| {
            let time_a = a.get("@timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let time_b = b.get("@timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            time_a.cmp(time_b)
        });
        
        // 构建错误链
        for log in &self.related_logs {
            if let Some(error_code) = log.get("error_code")
                .and_then(|v| v.as_str()) 
            {
                if error_code.starts_with("ERROR") || 
                   log.get("level")
                    .and_then(|v| v.as_str()) 
                    .map(|level| level == "ERROR")
                    .unwrap_or(false) 
                {
                    self.error_chain.push(error_code.to_string());
                }
            }
        }
    }
    
    pub fn get_correlation_summary(&self) -> HashMap<String, usize> {
        let mut summary = HashMap::new();
        
        for log in &self.related_logs {
            if let Some(service) = log.get("service")
                .and_then(|v| v.as_str()) 
            {
                *summary.entry(service.to_string()).or_insert(0) += 1;
            }
        }
        
        summary
    }
}
```

## 性能考虑

### 日志性能优化策略
```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use parking_lot::Mutex;

pub struct LogBuffer {
    buffer: Arc<Mutex<Vec<String>>>,
    max_size: usize,
    flush_threshold: usize,
    counter: Arc<AtomicUsize>,
}

impl LogBuffer {
    pub fn new(max_size: usize, flush_threshold: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            max_size,
            flush_threshold,
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn write(&self, message: String) -> Result<(), String> {
        let current_count = self.counter.fetch_add(1, Ordering::Relaxed);
        
        if current_count >= self.flush_threshold {
            // 触发异步刷新
            self.trigger_async_flush();
        }
        
        let mut buffer = self.buffer.lock();
        if buffer.len() >= self.max_size {
            buffer.clear();
        }
        
        buffer.push(message);
        Ok(())
    }
    
    fn trigger_async_flush(&self) {
        let buffer_clone = self.buffer.clone();
        let counter_clone = self.counter.clone();
        
        tokio::spawn(async move {
            let messages: Vec<String> = {
                let mut buffer = buffer_clone.lock();
                buffer.drain(..).collect()
            };
            
            if !messages.is_empty() {
                if let Err(e) = Self::flush_to_disk(&messages).await {
                    eprintln!("Failed to flush logs: {}", e);
                }
            }
            
            counter_clone.store(0, Ordering::Relaxed);
        });
    }
    
    async fn flush_to_disk(messages: &[String]) -> std::io::Result<()> {
        // 实现日志写入磁盘的逻辑
        Ok(())
    }
}
```

### 异步日志配置
```rust
use tokio::sync::mpsc;
use tracing_appender::{non_blocking, rolling};

pub fn setup_async_logging() {
    // 设置异步文件输出
    let file_appender = rolling::daily("logs", "application.log");
    let (non_blocking, _guard) = non_blocking(file_appender);
    
    // 设置异步错误文件输出
    let error_file_appender = rolling::daily("logs", "error.log");
    let (error_non_blocking, _error_guard) = non_blocking(error_file_appender);
    
    // 配置不同级别的输出目标
    let stdout_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(false)
        .with_thread_ids(true)
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);
    
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG);
    
    let error_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(error_non_blocking)
        .with_filter(tracing_subscriber::filter::LevelFilter::ERROR);
    
    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .with(error_layer)
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    // 保持guard不被释放
    // _guard和_error_guard需要在应用生命周期内保持
}
```

### 日志采样策略
```rust
use rand::Rng;

pub struct LogSampler {
    sample_rate: f64,
    always_sample_codes: Vec<String>,
    rng: rand::rngs::ThreadRng,
}

impl LogSampler {
    pub fn new(sample_rate: f64, always_sample_codes: Vec<String>) -> Self {
        Self {
            sample_rate,
            always_sample_codes,
            rng: rand::thread_rng(),
        }
    }
    
    pub fn should_sample(&mut self, error_code: &str, severity: &str) -> bool {
        // 总是采样的错误代码
        if self.always_sample_codes.contains(&error_code.to_string()) {
            return true;
        }
        
        // 总是采样的严重级别
        if severity == "ERROR" {
            return true;
        }
        
        // 根据采样率决定
        let sample_value: f64 = self.rng.gen();
        sample_value < self.sample_rate
    }
}

// 使用示例
pub fn sampled_log_info(
    sampler: &mut LogSampler,
    error_code: &str,
    fields: &str,
) {
    if sampler.should_sample(error_code, "INFO") {
        log_info!(
            error_code = error_code,
            details = fields,
            "采样日志记录"
        );
    }
}
```

### 2. 结合StructError进行错误分类

根据UvsReason类型选择合适的日志级别和处理策略：

```rust
fn handle_error_with_context(error: StructError<UvsReason>, ctx: &OperationContext) {
    match error.reason {
        // 严重错误 - Error级别
        UvsReason::DataError(_, _) | UvsReason::SystemError(_) => {
            ctx.error(&format!("Critical error occurred: {}", error));
            // 可能需要触发警报或通知
        }
        
        // 业务错误 - Warn级别
        UvsReason::BusinessError(_) | UvsReason::ValidationError(_) => {
            ctx.warn(&format!("Business error occurred: {}", error));
            // 可能需要记录到业务监控系统
        }
        
        // 网络错误 - Warn级别，可能需要重试
        UvsReason::NetworkError(_) => {
            ctx.warn(&format!("Network error occurred: {}", error));
            // 可能需要重试机制
        }
        
        // 配置错误 - Info级别
        UvsReason::ConfigError(_) => {
            ctx.info(&format!("Configuration error occurred: {}", error));
            // 可能需要配置更新
        }
    }
}
```

### 3. 错误处理策略与日志结合

将ErrStrategy与日志记录结合使用：

```rust
fn process_with_strategy<T>(
    operation: impl Fn() -> Result<T, StructError<UvsReason>>,
    strategy: ErrStrategy,
    ctx: &OperationContext
) -> Result<T, StructError<UvsReason>> {
    match operation() {
        Ok(result) => {
            ctx.info("Operation completed successfully");
            Ok(result)
        }
        Err(error) => {
            match strategy {
                ErrStrategy::Log => {
                    ctx.error(&format!("Operation failed: {}", error));
                    Err(error)
                }
                ErrStrategy::Retry(config) => {
                    ctx.warn(&format!("Operation failed, retrying: {}", error));
                    // 实现重试逻辑
                    retry_operation(operation, config, ctx)
                }
                ErrStrategy::Degrade(degrade_config) => {
                    ctx.warn(&format!("Operation failed, degrading: {}", error));
                    // 实现降级逻辑
                    degrade_operation(operation, degrade_config, ctx)
                }
                ErrStrategy::Ignore => {
                    ctx.debug(&format!("Operation failed, ignoring: {}", error));
                    Err(error)
                }
                _ => {
                    ctx.error(&format!("Operation failed with unhandled strategy: {}", error));
                    Err(error)
                }
            }
        }
    }
}
```

### 4. 上下文链式调用

使用OperationContext的链式调用构建丰富的上下文信息：

```rust
fn process_order(order: &Order) -> Result<(), StructError<UvsReason>> {
    let ctx = OperationContext::default()
        .with_target("order_processing")
        .with("order_id", order.id.to_string())
        .with("customer_id", order.customer_id.to_string())
        .with("amount", order.amount.to_string())
        .with_path("/api/orders/process")
        .with_want("process_order");
    
    ctx.info("Starting order processing");
    
    match validate_order(order) {
        Ok(_) => ctx.debug("Order validation passed"),
        Err(e) => {
            ctx.warn(&format!("Order validation failed: {}", e));
            return Err(e);
        }
    }
    
    match process_payment(order) {
        Ok(_) => ctx.info("Payment processed successfully"),
        Err(e) => {
            ctx.error(&format!("Payment processing failed: {}", e));
            return Err(e);
        }
    }
    
    ctx.info("Order processing completed successfully");
    Ok(())
}
```

### 5. 性能考虑与敏感信息保护

避免在生产环境中记录过多的调试信息，并保护敏感数据：

```rust
// 使用条件编译
#[cfg(debug_assertions)]
{
    let debug_ctx = OperationContext::default()
        .with_target("debug_info")
        .with("complex_data", format!("{:?}", complex_data));
    debug_ctx.debug("Detailed debug information");
}

// 使用日志级别控制
if log::log_enabled!(log::Level::Debug) {
    let debug_ctx = OperationContext::default()
        .with_target("expensive_computation")
        .with("result", expensive_computation().to_string());
    debug_ctx.debug("Expensive to compute debug info");
}

// 敏感信息保护
let ctx = OperationContext::default()
    .with_target("user_auth")
    .with("user_id", user_id.to_string())
    .with("login_status", "attempt".to_string());
ctx.info("User login attempt");

// 避免记录密码等敏感信息
// ctx.with("password", password.to_string()); // 错误的做法

// 使用脱敏
let masked_card = format!("****-****-****-{}", &card_number[card_number.len()-4..]);
let secure_ctx = OperationContext::default()
    .with_target("payment_processing")
    .with("card_number", masked_card);
secure_ctx.info("Credit card processed");
```

### 日志配置最佳实践

#### 1. 环境相关配置
```rust
#[derive(Debug, serde::Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub format: String,
    pub file_output: bool,
    pub max_file_size_mb: u64,
    pub max_file_count: u32,
    pub async_logging: bool,
    pub sample_rate: f64,
    pub sensitive_fields: Vec<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
            file_output: true,
            max_file_size_mb: 100,
            max_file_count: 10,
            async_logging: true,
            sample_rate: 1.0,
            sensitive_fields: vec![
                "password".to_string(),
                "credit_card".to_string(),
                "token".to_string(),
            ],
        }
    }
}

impl LogConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        if let Ok(level) = env::var("LOG_LEVEL") {
            config.level = level;
        }
        
        if let Ok(format) = env::var("LOG_FORMAT") {
            config.format = format;
        }
        
        if let Ok(sample_rate) = env::var("LOG_SAMPLE_RATE") {
            if let Ok(rate) = sample_rate.parse::<f64>() {
                config.sample_rate = rate;
            }
        }
        
        config
    }
    
    pub fn setup_logging(&self) {
        // 根据配置设置日志系统
        let level_filter = match self.level.as_str() {
            "error" => tracing_subscriber::filter::LevelFilter::ERROR,
            "warn" => tracing_subscriber::filter::LevelFilter::WARN,
            "info" => tracing_subscriber::filter::LevelFilter::INFO,
            "debug" => tracing_subscriber::filter::LevelFilter::DEBUG,
            "trace" => tracing_subscriber::filter::LevelFilter::TRACE,
            _ => tracing_subscriber::filter::LevelFilter::INFO,
        };
        
        if self.async_logging {
            setup_async_logging();
        } else {
            setup_sync_logging();
        }
    }
}
```

#### 2. 动态日志级别调整
```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

pub struct LogManager {
    config: Arc<RwLock<LogConfig>>,
    reload_signal: Arc<AtomicBool>,
}

impl LogManager {
    pub fn new(config: LogConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            reload_signal: Arc::new(AtomicBool::new(false)),
        }
    }
    
    pub async fn reload_config(&self, new_config: LogConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
        self.reload_signal.store(true, Ordering::Relaxed);
        
        // 重新设置日志系统
        config.setup_logging();
    }
    
    pub async fn get_config(&self) -> LogConfig {
        self.config.read().await.clone()
    }
    
    pub async fn watch_config_changes(&self) {
        let config = self.config.clone();
        let reload_signal = self.reload_signal.clone();
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                
                if reload_signal.load(Ordering::Relaxed) {
                    let current_config = config.read().await;
                    current_config.setup_logging();
                    reload_signal.store(false, Ordering::Relaxed);
                    
                    log_info!(
                        error_code = "LOG.CONFIG.RELOADED",
                        "日志配置已重新加载"
                    );
                }
            }
        });
    }
}
```

## 监控集成

### 监控指标定义
```rust
use prometheus::{Counter, Histogram, Gauge, Registry};

pub struct LogMetrics {
    pub log_total: Counter,
    pub log_by_level: prometheus::Vec<Counter>,
    pub log_by_error_code: prometheus::Vec<Counter>,
    pub log_processing_time: Histogram,
    pub log_buffer_size: Gauge,
    pub log_flush_failures: Counter,
}

impl LogMetrics {
    pub fn new(registry: &Registry) -> Self {
        let log_total = Counter::new("log_total", "Total number of log entries")
            .expect("Failed to create log_total counter");
        
        let log_by_level = prometheus::Vec::new(
            prometheus::Opts::new("log_by_level", "Number of logs by level"),
            &["level"]
        ).expect("Failed to create log_by_level counter");
        
        let log_by_error_code = prometheus::Vec::new(
            prometheus::Opts::new("log_by_error_code", "Number of logs by error code"),
            &["error_code"]
        ).expect("Failed to create log_by_error_code counter");
        
        let log_processing_time = Histogram::new(
            "log_processing_time_seconds",
            "Time spent processing log entries"
        ).expect("Failed to create log_processing_time histogram");
        
        let log_buffer_size = Gauge::new("log_buffer_size", "Current log buffer size")
            .expect("Failed to create log_buffer_size gauge");
        
        let log_flush_failures = Counter::new(
            "log_flush_failures_total", 
            "Total number of log flush failures"
        ).expect("Failed to create log_flush_failures counter");
        
        registry.register(Box::new(log_total.clone())).unwrap();
        registry.register(Box::new(log_by_level.clone())).unwrap();
        registry.register(Box::new(log_by_error_code.clone())).unwrap();
        registry.register(Box::new(log_processing_time.clone())).unwrap();
        registry.register(Box::new(log_buffer_size.clone())).unwrap();
        registry.register(Box::new(log_flush_failures.clone())).unwrap();
        
        Self {
            log_total,
            log_by_level,
            log_by_error_code,
            log_processing_time,
            log_buffer_size,
            log_flush_failures,
        }
    }
    
    pub fn record_log(&self, level: &str, error_code: &str, processing_time: f64) {
        self.log_total.inc();
        self.log_by_level.with_label_values(&[level]).inc();
        self.log_by_error_code.with_label_values(&[error_code]).inc();
        self.log_processing_time.observe(processing_time);
    }
    
    pub fn set_buffer_size(&self, size: f64) {
        self.log_buffer_size.set(size);
    }
    
    pub fn record_flush_failure(&self) {
        self.log_flush_failures.inc();
    }
}
```

### 告警规则定义
```yaml
# prometheus-alerts.yml
groups:
  - name: log_monitoring
    rules:
      # 高错误率告警
      - alert: HighErrorRate
        expr: rate(log_by_level{level="ERROR"}[5m]) > 10
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value }} errors per minute"
      
      # 日志处理延迟告警
      - alert: LogProcessingLatency
        expr: histogram_quantile(0.95, rate(log_processing_time_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High log processing latency"
          description: "95th percentile log processing time is {{ $value }} seconds"
      
      # 日志缓冲区大小告警
      - alert: LogBufferSizeHigh
        expr: log_buffer_size > 10000
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Log buffer size is high"
          description: "Log buffer size is {{ $value }} entries"
      
      # 特定错误类型告警
      - alert: SpecificErrorTypeHigh
        expr: rate(log_by_error_code{error_code="DATABASE.CONNECTION.FAILED"}[5m]) > 5
        for: 3m
        labels:
          severity: critical
        annotations:
          summary: "High rate of database connection failures"
          description: "Database connection failures rate is {{ $value }} per minute"
```

## 相关文档

- [错误分类体系](./01-error-classification.md) - 不同类型错误的分类方法
- [错误处理策略](./02-handling-strategies.md) - 不同类型错误的处理策略
- [错误归集机制](./03-error-aggregation.md) - 跨层错误转换方法
- [错误处理层级](./04-handling-layers.md) - 分层处理模型
- [最佳实践指南](./06-best-practices.md) - 综合最佳实践指南
