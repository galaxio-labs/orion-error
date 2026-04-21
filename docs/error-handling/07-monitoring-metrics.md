# 监控指标体系

> 历史设计说明：
> 本页保留的是监控与治理侧的设计示意，不是 `orion-error 0.6.x / V1 API` 的直接代码模板。
> 文中的指标名称、聚合方式和监控示例主要用于讨论观测策略，不能直接视为当前 crate 自动提供或稳定承诺的行为。
> 当前稳定的结构化输出请优先基于 `ErrorReport`、`source_frames`、`want/path` 等现有字段，参考 [使用教程](../tutorial.md) 和 [V1 修复与评审基线](../v1-fix-and-review-plan.md)。

## 概述

监控指标体系是错误处理系统的重要组成部分，通过对关键指标的实时监控和分析，可以及时发现系统异常、预测潜在问题，并为系统优化提供数据支持。本体系定义了全面的监控指标、告警阈值和数据分析方法。

## 核心监控指标

### 错误类指标

#### 订单相关错误指标
```rust
use prometheus::{Counter, Histogram, Gauge, Registry};

pub struct OrderErrorMetrics {
    // 错误总量统计
    pub order_failure_total: Counter,
    pub order_success_total: Counter,
    pub order_retry_total: Counter,
    
    // 错误率统计
    pub order_error_rate: Gauge,
    pub order_retry_rate: Gauge,
    
    // 错误分类统计
    pub order_validation_failures: Counter,
    pub order_payment_failures: Counter,
    pub order_inventory_failures: Counter,
    pub order_system_failures: Counter,
    
    // 处理时间统计
    pub order_processing_time: Histogram,
    pub order_error_handling_time: Histogram,
}

impl OrderErrorMetrics {
    pub fn new(registry: &Registry) -> Self {
        let order_failure_total = Counter::new(
            "order_failure_total",
            "Total number of failed order processing attempts"
        ).expect("Failed to create order_failure_total counter");

        let order_success_total = Counter::new(
            "order_success_total", 
            "Total number of successful order processing attempts"
        ).expect("Failed to create order_success_total counter");

        let order_retry_total = Counter::new(
            "order_retry_total",
            "Total number of order processing retry attempts"
        ).expect("Failed to create order_retry_total counter");

        let order_error_rate = Gauge::new(
            "order_error_rate",
            "Current order processing error rate (errors per minute)"
        ).expect("Failed to create order_error_rate gauge");

        let order_retry_rate = Gauge::new(
            "order_retry_rate",
            "Current order processing retry rate (retries per minute)"
        ).expect("Failed to create order_retry_rate gauge");

        let order_validation_failures = Counter::new(
            "order_validation_failures_total",
            "Total number of order validation failures"
        ).expect("Failed to create order_validation_failures counter");

        let order_payment_failures = Counter::new(
            "order_payment_failures_total",
            "Total number of order payment failures"
        ).expect("Failed to create order_payment_failures counter");

        let order_inventory_failures = Counter::new(
            "order_inventory_failures_total",
            "Total number of order inventory failures"
        ).expect("Failed to create order_inventory_failures counter");

        let order_system_failures = Counter::new(
            "order_system_failures_total",
            "Total number of order system failures"
        ).expect("Failed to create order_system_failures counter");

        let order_processing_time = Histogram::new(
            "order_processing_time_seconds",
            "Time spent processing orders"
        ).expect("Failed to create order_processing_time histogram");

        let order_error_handling_time = Histogram::new(
            "order_error_handling_time_seconds", 
            "Time spent handling order errors"
        ).expect("Failed to create order_error_handling_time histogram");

        // 注册所有指标
        registry.register(Box::new(order_failure_total.clone())).unwrap();
        registry.register(Box::new(order_success_total.clone())).unwrap();
        registry.register(Box::new(order_retry_total.clone())).unwrap();
        registry.register(Box::new(order_error_rate.clone())).unwrap();
        registry.register(Box::new(order_retry_rate.clone())).unwrap();
        registry.register(Box::new(order_validation_failures.clone())).unwrap();
        registry.register(Box::new(order_payment_failures.clone())).unwrap();
        registry.register(Box::new(order_inventory_failures.clone())).unwrap();
        registry.register(Box::new(order_system_failures.clone())).unwrap();
        registry.register(Box::new(order_processing_time.clone())).unwrap();
        registry.register(Box::new(order_error_handling_time.clone())).unwrap();

        Self {
            order_failure_total,
            order_success_total,
            order_retry_total,
            order_error_rate,
            order_retry_rate,
            order_validation_failures,
            order_payment_failures,
            order_inventory_failures,
            order_system_failures,
            order_processing_time,
            order_error_handling_time,
        }
    }

    pub fn record_order_success(&self, processing_time: f64) {
        self.order_success_total.inc();
        self.order_processing_time.observe(processing_time);
    }

    pub fn record_order_failure(&self, error_type: &str, processing_time: f64, handling_time: f64) {
        self.order_failure_total.inc();
        self.order_processing_time.observe(processing_time);
        self.order_error_handling_time.observe(handling_time);

        match error_type {
            "validation" => self.order_validation_failures.inc(),
            "payment" => self.order_payment_failures.inc(),
            "inventory" => self.order_inventory_failures.inc(),
            "system" => self.order_system_failures.inc(),
            _ => {}
        }
    }

    pub fn record_order_retry(&self) {
        self.order_retry_total.inc();
    }

    pub fn update_error_rate(&self, error_rate: f64) {
        self.order_error_rate.set(error_rate);
    }

    pub fn update_retry_rate(&self, retry_rate: f64) {
        self.order_retry_rate.set(retry_rate);
    }
}
```

