# 错误归集机制

> 历史设计说明：
> 本页主要保存早期的错误归集和跨层传播设计示意，不是 `orion-error 0.6.x / V1 API` 的直接使用手册。
> 文中的 `StructReason`、`UvsReason::from_*`、手写链路模型以及自定义包装示例，很多都不对应当前源码中的公开接口。
> 文中的 `owe_biz()` / `owe_*()` 也只是历史示意；当前代码仅保留兼容态的 `owe(...)`，`owe_*()` 已移除。
> 当前 V1 主路径请以 `into_as(...)` / `wrap_as(...)` / `err_conv()` / `with_std_source(...)` / `with_struct_source(...)` 为准，详见 [使用教程](../tutorial.md) 和 [V1 修复与评审基线](../v1-fix-and-review-plan.md)。

## 当前 V1 对照

如果你的目标是“按当前 crate 做跨层错误归集”，优先按下面的映射理解：

- 普通 `StdError` 第一次进入结构化体系：`into_as(...)`
- 已经是 `StructError<_>`，只做 reason 映射：`err_conv()`
- 已经是 `StructError<_>`，上层建立新语义边界：`wrap_as(...)`
- 普通底层 source：`with_std_source(...)`
- 结构化下层 source：`with_struct_source(...)`

下面正文中的很多类型和代码块，只能当作历史设计示意，不能直接复制为当前实现。

## 概述

错误归集机制是错误处理系统的核心组成部分，负责将不同层级、不同来源的错误信息进行统一收集、转换和管理。通过有效的错误归集，可以确保错误信息在系统各层级间传递时不会丢失关键上下文，同时保持错误类型的一致性和可追溯性。

## 跨层错误转换

### 转换原则

跨层错误转换遵循以下核心原则：

1. **语义保留**: 转换过程中保持错误的原始语义
2. **信息完整**: 保留所有相关的错误上下文信息
3. **层级映射**: 将底层错误映射到合适的业务层错误类型
4. **链式追踪**: 维护完整的错误转换链路

### 转换模式

以下示例是历史设计稿中的转换示意，不对应当前 `orion-error 0.6.x / V1 API` 的稳定接口：

#### 标准转换模式
```rust
// 底层存储错误到业务层错误的转换
impl From<StoreReason> for StructReason<OrderReason> {
    fn from(value: StoreReason) -> Self {
        match value {
            // 存储空间不足 -> 系统错误
            StoreReason::StorageFull => 
                StructReason::from(UvsReason::from_sys("storage full")),
            
            // 连接失败 -> 依赖错误
            StoreReason::ConnectionFailed => 
                StructReason::from(UvsReason::from_dep("database unavailable")),
            
            // 约束冲突 -> 业务错误
            StoreReason::ConstraintViolation => 
                StructReason::from(UvsReason::from_biz("data conflict")),
            
            // 权限错误 -> 业务错误
            StoreReason::PermissionDenied => 
                StructReason::from(UvsReason::from_biz("access denied")),
            
            // 未知错误 -> 系统错误
            StoreReason::Unknown(error) => 
                StructReason::from(UvsReason::from_sys(&format!("unknown error: {}", error))),
        }
    }
}
```

#### 带上下文的转换
```rust
// 带有上下文信息的错误转换
impl From<DatabaseError> for OrderError {
    fn from(db_error: DatabaseError) -> Self {
        let order_error = match db_error {
            DatabaseError::ConnectionFailed => {
                OrderError::ServiceUnavailable {
                    service: "database".to_string(),
                    message: "无法连接到数据库".to_string(),
                    retry_after: Some(Duration::from_secs(30)),
                }
            },
            DatabaseError::Timeout => {
                OrderError::Timeout {
                    operation: "database_query".to_string(),
                    timeout_ms: 5000,
                }
            },
            DatabaseError::ConstraintViolation { constraint, table } => {
                OrderError::ValidationFailed {
                    field: table.clone(),
                    message: format!("数据约束违反: {} in table {}", constraint, table),
                }
            },
            DatabaseError::RecordNotFound { table, id } => {
                OrderError::NotFound {
                    resource: format!("{}:{}", table, id),
                }
            },
        };
        
        // 附加原始错误信息
        order_error.with_source_error(db_error)
    }
}
```

### 转换链管理

本节的 `ErrorChain` / `ConversionStep` 是历史治理模型示意，不是当前 crate 已提供的公共类型。

