
    #[test]
    fn test_withcontext_new() {
        let ctx = OperationContext::new();
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 0);
    }

    #[test]
    fn test_withcontext_with() {
        let mut ctx = OperationContext::new();
        ctx.record("key1", "value1");
        ctx.record("key2", "value2");

        assert_eq!(ctx.context().items.len(), 2);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("key2".to_string(), "value2".to_string())
        );
    }

    #[test]
    fn test_withcontext_edge_cases() {
        let ctx1 = OperationContext::from("".to_string());
        assert_eq!(ctx1.context().items.len(), 1);
        assert_eq!(ctx1.context().items[0], ("key".to_string(), "".to_string()));

        let ctx2 = OperationContext::from(("".to_string(), "".to_string()));
        assert_eq!(ctx2.context().items.len(), 1);
        assert_eq!(ctx2.context().items[0], ("".to_string(), "".to_string()));
    }

    #[test]
    fn test_context_add_trait() {
        let mut ctx = OperationContext::new();

        // 测试ContextAdd trait的实现
        ctx.add_context(("key1", "value1"));
        ctx.add_context(("key2", "value2"));

        assert_eq!(ctx.context().items.len(), 2);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("key2".to_string(), "value2".to_string())
        );
    }

    #[test]
    fn test_context_with_special_characters() {
        let mut ctx = OperationContext::new();

        // 测试特殊字符
        ctx.record("key_with_spaces", "value with spaces");
        ctx.record("key_with_unicode", "值包含中文");
        ctx.record("key_with_symbols", "value@#$%^&*()");

        assert_eq!(ctx.context().items.len(), 3);
        assert_eq!(
            ctx.context().items[0],
            (
                "key_with_spaces".to_string(),
                "value with spaces".to_string()
            )
        );
        assert_eq!(
            ctx.context().items[1],
            ("key_with_unicode".to_string(), "值包含中文".to_string())
        );
        assert_eq!(
            ctx.context().items[2],
            ("key_with_symbols".to_string(), "value@#$%^&*()".to_string())
        );

        // 测试显示
        let display = format!("{ctx}");
        assert!(display.contains("key_with_spaces"));
        assert!(display.contains("值包含中文"));
        assert!(display.contains("value@#$%^&*()"));
    }

    #[test]
    fn test_context_builder_pattern() {
        // 测试构建器模式的使用
        let ctx = OperationContext::doing("builder_test").with_auto_log();

        assert_eq!(ctx.compat_target(), Some("builder_test".to_string()));
        assert_eq!(ctx.path(), &["builder_test".to_string()]);
        assert!(ctx.exit_log);
    }

    #[test]
    fn test_context_multiple_with_calls() {
        let mut ctx = OperationContext::new();

        // 多次调用with方法
        ctx.record("key1", "value1");
        ctx.record("key2", "value2");
        ctx.record("key3", "value3");
        ctx.record("key1", "new_value1"); // 覆盖key1

        // 注意：当前实现允许重复的key，这是预期的行为
        assert_eq!(ctx.context().items.len(), 4);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
        assert_eq!(
            ctx.context().items[3],
            ("key1".to_string(), "new_value1".to_string())
        );
    }