#### 系统资源监控指标
```rust
pub struct SystemResourceMetrics {
    // 存储使用情况
    pub storage_usage_bytes: Gauge,
    pub storage_available_bytes: Gauge,
    pub storage_usage_percent: Gauge,
    
    // 数据库连接情况
    pub database_connections_active: Gauge,
    pub database_connections_idle: Gauge,
    pub database_connections_total: Gauge,
    pub database_connection_errors: Counter,
    
    // HTTP客户端监控
    pub http_requests_total: Counter,
    pub http_request_errors: Counter,
    pub http_request_duration: Histogram,
    pub http_client_errors_4xx: Counter,
    pub http_client_errors_5xx: Counter,
    
    // 内存和CPU使用
    pub memory_usage_bytes: Gauge,
    pub cpu_usage_percent: Gauge,
    pub goroutines_count: Gauge,
    pub thread_count: Gauge,
}

impl SystemResourceMetrics {
    pub fn new(registry: &Registry) -> Self {
        let storage_usage_bytes = Gauge::new(
            "storage_usage_bytes",
            "Total storage usage in bytes"
        ).expect("Failed to create storage_usage_bytes gauge");

        let storage_available_bytes = Gauge::new(
            "storage_available_bytes", 
            "Total available storage in bytes"
        ).expect("Failed to create storage_available_bytes gauge");

        let storage_usage_percent = Gauge::new(
            "storage_usage_percent",
            "Storage usage percentage"
        ).expect("Failed to create storage_usage_percent gauge");

        let database_connections_active = Gauge::new(
            "database_connections_active",
            "Number of active database connections"
        ).expect("Failed to create database_connections_active gauge");

        let database_connections_idle = Gauge::new(
            "database_connections_idle",
            "Number of idle database connections"
        ).expect("Failed to create database_connections_idle gauge");

        let database_connections_total = Gauge::new(
            "database_connections_total",
            "Total number of database connections"
        ).expect("Failed to create database_connections_total gauge");

        let database_connection_errors = Counter::new(
            "database_connection_errors_total",
            "Total number of database connection errors"
        ).expect("Failed to create database_connection_errors counter");

        let http_requests_total = Counter::new(
            "http_requests_total",
            "Total number of HTTP requests"
        ).expect("Failed to create http_requests_total counter");

        let http_request_errors = Counter::new(
            "http_request_errors_total",
            "Total number of HTTP request errors"
        ).expect("Failed to create http_request_errors counter");

        let http_request_duration = Histogram::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds"
        ).expect("Failed to create http_request_duration histogram");

        let http_client_errors_4xx = Counter::new(
            "http_client_errors_4xx_total",
            "Total number of HTTP 4xx client errors"
        ).expect("Failed to create http_client_errors_4xx counter");

        let http_client_errors_5xx = Counter::new(
            "http_client_errors_5xx_total",
            "Total number of HTTP 5xx server errors"
        ).expect("Failed to create http_client_errors_5xx counter");

        let memory_usage_bytes = Gauge::new(
            "memory_usage_bytes",
            "Current memory usage in bytes"
        ).expect("Failed to create memory_usage_bytes gauge");

        let cpu_usage_percent = Gauge::new(
            "cpu_usage_percent",
            "Current CPU usage percentage"
        ).expect("Failed to create cpu_usage_percent gauge");

        let goroutines_count = Gauge::new(
            "goroutines_count",
            "Number of active goroutines"
        ).expect("Failed to create goroutines_count gauge");

        let thread_count = Gauge::new(
            "thread_count",
            "Number of active threads"
        ).expect("Failed to create thread_count gauge");

        // 注册所有指标
        registry.register(Box::new(storage_usage_bytes.clone())).unwrap();
        registry.register(Box::new(storage_available_bytes.clone())).unwrap();
        registry.register(Box::new(storage_usage_percent.clone())).unwrap();
        registry.register(Box::new(database_connections_active.clone())).unwrap();
        registry.register(Box::new(database_connections_idle.clone())).unwrap();
        registry.register(Box::new(database_connections_total.clone())).unwrap();
        registry.register(Box::new(database_connection_errors.clone())).unwrap();
        registry.register(Box::new(http_requests_total.clone())).unwrap();
        registry.register(Box::new(http_request_errors.clone())).unwrap();
        registry.register(Box::new(http_request_duration.clone())).unwrap();
        registry.register(Box::new(http_client_errors_4xx.clone())).unwrap();
        registry.register(Box::new(http_client_errors_5xx.clone())).unwrap();
        registry.register(Box::new(memory_usage_bytes.clone())).unwrap();
        registry.register(Box::new(cpu_usage_percent.clone())).unwrap();
        registry.register(Box::new(goroutines_count.clone())).unwrap();
        registry.register(Box::new(thread_count.clone())).unwrap();

        Self {
            storage_usage_bytes,
            storage_available_bytes,
            storage_usage_percent,
            database_connections_active,
            database_connections_idle,
            database_connections_total,
            database_connection_errors,
            http_requests_total,
            http_request_errors,
            http_request_duration,
            http_client_errors_4xx,
            http_client_errors_5xx,
            memory_usage_bytes,
            cpu_usage_percent,
            goroutines_count,
            thread_count,
        }
    }

    pub fn update_storage_metrics(&self, used_bytes: u64, total_bytes: u64) {
        self.storage_usage_bytes.set(used_bytes as f64);
        self.storage_available_bytes.set((total_bytes - used_bytes) as f64);
        
        let usage_percent = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };
        self.storage_usage_percent.set(usage_percent);
    }

    pub fn update_database_connections(&self, active: u32, idle: u32) {
        self.database_connections_active.set(active as f64);
        self.database_connections_idle.set(idle as f64);
        self.database_connections_total.set((active + idle) as f64);
    }

    pub fn record_database_connection_error(&self) {
        self.database_connection_errors.inc();
    }

    pub fn record_http_request(&self, duration: f64, status_code: u16) {
        self.http_requests_total.inc();
        self.http_request_duration.observe(duration);

        match status_code {
            400..=499 => self.http_client_errors_4xx.inc(),
            500..=599 => self.http_client_errors_5xx.inc(),
            _ => {}
        }
    }

    pub fn record_http_request_error(&self) {
        self.http_request_errors.inc();
    }

    pub fn update_system_resources(&self, memory_bytes: u64, cpu_percent: f64, goroutines: u64, threads: u64) {
        self.memory_usage_bytes.set(memory_bytes as f64);
        self.cpu_usage_percent.set(cpu_percent);
        self.goroutines_count.set(goroutines as f64);
        self.thread_count.set(threads as f64);
    }
}
```