#### 错误转换链定义
```rust
#[derive(Debug, Clone)]
pub struct ErrorChain {
    pub original_error: String,
    pub conversion_steps: Vec<ConversionStep>,
    pub final_error: String,
}

#[derive(Debug, Clone)]
pub struct ConversionStep {
    pub from_type: String,
    pub to_type: String,
    pub timestamp: SystemTime,
    pub context: HashMap<String, String>,
}

impl ErrorChain {
    pub fn new(original_error: String) -> Self {
        Self {
            original_error,
            conversion_steps: Vec::new(),
            final_error: String::new(),
        }
    }
    
    pub fn add_conversion(&mut self, from_type: String, to_type: String, context: HashMap<String, String>) {
        self.conversion_steps.push(ConversionStep {
            from_type,
            to_type,
            timestamp: SystemTime::now(),
            context,
        });
    }
    
    pub fn set_final_error(&mut self, final_error: String) {
        self.final_error = final_error;
    }
}
```

#### 转换链使用示例
```rust
pub fn process_order_with_chain(order: Order) -> Result<OrderResponse, OrderError> {
    let mut error_chain = ErrorChain::new("开始处理订单".to_string());
    
    match save_order_to_database(&order) {
        Ok(saved_order) => {
            error_chain.add_conversion(
                "DatabaseOperation".to_string(),
                "OrderSaved".to_string(),
                HashMap::new(),
            );
            // 继续处理...
        },
        Err(db_error) => {
            error_chain.add_conversion(
                "DatabaseError".to_string(),
                "OrderError".to_string(),
                HashMap::from([
                    ("order_id".to_string(), order.id.to_string()),
                    ("operation".to_string(), "save_order".to_string()),
                ]),
            );
            
            let order_error: OrderError = db_error.into();
            error_chain.set_final_error(format!("{:?}", order_error));
            
            return Err(order_error.with_error_chain(error_chain));
        }
    }
}
```

## 错误上下文保留

### 上下文类型

#### 请求上下文
```rust
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub user_id: Option<String>,
    pub client_ip: String,
    pub user_agent: String,
    pub timestamp: SystemTime,
    pub trace_id: String,
}

impl RequestContext {
    pub fn new() -> Self {
        Self {
            request_id: generate_request_id(),
            user_id: None,
            client_ip: String::new(),
            user_agent: String::new(),
            timestamp: SystemTime::now(),
            trace_id: generate_trace_id(),
        }
    }
    
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    pub fn with_client_info(mut self, client_ip: String, user_agent: String) -> Self {
        self.client_ip = client_ip;
        self.user_agent = user_agent;
        self
    }
}
```

#### 业务上下文
```rust
#[derive(Debug, Clone)]
pub struct BusinessContext {
    pub operation: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub business_rules: Vec<String>,
    pub custom_fields: HashMap<String, String>,
}

impl BusinessContext {
    pub fn new(operation: String, entity_type: String) -> Self {
        Self {
            operation,
            entity_type,
            entity_id: None,
            business_rules: Vec::new(),
            custom_fields: HashMap::new(),
        }
    }
    
    pub fn with_entity_id(mut self, entity_id: String) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
    
    pub fn add_business_rule(mut self, rule: String) -> Self {
        self.business_rules.push(rule);
        self
    }
    
    pub fn add_custom_field(mut self, key: String, value: String) -> Self {
        self.custom_fields.insert(key, value);
        self
    }
}
```

### 上下文管理器

#### WithContext 实现
```rust
pub struct WithContext {
    pub request_context: RequestContext,
    pub business_context: BusinessContext,
    pub error_context: Vec<ContextFrame>,
}

#[derive(Debug, Clone)]
pub struct ContextFrame {
    pub location: String,
    pub timestamp: SystemTime,
    pub data: HashMap<String, String>,
}

impl WithContext {
    pub fn want(operation: &str) -> Self {
        Self {
            request_context: RequestContext::new(),
            business_context: BusinessContext::new(operation.to_string(), "unknown".to_string()),
            error_context: Vec::new(),
        }
    }
    
    pub fn with(mut self, context_data: &str) -> Self {
        let frame = ContextFrame {
            location: "user_provided".to_string(),
            timestamp: SystemTime::now(),
            data: HashMap::from([
                ("context".to_string(), context_data.to_string()),
            ]),
        };
        self.error_context.push(frame);
        self
    }
    
    pub fn with_request_context(mut self, request_context: RequestContext) -> Self {
        self.request_context = request_context;
        self
    }
    
    pub fn with_business_context(mut self, business_context: BusinessContext) -> Self {
        self.business_context = business_context;
        self
    }
    
    pub fn add_frame(mut self, location: String, data: HashMap<String, String>) -> Self {
        let frame = ContextFrame {
            location,
            timestamp: SystemTime::now(),
            data,
        };
        self.error_context.push(frame);
        self
    }
}
```

