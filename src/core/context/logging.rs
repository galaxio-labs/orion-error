    #[test]
    fn test_mark_suc() {
        let mut ctx = OperationContext::new();
        assert!(ctx.result == OperationResult::Fail);

        ctx.mark_suc();
        assert!(ctx.result == OperationResult::Suc);
    }

    #[test]
    fn test_with_auto_log() {
        let ctx = OperationContext::new().with_auto_log();
        assert!(ctx.exit_log);

        let ctx2 = OperationContext::doing("test").with_auto_log();
        assert!(ctx2.exit_log);
        assert_eq!(ctx2.compat_target(), Some("test".to_string()));
    }

    #[test]
    fn test_scope_marks_success() {
        let mut ctx = OperationContext::doing("scope_success");
        {
            let _scope = ctx.scoped_success();
        }
        assert!(matches!(ctx.result(), OperationResult::Suc));
    }

    #[test]
    fn test_scope_preserves_failure() {
        let mut ctx = OperationContext::doing("scope_fail");
        {
            let mut scope = ctx.scoped_success();
            scope.mark_failure();
        }
        assert!(matches!(ctx.result(), OperationResult::Fail));
    }

    #[test]
    fn test_scope_cancel() {
        let mut ctx = OperationContext::doing("scope_cancel");
        {
            let mut scope = ctx.scoped_success();
            scope.cancel();
        }
        assert!(matches!(ctx.result(), OperationResult::Cancel));
    }

    #[test]
    fn test_logging_methods() {
        let ctx = OperationContext::doing("test_target");

        // 这些方法主要测试它们不会panic，实际日志输出需要日志框架支持
        ctx.info("info message");
        ctx.debug("debug message");
        ctx.warn("warn message");
        ctx.error("error message");
        ctx.trace("trace message");
    }

    #[test]
    fn test_logging_methods_with_empty_context() {
        let ctx = OperationContext::new();

        // 测试空上下文时的日志方法
        ctx.info("info message");
        ctx.debug("debug message");
        ctx.warn("warn message");
        ctx.error("error message");
        ctx.trace("trace message");
    }

    #[test]
    fn test_drop_trait_with_success() {
        {
            let mut ctx = OperationContext::doing("test_drop").with_auto_log();
            ctx.record("operation", "test");
            ctx.mark_suc(); // 标记为成功
                            // ctx 在这里离开作用域，会触发Drop trait
        }
        // 注意：Drop trait的日志输出需要日志框架配置才能看到
        // 这里主要测试Drop trait不会panic
    }

    #[test]
    fn test_drop_trait_with_failure() {
        {
            let mut ctx = OperationContext::doing("test_drop_fail").with_auto_log();
            ctx.record("operation", "test_fail");
            // 不调用mark_suc，保持is_suc = false
            // ctx 在这里离开作用域，会触发Drop trait
        }
        // 注意：Drop trait的日志输出需要日志框架配置才能看到
        // 这里主要测试Drop trait不会panic
    }

    #[test]
    fn test_drop_trait_without_exit_log() {
        {
            let mut ctx = OperationContext::doing("test_no_log");
            ctx.record("operation", "no_log");
            ctx.mark_suc();
            // exit_log = false，不会触发日志输出
            // ctx 在这里离开作用域，Drop trait应该什么都不做
        }
        // 测试通过即可
    }

    #[test]
    fn test_complex_context_scenario() {
        // 模拟一个复杂的操作场景
        let mut ctx = OperationContext::doing("user_registration").with_auto_log();

        // 添加各种上下文信息
        ctx.record("user_id", "12345");
        ctx.record("email", "test@example.com");
        ctx.record("role", "user");

        // 记录各种级别的日志
        ctx.info("开始用户注册流程");
        ctx.debug("验证用户输入");
        ctx.warn("检测到潜在的安全风险");

        // 模拟操作成功
        ctx.mark_suc();
        ctx.info("用户注册成功");

        // 验证上下文状态
        assert!(ctx.result == OperationResult::Suc);
        assert!(ctx.exit_log);
        assert_eq!(ctx.compat_target(), Some("user_registration".to_string()));
        assert_eq!(ctx.context().items.len(), 3);

        // 验证format_context输出
        let formatted = ctx.format_context();
        assert!(formatted.contains("user_registration"));
        assert!(formatted.contains("user_id"));
        assert!(formatted.contains("email"));
        assert!(formatted.contains("role"));
    }