### 业务健康度指标
```rust
pub struct BusinessHealthMetrics {
    // 服务可用性
    pub service_availability: Gauge,
    pub service_uptime: Counter,
    pub service_downtime: Counter,
    
    // 业务成功率
    pub business_success_rate: Gauge,
    pub business_throughput: Gauge,
    pub business_latency_p50: Histogram,
    pub business_latency_p95: Histogram,
    pub business_latency_p99: Histogram,
    
    // 错误恢复情况
    pub error_recovery_success_rate: Gauge,
    pub error_recovery_time: Histogram,
    pub automatic_recovery_count: Counter,
    pub manual_recovery_count: Counter,
    
    // 用户体验指标
    pub user_satisfaction_score: Gauge,
    pub user_error_reports: Counter,
    pub user_complaints: Counter,
    pub feature_adoption_rate: Gauge,
}

impl BusinessHealthMetrics {
    pub fn new(registry: &Registry) -> Self {
        let service_availability = Gauge::new(
            "service_availability_percent",
            "Service availability percentage"
        ).expect("Failed to create service_availability gauge");

        let service_uptime = Counter::new(
            "service_uptime_seconds_total",
            "Total service uptime in seconds"
        ).expect("Failed to create service_uptime counter");

        let service_downtime = Counter::new(
            "service_downtime_seconds_total", 
            "Total service downtime in seconds"
        ).expect("Failed to create service_downtime counter");

        let business_success_rate = Gauge::new(
            "business_success_rate_percent",
            "Business operation success rate percentage"
        ).expect("Failed to create business_success_rate gauge");

        let business_throughput = Gauge::new(
            "business_throughput_per_minute",
            "Business operations processed per minute"
        ).expect("Failed to create business_throughput gauge");

        let business_latency_p50 = Histogram::new(
            "business_latency_p50_seconds",
            "Business operation 50th percentile latency"
        ).expect("Failed to create business_latency_p50 histogram");

        let business_latency_p95 = Histogram::new(
            "business_latency_p95_seconds",
            "Business operation 95th percentile latency"
        ).expect("Failed to create business_latency_p95 histogram");

        let business_latency_p99 = Histogram::new(
            "business_latency_p99_seconds",
            "Business operation 99th percentile latency"
        ).expect("Failed to create business_latency_p99 histogram");

        let error_recovery_success_rate = Gauge::new(
            "error_recovery_success_rate_percent",
            "Error recovery success rate percentage"
        ).expect("Failed to create error_recovery_success_rate gauge");

        let error_recovery_time = Histogram::new(
            "error_recovery_time_seconds",
            "Time taken to recover from errors"
        ).expect("Failed to create error_recovery_time histogram");

        let automatic_recovery_count = Counter::new(
            "automatic_recovery_count_total",
            "Total number of automatic error recoveries"
        ).expect("Failed to create automatic_recovery_count counter");

        let manual_recovery_count = Counter::new(
            "manual_recovery_count_total",
            "Total number of manual error recoveries"
        ).expect("Failed to create manual_recovery_count counter");

        let user_satisfaction_score = Gauge::new(
            "user_satisfaction_score",
            "User satisfaction score (0-100)"
        ).expect("Failed to create user_satisfaction_score gauge");

        let user_error_reports = Counter::new(
            "user_error_reports_total",
            "Total number of user error reports"
        ).expect("Failed to create user_error_reports counter");

        let user_complaints = Counter::new(
            "user_complaints_total",
            "Total number of user complaints"
        ).expect("Failed to create user_complaints counter");

        let feature_adoption_rate = Gauge::new(
            "feature_adoption_rate_percent",
            "Feature adoption rate percentage"
        ).expect("Failed to create feature_adoption_rate gauge");

        // 注册所有指标
        registry.register(Box::new(service_availability.clone())).unwrap();
        registry.register(Box::new(service_uptime.clone())).unwrap();
        registry.register(Box::new(service_downtime.clone())).unwrap();
        registry.register(Box::new(business_success_rate.clone())).unwrap();
        registry.register(Box::new(business_throughput.clone())).unwrap();
        registry.register(Box::new(business_latency_p50.clone())).unwrap();
        registry.register(Box::new(business_latency_p95.clone())).unwrap();
        registry.register(Box::new(business_latency_p99.clone())).unwrap();
        registry.register(Box::new(error_recovery_success_rate.clone())).unwrap();
        registry.register(Box::new(error_recovery_time.clone())).unwrap();
        registry.register(Box::new(automatic_recovery_count.clone())).unwrap();
        registry.register(Box::new(manual_recovery_count.clone())).unwrap();
        registry.register(Box::new(user_satisfaction_score.clone())).unwrap();
        registry.register(Box::new(user_error_reports.clone())).unwrap();
        registry.register(Box::new(user_complaints.clone())).unwrap();
        registry.register(Box::new(feature_adoption_rate.clone())).unwrap();

        Self {
            service_availability,
            service_uptime,
            service_downtime,
            business_success_rate,
            business_throughput,
            business_latency_p50,
            business_latency_p95,
            business_latency_p99,
            error_recovery_success_rate,
            error_recovery_time,
            automatic_recovery_count,
            manual_recovery_count,
            user_satisfaction_score,
            user_error_reports,
            user_complaints,
            feature_adoption_rate,
        }
    }

    pub fn update_service_availability(&self, availability_percent: f64) {
        self.service_availability.set(availability_percent);
    }

    pub fn add_uptime(&self, seconds: u64) {
        self.service_uptime.inc_by(seconds as f64);
    }

    pub fn add_downtime(&self, seconds: u64) {
        self.service_downtime.inc_by(seconds as f64);
    }

    pub fn update_business_metrics(&self, success_rate: f64, throughput: f64) {
        self.business_success_rate.set(success_rate);
        self.business_throughput.set(throughput);
    }

    pub fn record_business_latency(&self, latency: f64) {
        self.business_latency_p50.observe(latency);
        self.business_latency_p95.observe(latency);
        self.business_latency_p99.observe(latency);
    }

    pub fn record_error_recovery(&self, success: bool, recovery_time: f64, automatic: bool) {
        if success {
            // 更新恢复成功率逻辑
        }
        self.error_recovery_time.observe(recovery_time);
        
        if automatic {
            self.automatic_recovery_count.inc();
        } else {
            self.manual_recovery_count.inc();
        }
    }

    pub fn update_user_metrics(&self, satisfaction_score: f64) {
        self.user_satisfaction_score.set(satisfaction_score);
    }

    pub fn record_user_error_report(&self) {
        self.user_error_reports.inc();
    }

    pub fn record_user_complaint(&self) {
        self.user_complaints.inc();
    }

    pub fn update_feature_adoption(&self, adoption_rate: f64) {
        self.feature_adoption_rate.set(adoption_rate);
    }
}
```