#### 上下文使用示例

以下示例属于历史设计说明，用于解释旧式上下文聚合写法。

对于 `0.6.x / V1 API`：

- 不再把 `WithContext::want(...)` / 链式 `.want(...)` 作为推荐主路径
- 不再把 `.owe(...)` 作为普通错误第一次进入结构化体系的首选写法
- 请优先参考 `into_as(...)` / `wrap_as(...)` / `doing(...)` 的主路径说明

```rust
fn place_order() -> Result<Order> {
    // 创建错误上下文
    let mut ctx = WithContext::want("place_order");
    ctx.with(order_txt);
    
    // 解析订单并绑定上下文
    parse_order()
        .want("解析订单")
        .attach_context(&ctx)  // 绑定上下文
        .owe(BusinessError::default())?
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn want(self, description: &str) -> Result<T, ContextualError<E>> {
        self.map_err(|e| ContextualError::new(e, description))
    }
    
    fn with(mut self, ctx: &WithContext) -> Result<T, ContextualError<E>> {
        if let Err(ref mut contextual_error) = self {
            contextual_error.request_context = ctx.request_context.clone();
            contextual_error.business_context = ctx.business_context.clone();
            contextual_error.context_frames.extend(ctx.error_context.clone());
        }
        self
    }
    
    fn owe_biz(self) -> Result<T, BusinessError> {
        self.map_err(|e| BusinessError::from_contextual_error(e))
    }
}
```

### 上下文序列化

#### JSON 序列化
```rust
impl serde::Serialize for ContextualError<E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("ContextualError", 6)?;
        
        state.serialize_field("error", &self.error)?;
        state.serialize_field("description", &self.description)?;
        state.serialize_field("timestamp", &self.timestamp)?;
        state.serialize_field("request_context", &self.request_context)?;
        state.serialize_field("business_context", &self.business_context)?;
        state.serialize_field("context_frames", &self.context_frames)?;
        
        state.end()
    }
}

// 序列化示例
fn log_error_with_context<E: std::fmt::Debug>(error: &ContextualError<E>) -> String {
    serde_json::to_string_pretty(error).unwrap_or_else(|_| {
        format!("Error serializing contextual error: {:?}", error)
    })
}
```

#### 日志格式化
```rust
impl<E: std::fmt::Debug> std::fmt::Display for ContextualError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error: {} - {}\n", self.description, self.error)?;
        write!(f, "Timestamp: {}\n", self.timestamp)?;
        write!(f, "Request ID: {}\n", self.request_context.request_id)?;
        write!(f, "Trace ID: {}\n", self.request_context.trace_id)?;
        write!(f, "Operation: {}\n", self.business_context.operation)?;
        
        if let Some(entity_id) = &self.business_context.entity_id {
            write!(f, "Entity ID: {}\n", entity_id)?;
        }
        
        if !self.context_frames.is_empty() {
            write!(f, "Context Frames:\n")?;
            for (index, frame) in self.context_frames.iter().enumerate() {
                write!(f, "  [{}]: {} at {}\n", index, frame.location, frame.timestamp)?;
                for (key, value) in &frame.data {
                    write!(f, "    {}: {}\n", key, value)?;
                }
            }
        }
        
        Ok(())
    }
}
```

## 错误聚合策略

### 聚合维度

#### 时间维度聚合
```rust
pub struct TimeBasedAggregator {
    buckets: HashMap<SystemTime, ErrorBucket>,
    bucket_duration: Duration,
    max_buckets: usize,
}

impl TimeBasedAggregator {
    pub fn new(bucket_duration: Duration, max_buckets: usize) -> Self {
        Self {
            buckets: HashMap::new(),
            bucket_duration,
            max_buckets,
        }
    }
    
    pub fn add_error(&mut self, error: &dyn std::fmt::Debug) {
        let now = SystemTime::now();
        let bucket_time = self.get_bucket_time(now);
        
        let bucket = self.buckets.entry(bucket_time).or_insert_with(|| ErrorBucket::new(bucket_time));
        bucket.add_error(error);
        
        // 清理过期的桶
        self.cleanup_old_buckets(now);
    }
    
    fn get_bucket_time(&self, time: SystemTime) -> SystemTime {
        let duration_since_epoch = time.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let bucket_duration_since_epoch = duration_since_epoch.as_secs() / self.bucket_duration.as_secs();
        SystemTime::UNIX_EPOCH + Duration::from_secs(bucket_duration_since_epoch * self.bucket_duration.as_secs())
    }
    
    fn cleanup_old_buckets(&mut self, current_time: SystemTime) {
        let cutoff_time = current_time - Duration::from_secs(self.bucket_duration.as_secs() * self.max_buckets as u64);
        self.buckets.retain(|&time, _| time >= cutoff_time);
    }
    
    pub fn get_error_summary(&self) -> Vec<ErrorSummary> {
        self.buckets.values().map(|bucket| bucket.summary()).collect()
    }
}
```