## 告警阈值定义

### 基础告警规则
```yaml
# prometheus-alerts.yml
groups:
  - name: error_monitoring
    rules:
      # 高错误率告警
      - alert: HighOrderErrorRate
        expr: rate(order_failure_total[5m]) > 10
        for: 5m
        labels:
          severity: critical
          category: order_processing
        annotations:
          summary: "订单处理错误率过高"
          description: "过去5分钟内订单错误率达到 {{ $value }} 错误/分钟，超过阈值 10 错误/分钟"
          runbook_url: "https://wiki.example.com/runbooks/high_order_error_rate"
          
      # 存储使用率告警
      - alert: HighStorageUsage
        expr: storage_usage_percent > 90
        for: 2m
        labels:
          severity: warning
          category: system_resource
        annotations:
          summary: "存储使用率过高"
          description: "存储使用率达到 {{ $value }}%，超过阈值 90%"
          runbook_url: "https://wiki.example.com/runbooks/high_storage_usage"
          
      # HTTP客户端错误告警
      - alert: HighHttpClientErrorRate
        expr: (rate(http_client_errors_5xx[5m]) / rate(http_requests_total[5m])) * 100 > 5
        for: 5m
        labels:
          severity: warning
          category: http_client
        annotations:
          summary: "HTTP客户端5xx错误率过高"
          description: "过去5分钟内HTTP客户端5xx错误率达到 {{ $value }}%，超过阈值 5%"
          runbook_url: "https://wiki.example.com/runbooks/high_http_error_rate"

      # 数据库连接问题告警
      - alert: DatabaseConnectionIssues
        expr: rate(database_connection_errors[5m]) > 5
        for: 2m
        labels:
          severity: critical
          category: database
        annotations:
          summary: "数据库连接问题"
          description: "过去5分钟内数据库连接错误率达到 {{ $value }} 错误/分钟，超过阈值 5 错误/分钟"
          runbook_url: "https://wiki.example.com/runbooks/database_connection_issues"

      # 服务可用性下降告警
      - alert: LowServiceAvailability
        expr: service_availability_percent < 95
        for: 10m
        labels:
          severity: critical
          category: service_health
        annotations:
          summary: "服务可用性下降"
          description: "服务可用性下降到 {{ $value }}%，低于阈值 95%"
          runbook_url: "https://wiki.example.com/runbooks/low_service_availability"
```

### 延迟和性能告警
```yaml
  - name: performance_monitoring
    rules:
      # 订单处理延迟告警
      - alert: HighOrderProcessingLatency
        expr: histogram_quantile(0.95, rate(order_processing_time_seconds_bucket[5m])) > 2.0
        for: 5m
        labels:
          severity: warning
          category: performance
        annotations:
          summary: "订单处理延迟过高"
          description: "订单处理95分位延迟达到 {{ $value }} 秒，超过阈值 2.0 秒"
          runbook_url: "https://wiki.example.com/runbooks/high_order_latency"
          
      # HTTP请求延迟告警
      - alert: HighHttpRequestLatency
        expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 1.0
        for: 5m
        labels:
          severity: warning
          category: performance
        annotations:
          summary: "HTTP请求延迟过高"
          description: "HTTP请求95分位延迟达到 {{ $value }} 秒，超过阈值 1.0 秒"
          runbook_url: "https://wiki.example.com/runbooks/high_http_latency"

      # 业务延迟告警
      - alert: HighBusinessLatencyP99
        expr: histogram_quantile(0.99, rate(business_latency_p99_seconds_bucket[5m])) > 5.0
        for: 5m
        labels:
          severity: critical
          category: business_performance
        annotations:
          summary: "业务操作99分位延迟过高"
          description: "业务操作99分位延迟达到 {{ $value }} 秒，超过阈值 5.0 秒"
          runbook_url: "https://wiki.example.com/runbooks/high_business_latency"
```

### 资源使用告警
```yaml
  - name: resource_monitoring
    rules:
      # 内存使用率告警
      - alert: HighMemoryUsage
        expr: (memory_usage_bytes / total_memory_bytes) * 100 > 85
        for: 5m
        labels:
          severity: warning
          category: system_resource
        annotations:
          summary: "内存使用率过高"
          description: "内存使用率达到 {{ $value }}%，超过阈值 85%"
          runbook_url: "https://wiki.example.com/runbooks/high_memory_usage"

      # CPU使用率告警
      - alert: HighCpuUsage
        expr: cpu_usage_percent > 80
        for: 10m
        labels:
          severity: warning
          category: system_resource
        annotations:
          summary: "CPU使用率过高"
          description: "CPU使用率达到 {{ $value }}%，超过阈值 80%"
          runbook_url: "https://wiki.example.com/runbooks/high_cpu_usage"

      # 数据库连接池告警
      - alert: DatabaseConnectionPoolHigh
        expr: database_connections_active / database_connections_total * 100 > 90
        for: 2m
        labels:
          severity: warning
          category: database
        annotations:
          summary: "数据库连接池使用率过高"
          description: "数据库连接池使用率达到 {{ $value }}%，超过阈值 90%"
          runbook_url: "https://wiki.example.com/runbooks/database_pool_high"
```

## 监控数据收集

### 指标收集器
```rust
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use sysinfo::{System, SystemExt, ProcessExt, CpuExt};

pub struct MetricsCollector {
    order_metrics: Arc<OrderErrorMetrics>,
    system_metrics: Arc<SystemResourceMetrics>,
    business_metrics: Arc<BusinessHealthMetrics>,
    system_info: Arc<RwLock<System>>,
    collection_interval: Duration,
}

impl MetricsCollector {
    pub fn new(
        order_metrics: Arc<OrderErrorMetrics>,
        system_metrics: Arc<SystemResourceMetrics>,
        business_metrics: Arc<BusinessHealthMetrics>,
        collection_interval: Duration,
    ) -> Self {
        Self {
            order_metrics,
            system_metrics,
            business_metrics,
            system_info: Arc::new(RwLock::new(System::new_all())),
            collection_interval,
        }
    }

    pub async fn start_collection(&self) {
        let mut interval_timer = tokio::time::interval(self.collection_interval);
        
        loop {
            interval_timer.tick().await;
            
            if let Err(e) = self.collect_system_metrics().await {
                error!("收集系统指标失败: {}", e);
            }
            
            if let Err(e) = self.calculate_derived_metrics().await {
                error!("计算派生指标失败: {}", e);
            }
        }
    }

    async fn collect_system_metrics(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut system = self.system_info.write().await;
        system.refresh_all();

        // 更新系统资源指标
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let cpu_usage = system.global_cpu_info().cpu_usage();

        // 更新goroutine数量（如果是Go应用）或线程数量
        let thread_count = sysinfo::get_current_pid()
            .and_then(|pid| system.process(pid).map(|p| p.thread_count() as u64))
            .unwrap_or(0);

        self.system_metrics.update_system_resources(
            used_memory,
            cpu_usage,
            0, // goroutines count (Go specific)
            thread_count,
        );

        // 更新存储指标（示例：使用/tmp目录）
        if let Ok(tmp_stat) = std::fs::metadata("/tmp") {
            let total_space = std::fs::metadata("/").map(|m| m.len()).unwrap_or(0);
            let available_space = tmp_stat.len();
            let used_space = total_space.saturating_sub(available_space);
            
            self.system_metrics.update_storage_metrics(used_space, total_space);
        }

        // 更新数据库连接指标（需要从连接池获取）
        // 这里应该是实际数据库连接池的统计信息
        self.system_metrics.update_database_connections(5, 10);

        Ok(())
    }

    async fn calculate_derived_metrics(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 计算错误率
        let failure_rate = self.calculate_error_rate().await;
        self.order_metrics.update_error_rate(failure_rate);

        // 计算重试率
        let retry_rate = self.calculate_retry_rate().await;
        self.order_metrics.update_retry_rate(retry_rate);

        // 计算业务成功率
        let business_success_rate = self.calculate_business_success_rate().await;
        self.business_metrics.update_business_metrics(business_success_rate, 0.0);

        Ok(())
    }

    async fn calculate_error_rate(&self) -> f64 {
        // 这里应该实现实际的错误率计算逻辑
        // 例如：从时间窗口中统计错误数量
        0.0 // 示例值
    }

    async fn calculate_retry_rate(&self) -> f64 {
        // 这里应该实现实际的重试率计算逻辑
        0.0 // 示例值
    }

    async fn calculate_business_success_rate(&self) -> f64 {
        // 这里应该实现实际的业务成功率计算逻辑
        95.0 // 示例值
    }
}
```

### HTTP指标导出器
```rust
use axum::{extract, http::StatusCode, response::IntoResponse, Json};
use prometheus::{Encoder, TextEncoder};
use std::collections::HashMap;

pub struct MetricsExporter {
    registry: prometheus::Registry,
}

impl MetricsExporter {
    pub fn new(registry: prometheus::Registry) -> Self {
        Self { registry }
    }

    pub async fn export_metrics(&self) -> Result<impl IntoResponse, StatusCode> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        
        let mut buffer = Vec::new();
        if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
            error!("编码指标失败: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        let response = String::from_utf8(buffer).map_err(|e| {
            error!("指标字符串转换失败: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        Ok((
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4")],
            response,
        ))
    }

    pub async fn health_check(&self) -> Result<Json<HashMap<String, String>>, StatusCode> {
        let mut health_status = HashMap::new();
        health_status.insert("status".to_string(), "healthy".to_string());
        health_status.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        health_status.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        
        Ok(Json(health_status))
    }
}

// HTTP路由配置
pub fn create_metrics_routes() -> axum::Router {
    let registry = prometheus::Registry::new();
    
    // 创建所有指标
    let order_metrics = Arc::new(OrderErrorMetrics::new(&registry));
    let system_metrics = Arc::new(SystemResourceMetrics::new(&registry));
    let business_metrics = Arc::new(BusinessHealthMetrics::new(&registry));
    
    let exporter = MetricsExporter::new(registry);
    let collector = MetricsCollector::new(
        order_metrics,
        system_metrics,
        business_metrics,
        Duration::from_secs(30),
    );

    // 启动指标收集
    tokio::spawn(async move {
        collector.start_collection().await;
    });

    axum::Router::new()
        .route("/metrics", axum::routing::get(export_metrics))
        .route("/health", axum::routing::get(health_check))
        .with_state(exporter)
}

async fn export_metrics(
    extract::State(exporter): extract::State<MetricsExporter>,
) -> Result<impl IntoResponse, StatusCode> {
    exporter.export_metrics().await
}

async fn health_check(
    extract::State(exporter): extract::State<MetricsExporter>,
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    exporter.health_check().await
}
```

## 监控仪表板