#### 类型维度聚合
```rust
pub struct TypeBasedAggregator {
    error_counts: HashMap<String, usize>,
    error_samples: HashMap<String, Vec<String>>,
    max_samples_per_type: usize,
}

impl TypeBasedAggregator {
    pub fn new(max_samples_per_type: usize) -> Self {
        Self {
            error_counts: HashMap::new(),
            error_samples: HashMap::new(),
            max_samples_per_type,
        }
    }
    
    pub fn add_error(&mut self, error_type: String, error_message: String) {
        // 增加计数
        *self.error_counts.entry(error_type.clone()).or_insert(0) += 1;
        
        // 添加样本
        let samples = self.error_samples.entry(error_type).or_insert_with(Vec::new);
        samples.push(error_message);
        
        // 限制样本数量
        if samples.len() > self.max_samples_per_type {
            samples.remove(0);
        }
    }
    
    pub fn get_error_statistics(&self) -> Vec<ErrorStatistic> {
        self.error_counts.iter().map(|(error_type, count)| {
            let samples = self.error_samples.get(error_type).cloned().unwrap_or_default();
            ErrorStatistic {
                error_type: error_type.clone(),
                count: *count,
                recent_samples: samples,
            }
        }).collect()
    }
}
```

### 聚合配置

#### 聚合策略配置
```rust
#[derive(Debug, Clone)]
pub struct AggregationConfig {
    pub time_window: Duration,
    pub max_errors_per_window: usize,
    pub sampling_rate: f64,
    pub aggregation_dimensions: Vec<AggregationDimension>,
    pub alert_thresholds: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggregationDimension {
    ErrorType,
    ErrorLocation,
    ErrorSeverity,
    UserSegment,
    ServiceName,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            time_window: Duration::from_secs(300), // 5分钟
            max_errors_per_window: 1000,
            sampling_rate: 0.1, // 10%采样率
            aggregation_dimensions: vec![
                AggregationDimension::ErrorType,
                AggregationDimension::ErrorLocation,
            ],
            alert_thresholds: HashMap::from([
                ("critical".to_string(), 10),
                ("high".to_string(), 50),
                ("medium".to_string(), 100),
            ]),
        }
    }
}
```

## 监控与告警

### 聚合指标

#### 关键指标定义
```rust
#[derive(Debug, Clone)]
pub struct ErrorAggregationMetrics {
    pub total_errors: usize,
    pub unique_error_types: usize,
    pub error_rate_per_minute: f64,
    pub top_error_types: Vec<(String, usize)>,
    pub error_trend: TrendDirection,
    pub aggregation_efficiency: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

impl ErrorAggregationMetrics {
    pub fn calculate_from_buckets(buckets: &[ErrorBucket]) -> Self {
        let total_errors: usize = buckets.iter().map(|b| b.error_count).sum();
        let unique_types: HashSet<String> = buckets.iter()
            .flat_map(|b| b.error_types.keys())
            .cloned()
            .collect();
        
        let duration_minutes = buckets.len() as f64;
        let error_rate_per_minute = total_errors as f64 / duration_minutes.max(1.0);
        
        let top_error_types = buckets.iter()
            .flat_map(|b| b.error_types.iter())
            .map(|(error_type, count)| (error_type.clone(), *count))
            .collect::<HashMap<String, usize>>()
            .into_iter()
            .map(|(error_type, total_count)| (error_type, total_count))
            .collect::<Vec<(String, usize)>>();
        
        let trend = Self::calculate_trend(buckets);
        let efficiency = Self::calculate_efficiency(buckets);
        
        Self {
            total_errors,
            unique_error_types: unique_types.len(),
            error_rate_per_minute,
            top_error_types,
            error_trend: trend,
            aggregation_efficiency: efficiency,
        }
    }
    
    fn calculate_trend(buckets: &[ErrorBucket]) -> TrendDirection {
        if buckets.len() < 2 {
            return TrendDirection::Stable;
        }
        
        let recent_errors = buckets[buckets.len()-1].error_count;
        let previous_errors = buckets[buckets.len()-2].error_count;
        
        if recent_errors > previous_errors * 12 / 10 { // 20% increase
            TrendDirection::Increasing
        } else if recent_errors < previous_errors * 8 / 10 { // 20% decrease
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        }
    }
    
    fn calculate_efficiency(buckets: &[ErrorBucket]) -> f64 {
        if buckets.is_empty() {
            return 0.0;
        }
        
        let total_raw_errors: usize = buckets.iter().map(|b| b.raw_error_count).sum();
        let total_aggregated_errors: usize = buckets.iter().map(|b| b.error_count).sum();
        
        if total_raw_errors == 0 {
            return 1.0;
        }
        
        total_aggregated_errors as f64 / total_raw_errors as f64
    }
}
```