### Grafana仪表板配置
```json
{
  "dashboard": {
    "id": null,
    "title": "错误处理监控仪表板",
    "description": "系统错误处理和健康度监控仪表板",
    "tags": ["error-handling", "monitoring"],
    "timezone": "browser",
    "panels": [
      {
        "id": 1,
        "title": "订单错误率",
        "type": "stat",
        "targets": [
          {
            "expr": "rate(order_failure_total[5m])",
            "legendFormat": "错误率 (错误/分钟)"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "none",
            "min": 0,
            "thresholds": {
              "steps": [
                {"color": "green", "value": 0},
                {"color": "yellow", "value": 5},
                {"color": "red", "value": 10}
              ]
            }
          }
        },
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0}
      },
      {
        "id": 2,
        "title": "存储使用率",
        "type": "gauge",
        "targets": [
          {
            "expr": "storage_usage_percent",
            "legendFormat": "使用率 (%)"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percent",
            "min": 0,
            "max": 100,
            "thresholds": {
              "steps": [
                {"color": "green", "value": 0},
                {"color": "yellow", "value": 70},
                {"color": "red", "value": 90}
              ]
            }
          }
        },
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0}
      },
      {
        "id": 3,
        "title": "HTTP客户端错误率",
        "type": "timeseries",
        "targets": [
          {
            "expr": "(rate(http_client_errors_5xx[5m]) / rate(http_requests_total[5m])) * 100",
            "legendFormat": "5xx错误率 (%)"
          },
          {
            "expr": "(rate(http_client_errors_4xx[5m]) / rate(http_requests_total[5m])) * 100",
            "legendFormat": "4xx错误率 (%)"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percent",
            "min": 0
          }
        },
        "gridPos": {"h": 8, "w": 24, "x": 0, "y": 8}
      },
      {
        "id": 4,
        "title": "订单处理延迟分布",
        "type": "histogram",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(order_processing_time_seconds_bucket[5m]))",
            "legendFormat": "P50"
          },
          {
            "expr": "histogram_quantile(0.95, rate(order_processing_time_seconds_bucket[5m]))",
            "legendFormat": "P95"
          },
          {
            "expr": "histogram_quantile(0.99, rate(order_processing_time_seconds_bucket[5m]))",
            "legendFormat": "P99"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "s",
            "min": 0
          }
        },
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 16}
      },
      {
        "id": 5,
        "title": "数据库连接状态",
        "type": "stat",
        "targets": [
          {
            "expr": "database_connections_active",
            "legendFormat": "活跃连接"
          },
          {
            "expr": "database_connections_idle",
            "legendFormat": "空闲连接"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 16}
      },
      {
        "id": 6,
        "title": "服务可用性",
        "type": "stat",
        "targets": [
          {
            "expr": "service_availability_percent",
            "legendFormat": "可用率 (%)"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percent",
            "min": 0,
            "max": 100,
            "thresholds": {
              "steps": [
                {"color": "red", "value": 0},
                {"color": "yellow", "value": 95},
                {"color": "green", "value": 99}
              ]
            }
          }
        },
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 24}
      },
      {
        "id": 7,
        "title": "业务成功率",
        "type": "stat",
        "targets": [
          {
            "expr": "business_success_rate",
            "legendFormat": "成功率 (%)"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percent",
            "min": 0,
            "max": 100,
            "thresholds": {
              "steps": [
                {"color": "red", "value": 0},
                {"color": "yellow", "value": 90},
                {"color": "green", "value": 99}
              ]
            }
          }
        },
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 24}
      }
    ],
    "time": {
      "from": "now-1h",
      "to": "now"
    },
    "refresh": "30s"
  }
}
```

## 数据分析和报告

### 错误分析器
```rust
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorAnalysisReport {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary: ErrorSummary,
    pub error_trends: ErrorTrends,
    pub error_patterns: Vec<ErrorPattern>,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorSummary {
    pub total_errors: u64,
    pub unique_error_types: u32,
    pub critical_errors: u64,
    pub warning_errors: u64,
    pub average_recovery_time: f64,
    pub error_rate_per_minute: f64,
    pub success_rate_percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorTrends {
    pub error_count_trend: TrendDirection,
    pub error_rate_trend: TrendDirection,
    pub recovery_time_trend: TrendDirection,
    pub user_impact_trend: TrendDirection,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub pattern_id: String,
    pub error_type: String,
    pub frequency: u64,
    pub avg_recovery_time: f64,
    pub impact_score: f64,
    pub related_errors: Vec<String>,
    pub first_occurrence: DateTime<Utc>,
    pub last_occurrence: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub priority: Priority,
    pub category: String,
    pub title: String,
    pub description: String,
    pub action_items: Vec<String>,
    pub expected_impact: String,
    pub estimated_effort: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

pub struct ErrorAnalyzer {
    metrics_collector: Arc<MetricsCollector>,
    historical_data: Arc<RwLock<Vec<ErrorDataPoint>>>,
}

#[derive(Debug, Clone)]
pub struct ErrorDataPoint {
    pub timestamp: DateTime<Utc>,
    pub error_count: u64,
    pub error_types: HashMap<String, u64>,
    pub recovery_times: Vec<f64>,
    pub success_count: u64,
}

impl ErrorAnalyzer {
    pub fn new(metrics_collector: Arc<MetricsCollector>) -> Self {
        Self {
            metrics_collector,
            historical_data: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn generate_report(&self, period_start: DateTime<Utc>, period_end: DateTime<Utc>) -> Result<ErrorAnalysisReport, Box<dyn std::error::Error>> {
        let data = self.collect_data_for_period(period_start, period_end).await?;
        
        let summary = self.calculate_summary(&data).await;
        let trends = self.analyze_trends(&data).await;
        let patterns = self.identify_patterns(&data).await;
        let recommendations = self.generate_recommendations(&summary, &trends, &patterns).await;

        Ok(ErrorAnalysisReport {
            period_start,
            period_end,
            summary,
            trends,
            patterns,
            recommendations,
        })
    }

    async fn collect_data_for_period(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ErrorDataPoint>, Box<dyn std::error::Error>> {
        let data = self.historical_data.read().await;
        Ok(data.iter()
            .filter(|point| point.timestamp >= start && point.timestamp <= end)
            .cloned()
            .collect())
    }

    async fn calculate_summary(&self, data: &[ErrorDataPoint]) -> Result<ErrorSummary, Box<dyn std::error::Error>> {
        if data.is_empty() {
            return Ok(ErrorSummary {
                total_errors: 0,
                unique_error_types: 0,
                critical_errors: 0,
                warning_errors: 0,
                average_recovery_time: 0.0,
                error_rate_per_minute: 0.0,
                success_rate_percent: 0.0,
            });
        }

        let total_errors: u64 = data.iter().map(|d| d.error_count).sum();
        let total_success: u64 = data.iter().map(|d| d.success_count).sum();
        
        let all_error_types: HashMap<String, u64> = data.iter()
            .flat_map(|d| d.error_types.clone())
            .fold(HashMap::new(), |mut acc, (k, v)| {
                *acc.entry(k).or_insert(0) += v;
                acc
            });
        
        let unique_error_types = all_error_types.len() as u32;
        
        let all_recovery_times: Vec<f64> = data.iter()
            .flat_map(|d| d.recovery_times.clone())
            .collect();
        
        let average_recovery_time = if all_recovery_times.is_empty() {
            0.0
        } else {
            all_recovery_times.iter().sum::<f64>() / all_recovery_times.len() as f64
        };

        let period_minutes = data.len() as f64;
        let error_rate_per_minute = total_errors as f64 / period_minutes.max(1.0);
        
        let success_rate_percent = if total_errors + total_success > 0 {
            (total_success as f64 / (total_errors + total_success) as f64) * 100.0
        } else {
            100.0
        };

        // 简化的分类逻辑，实际应用中需要更复杂的分类
        let critical_errors = all_error_types.get("critical").copied().unwrap_or(0);
        let warning_errors = all_error_types.get("warning").copied().unwrap_or(0);

        Ok(ErrorSummary {
            total_errors,
            unique_error_types,
            critical_errors,
            warning_errors,
            average_recovery_time,
            error_rate_per_minute,
            success_rate_percent,
        })
    }

    async fn analyze_trends(&self, data: &[ErrorDataPoint]) -> Result<ErrorTrends, Box<dyn std::error::Error>> {
        if data.len() < 2 {
            return Ok(ErrorTrends {
                error_count_trend: TrendDirection::Stable,
                error_rate_trend: TrendDirection::Stable,
                recovery_time_trend: TrendDirection::Stable,
                user_impact_trend: TrendDirection::Stable,
            });
        }

        let error_count_trend = self.calculate_trend(data.iter().map(|d| d.error_count).collect());
        let error_rate_trend = self.calculate_trend(
            data.iter().map(|d| d.error_count as f64 / 60.0).collect()
        );
        
        let recovery_times: Vec<f64> = data.iter()
            .flat_map(|d| d.recovery_times.clone())
            .collect();
        let recovery_time_trend = self.calculate_trend(recovery_times);
        
        let user_impact_trend = self.calculate_trend(
            data.iter().map(|d| (d.error_count as f64 / (d.error_count + d.success_count) as f64) * 100.0).collect()
        );

        Ok(ErrorTrends {
            error_count_trend,
            error_rate_trend,
            recovery_time_trend,
            user_impact_trend,
        })
    }

    fn calculate_trend(&self, values: Vec<f64>) -> TrendDirection {
        if values.len() < 2 {
            return TrendDirection::Stable;
        }

        let first_half = &values[..values.len() / 2];
        let second_half = &values[values.len() / 2..];

        let first_avg: f64 = first_half.iter().sum::<f64>() / first_half.len() as f64;
        let second_avg: f64 = second_half.iter().sum::<f64>() / second_half.len() as f64;

        let change_ratio = if first_avg > 0.0 {
            (second_avg - first_avg) / first_avg
        } else {
            0.0
        };

        if change_ratio > 0.1 {
            TrendDirection::Increasing
        } else if change_ratio < -0.1 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        }
    }

    async fn identify_patterns(&self, data: &[ErrorDataPoint]) -> Result<Vec<ErrorPattern>, Box<dyn std::error::Error>> {
        let mut patterns = Vec::new();
        
        // 聚合错误类型
        let mut error_type_stats: HashMap<String, ErrorTypeStats> = HashMap::new();
        
        for point in data {
            for (error_type, count) in &point.error_types {
                let stats = error_type_stats.entry(error_type.clone()).or_insert(ErrorTypeStats::new());
                stats.frequency += count;
                stats.recovery_times.extend(point.recovery_times.clone());
                if stats.first_occurrence.is_none() || point.timestamp < stats.first_occurrence.unwrap() {
                    stats.first_occurrence = Some(point.timestamp);
                }
                if stats.last_occurrence.is_none() || point.timestamp > stats.last_occurrence.unwrap() {
                    stats.last_occurrence = Some(point.timestamp);
                }
            }
        }

        // 为每个错误类型创建模式
        for (error_type, stats) in error_type_stats {
            let avg_recovery_time = if stats.recovery_times.is_empty() {
                0.0
            } else {
                stats.recovery_times.iter().sum::<f64>() / stats.recovery_times.len() as f64
            };

            let impact_score = self.calculate_impact_score(stats.frequency, avg_recovery_time);

            patterns.push(ErrorPattern {
                pattern_id: format!("pattern_{}", patterns.len() + 1),
                error_type: error_type.clone(),
                frequency: stats.frequency,
                avg_recovery_time,
                impact_score,
                related_errors: self.find_related_errors(&error_type, data).await,
                first_occurrence: stats.first_occurrence.unwrap_or_else(Utc::now),
                last_occurrence: stats.last_occurrence.unwrap_or_else(Utc::now),
            });
        }

        // 按影响分数排序
        patterns.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(patterns)
    }

    fn calculate_impact_score(&self, frequency: u64, avg_recovery_time: f64) -> f64 {
        // 简化的影响分数计算，可以根据实际需求调整
        let frequency_score = (frequency as f64).ln();
        let recovery_time_score = avg_recovery_time;
        frequency_score * recovery_time_score
    }

    async fn find_related_errors(&self, error_type: &str, data: &[ErrorDataPoint]) -> Vec<String> {
        // 简化的关联错误查找逻辑
        // 实际应用中可以使用更复杂的关联分析算法
        vec![] // 示例返回空列表
    }

    async fn generate_recommendations(&self, summary: &ErrorSummary, trends: &ErrorTrends, patterns: &[ErrorPattern]) -> Result<Vec<Recommendation>, Box<dyn std::error::Error>> {
        let mut recommendations = Vec::new();

        // 基于摘要生成建议
        if summary.success_rate_percent < 95.0 {
            recommendations.push(Recommendation {
                id: "improve_success_rate".to_string(),
                priority: Priority::High,
                category: "business_health".to_string(),
                title: "提高业务成功率".to_string(),
                description: format!("当前业务成功率为 {:.2}%，低于95%的目标", summary.success_rate_percent),
                action_items: vec![
                    "分析失败案例，找出共同原因".to_string(),
                    "优化错误处理逻辑".to_string(),
                    "加强输入验证".to_string(),
                ],
                expected_impact: "将成功率提高到95%以上".to_string(),
                estimated_effort: "2-3周".to_string(),
            });
        }

        if summary.error_rate_per_minute > 5.0 {
            recommendations.push(Recommendation {
                id: "reduce_error_rate".to_string(),
                priority: Priority::Critical,
                category: "error_rate".to_string(),
                title: "降低错误率".to_string(),
                description: format!("当前错误率为 {:.2} 错误/分钟，超过5的阈值", summary.error_rate_per_minute),
                action_items: vec![
                    "实施更严格的输入验证".to_string(),
                    "增加系统资源监控".to_string(),
                    "优化错误重试机制".to_string(),
                ],
                expected_impact: "将错误率降低到5以下".to_string(),
                estimated_effort: "1-2周".to_string(),
            });
        }

        // 基于趋势生成建议
        if trends.error_count_trend == TrendDirection::Increasing {
            recommendations.push(Recommendation {
                id: "address_increasing_errors".to_string(),
                priority: Priority::High,
                category: "trend_analysis".to_string(),
                title: "处理错误增长趋势".to_string(),
                description: "错误数量呈现增长趋势，需要立即关注".to_string(),
                action_items: vec![
                    "分析错误增长的根本原因".to_string(),
                    "实施预防措施".to_string(),
                    "加强监控和告警".to_string(),
                ],
                expected_impact: "停止错误增长并开始下降".to_string(),
                estimated_effort: "1周".to_string(),
            });
        }

        // 基于错误模式生成建议
        for pattern in patterns.iter().take(3) { // 只处理前3个最重要的模式
            if pattern.impact_score > 10.0 {
                recommendations.push(Recommendation {
                    id: format!("fix_pattern_{}", pattern.pattern_id),
                    priority: Priority::Medium,
                    category: "pattern_fix".to_string(),
                    title: format!("修复错误模式: {}", pattern.error_type),
                    description: format!("错误类型 '{}' 具有较高影响分数 {:.2}", pattern.error_type, pattern.impact_score),
                    action_items: vec![
                        format!("调查 {} 错误的根本原因", pattern.error_type),
                        "实施针对性修复".to_string(),
                        "添加预防性检查".to_string(),
                    ],
                    expected_impact: "显著降低此类错误的发生频率".to_string(),
                    estimated_effort: "1-2周".to_string(),
                });
            }
        }

        Ok(recommendations)
    }
}

#[derive(Debug)]
struct ErrorTypeStats {
    frequency: u64,
    recovery_times: Vec<f64>,
    first_occurrence: Option<DateTime<Utc>>,
    last_occurrence: Option<DateTime<Utc>>,
}

impl ErrorTypeStats {
    fn new() -> Self {
        Self {
            frequency: 0,
            recovery_times: Vec::new(),
            first_occurrence: None,
            last_occurrence: None,
        }
    }
}
```