#### 告警触发逻辑
```rust
pub struct AlertManager {
    config: AggregationConfig,
    active_alerts: HashMap<String, Alert>,
    notification_channels: Vec<Box<dyn NotificationChannel>>,
}

impl AlertManager {
    pub fn new(config: AggregationConfig) -> Self {
        Self {
            config,
            active_alerts: HashMap::new(),
            notification_channels: Vec::new(),
        }
    }
    
    pub fn evaluate_metrics(&mut self, metrics: &ErrorAggregationMetrics) -> Vec<Alert> {
        let mut new_alerts = Vec::new();
        
        // 检查错误率告警
        if metrics.error_rate_per_minute > 100.0 {
            let alert = Alert::new(
                "HIGH_ERROR_RATE".to_string(),
                format!("错误率过高: {:.2} 错误/分钟", metrics.error_rate_per_minute),
                AlertSeverity::High,
            );
            new_alerts.push(alert.clone());
            self.active_alerts.insert(alert.id.clone(), alert);
        }
        
        // 检查错误类型增长告警
        if metrics.unique_error_types > 50 && metrics.error_trend == TrendDirection::Increasing {
            let alert = Alert::new(
                "ERROR_TYPE_PROLIFERATION".to_string(),
                format!("错误类型数量增长: {} 种不同错误类型", metrics.unique_error_types),
                AlertSeverity::Medium,
            );
            new_alerts.push(alert.clone());
            self.active_alerts.insert(alert.id.clone(), alert);
        }
        
        // 检查特定错误类型告警
        for (error_type, count) in &metrics.top_error_types {
            if let Some(&threshold) = self.config.alert_thresholds.get(error_type) {
                if *count > threshold {
                    let alert = Alert::new(
                        format!("{}_EXCEEDED_THRESHOLD", error_type.to_uppercase()),
                        format!("错误类型 '{}' 超过阈值: {} > {}", error_type, count, threshold),
                        AlertSeverity::High,
                    );
                    new_alerts.push(alert.clone());
                    self.active_alerts.insert(alert.id.clone(), alert);
                }
            }
        }
        
        // 发送新告警
        for alert in &new_alerts {
            self.send_alert(alert);
        }
        
        new_alerts
    }
    
    fn send_alert(&self, alert: &Alert) {
        for channel in &self.notification_channels {
            if let Err(e) = channel.send(alert) {
                log::error!("Failed to send alert via channel: {:?}", e);
            }
        }
    }
}
```

## 最佳实践

### 错误转换最佳实践

1. **保持语义一致性**: 确保转换后的错误类型在业务语义上准确
2. **避免信息丢失**: 转换过程中保留所有重要的错误信息
3. **建立转换映射**: 维护明确的错误类型转换映射关系
4. **文档化转换规则**: 为每种转换编写详细的文档说明

### 上下文管理最佳实践

1. **及早绑定**: 在错误发生时尽早绑定上下文信息
2. **信息分层**: 按照重要性和使用频率组织上下文信息
3. **性能考虑**: 避免在上下文中存储过大的数据结构
4. **隐私保护**: 确保敏感信息在上下文传播中得到适当处理

### 聚合策略最佳实践

1. **合理配置窗口**: 根据业务特点设置合适的聚合时间窗口
2. **动态调整**: 根据系统负载动态调整聚合策略
3. **采样优化**: 在高并发场景下使用适当的采样策略
4. **实时监控**: 建立实时的聚合指标监控和告警机制

## 相关文档

- [错误分类体系](./01-error-classification.md) - 不同类型错误的分类方法
- [错误处理策略](./02-handling-strategies.md) - 不同类型错误的处理策略
- [错误日志规范](./05-logging-standards.md) - 标准化错误日志字段格式