## 最佳实践

### 监控配置最佳实践
```rust
pub struct MonitoringConfig {
    pub collection_interval: Duration,
    pub retention_period: Duration,
    pub alert_thresholds: AlertThresholds,
    pub dashboard_refresh_interval: Duration,
    pub export_enabled: bool,
    pub sampling_rate: f64,
}

#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub critical_error_rate: f64,
    pub warning_error_rate: f64,
    pub high_storage_usage: f64,
    pub high_memory_usage: f64,
    pub high_cpu_usage: f64,
    pub high_latency_p95: f64,
    pub low_availability: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(30),
            retention_period: Duration::from_secs(86400 * 30), // 30天
            alert_thresholds: AlertThresholds {
                critical_error_rate: 10.0,
                warning_error_rate: 5.0,
                high_storage_usage: 90.0,
                high_memory_usage: 85.0,
                high_cpu_usage: 80.0,
                high_latency_p95: 2.0,
                low_availability: 95.0,
            },
            dashboard_refresh_interval: Duration::from_secs(30),
            export_enabled: true,
            sampling_rate: 1.0,
        }
    }
}

impl MonitoringConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        if let Ok(interval) = env::var("MONITORING_COLLECTION_INTERVAL") {
            if let Ok(seconds) = interval.parse::<u64>() {
                config.collection_interval = Duration::from_secs(seconds);
            }
        }
        
        if let Ok(enabled) = env::var("METRICS_EXPORT_ENABLED") {
            config.export_enabled = enabled.to_lowercase() == "true";
        }
        
        if let Ok(rate) = env::var("METRICS_SAMPLING_RATE") {
            if let Ok(sampling_rate) = rate.parse::<f64>() {
                config.sampling_rate = sampling_rate.clamp(0.0, 1.0);
            }
        }
        
        config
    }
    
    pub fn validate(&self) -> Result<(), String> {
        if self.collection_interval < Duration::from_secs(10) {
            return Err("Collection interval must be at least 10 seconds".to_string());
        }
        
        if self.alert_thresholds.critical_error_rate <= self.alert_thresholds.warning_error_rate {
            return Err("Critical error rate threshold must be greater than warning threshold".to_string());
        }
        
        if self.sampling_rate < 0.0 || self.sampling_rate > 1.0 {
            return Err("Sampling rate must be between 0.0 and 1.0".to_string());
        }
        
        Ok(())
    }
}
```

### 监控系统部署建议
1. **分层监控**: 建立基础设施、应用、业务三个层次的监控体系
2. **实时告警**: 确保关键错误能够实时触发告警
3. **定期巡检**: 每周分析监控数据，识别潜在问题
4. **容量规划**: 基于历史数据进行容量预测和规划
5. **灾备演练**: 定期进行监控系统的灾备演练

## 相关文档

- [错误分类体系](./01-error-classification.md) - 不同类型错误的分类方法
- [错误处理策略](./02-handling-strategies.md) - 不同类型错误的处理策略
- [错误归集机制](./03-error-aggregation.md) - 跨层错误转换方法
- [错误处理层级](./04-handling-layers.md) - 分层处理模型
- [错误日志规范](./05-logging-standards.md) - 标准化错误日志字段格式
- [最佳实践指南](./06-best-practices.md) - 综合最佳实践指南
- [错误恢复模式](./08-recovery-patterns.md) - 重试机制和容错模式
