    #[test]
    fn test_errcontext_from_string() {
        let ctx = CallContext::from(("key".to_string(), "test_string".to_string()));
        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0], ("key".to_string(), "test_string".to_string()));
    }

    #[test]
    fn test_errcontext_from_str() {
        let ctx = CallContext::from(("key", "test_str"));
        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0], ("key".to_string(), "test_str".to_string()));
    }

    #[test]
    fn test_errcontext_from_string_pair() {
        let ctx = CallContext::from(("key1".to_string(), "value1".to_string()));
        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0], ("key1".to_string(), "value1".to_string()));
    }

    #[test]
    fn test_errcontext_from_str_pair() {
        let ctx = CallContext::from(("key1", "value1"));
        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0], ("key1".to_string(), "value1".to_string()));
    }

    #[test]
    fn test_errcontext_from_mixed_pair() {
        let ctx = CallContext::from(("key1", "value1".to_string()));
        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0], ("key1".to_string(), "value1".to_string()));
    }

    #[test]
    fn test_errcontext_default() {
        let ctx = CallContext::default();
        assert_eq!(ctx.items.len(), 0);
    }

    #[test]
    fn test_errcontext_display_single() {
        let ctx = CallContext::from(("key", "test"));
        let display = format!("{ctx}");
        assert!(display.contains("call context:"));
        assert!(display.contains("key : test"));
    }

    #[test]
    fn test_errcontext_display_multiple() {
        let mut ctx = CallContext::default();
        ctx.items.push(("key1".to_string(), "value1".to_string()));
        ctx.items.push(("key2".to_string(), "value2".to_string()));
        let display = format!("{ctx}");
        assert!(display.contains("call context:"));
        assert!(display.contains("key1 : value1"));
        assert!(display.contains("key2 : value2"));
    }

    #[test]
    fn test_errcontext_display_empty() {
        let ctx = CallContext::default();
        let display = format!("{ctx}");
        assert_eq!(display, "");
    }

    #[test]
    fn test_withcontext_from_string() {
        let ctx = OperationContext::from("test_string".to_string());
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key".to_string(), "test_string".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_str() {
        let ctx = OperationContext::from("test_str".to_string());
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key".to_string(), "test_str".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_string_pair() {
        let ctx = OperationContext::from(("key1".to_string(), "value1".to_string()));
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_str_pair() {
        let ctx = OperationContext::from(("key1", "value1"));
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_mixed_pair() {
        let ctx = OperationContext::from(("key1", "value1".to_string()));
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_errcontext() {
        let err_ctx = CallContext::from(("key1", "value1"));
        let ctx = OperationContext::from(err_ctx);
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_withcontext() {
        let mut ctx1 = OperationContext::doing("target1");
        ctx1.record("key1", "value1");
        ctx1.with_doing("step1");
        let ctx2 = OperationContext::from(&ctx1);
        assert_eq!(ctx2.compat_target(), Some("target1".to_string()));
        assert_eq!(ctx2.path(), &["target1".to_string(), "step1".to_string()]);
        assert_eq!(ctx2.context().items.len(), 1);
        assert_eq!(
            ctx2.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
    }

    #[test]
    fn test_errcontext_equality() {
        let ctx1 = CallContext::from(("key1", "value1"));
        let ctx2 = CallContext::from(("key1", "value1"));
        let ctx3 = CallContext::from(("key1", "value2"));

        assert_eq!(ctx1, ctx2);
        assert_ne!(ctx1, ctx3);
    }

    #[test]
    fn test_withcontext_equality() {
        let ctx1 = OperationContext::from(("key1", "value1"));
        let ctx2 = OperationContext::from(("key1", "value1"));
        let ctx3 = OperationContext::from(("key1", "value2"));

        assert_eq!(ctx1, ctx2);
        assert_ne!(ctx1, ctx3);
    }

    #[test]
    fn test_withcontext_clone() {
        let mut ctx = OperationContext::doing("target");
        ctx.record("key", "value");

        let cloned = ctx.clone();
        assert_eq!(ctx.compat_target(), cloned.compat_target());
        assert_eq!(ctx.context().items.len(), cloned.context().items.len());
        assert_eq!(ctx.context().items[0], cloned.context().items[0]);
    }

    #[test]
    fn test_withcontext_with_types() {
        let mut ctx = OperationContext::new();

        // 测试各种类型转换
        ctx.record("string_key", "string_value");
        ctx.record("string_key", 42.to_string()); // 数字转字符串
        ctx.record("bool_key", true.to_string()); // 布尔转字符串

        assert_eq!(ctx.context().items.len(), 3);

        // 验证最后一个添加的值
        assert_eq!(
            ctx.context().items[2],
            ("bool_key".to_string(), "true".to_string())
        );
    }

    #[test]
    fn test_context_from_various_types() {
        // 测试从各种类型创建OperationContext
        let ctx1 = OperationContext::from("simple_string");
        assert_eq!(
            ctx1.context().items[0],
            ("key".to_string(), "simple_string".to_string())
        );

        let ctx2 = OperationContext::from(("custom_key", "custom_value"));
        assert_eq!(
            ctx2.context().items[0],
            ("custom_key".to_string(), "custom_value".to_string())
        );

        let path = PathBuf::from("/test/path/file.txt");
        let ctx3 = OperationContext::from(&path);
        assert!(ctx3.context().items[0].0.contains("path"));
        assert!(ctx3.context().items[0].1.contains("/test/path/file.txt"));
    }

    // ContextTake trait 测试用例
    #[test]
    fn test_context_take_with_string_types() {
        let mut ctx = OperationContext::new();

        // 测试字符串类型的ContextTake实现
        ctx.record("string_key", "string_value");
        ctx.record("string_key2", String::from("string_value2"));
        ctx.record(String::from("string_key3"), "string_value3");
        ctx.record(String::from("string_key4"), String::from("string_value4"));

        assert_eq!(ctx.context().items.len(), 4);
        assert_eq!(
            ctx.context().items[0],
            ("string_key".to_string(), "string_value".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("string_key2".to_string(), "string_value2".to_string())
        );
        assert_eq!(
            ctx.context().items[2],
            ("string_key3".to_string(), "string_value3".to_string())
        );
        assert_eq!(
            ctx.context().items[3],
            ("string_key4".to_string(), "string_value4".to_string())
        );
    }

    #[test]
    fn test_context_take_with_numeric_types() {
        let mut ctx = OperationContext::new();

        // 测试数字类型的ContextTake实现（需要转换为字符串）
        ctx.record("int_key", 42.to_string());
        ctx.record("float_key", 3.24.to_string());
        ctx.record("bool_key", true.to_string());

        assert_eq!(ctx.context().items.len(), 3);
        assert_eq!(
            ctx.context().items[0],
            ("int_key".to_string(), "42".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("float_key".to_string(), "3.24".to_string())
        );
        assert_eq!(
            ctx.context().items[2],
            ("bool_key".to_string(), "true".to_string())
        );
    }

    #[test]
    fn test_context_take_mixed_types() {
        let mut ctx = OperationContext::new();

        // 测试混合使用字符串和PathContext类型
        ctx.record("name", "test_user");
        ctx.record("age", 25.to_string());
        ctx.record("config_file", PathBuf::from("/etc/config.toml").display());
        ctx.record("status", "active");

        assert_eq!(ctx.context().items.len(), 4);
        assert_eq!(
            ctx.context().items[0],
            ("name".to_string(), "test_user".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("age".to_string(), "25".to_string())
        );
        assert_eq!(ctx.context().items[2].0, "config_file");
        assert!(ctx.context().items[2].1.contains("/etc/config.toml"));
        assert_eq!(
            ctx.context().items[3],
            ("status".to_string(), "active".to_string())
        );
    }

    #[test]
    fn test_context_take_edge_cases() {
        let mut ctx = OperationContext::new();

        // 测试边界情况
        ctx.record("", ""); // 空字符串
        ctx.record("empty_value", ""); // 空值
        ctx.record("", "empty_key"); // 空键
        ctx.record("special_chars", "@#$%^&*()"); // 特殊字符
        ctx.record("unicode", "测试中文字符"); // Unicode字符

        assert_eq!(ctx.context().items.len(), 5);
        assert_eq!(ctx.context().items[0], ("".to_string(), "".to_string()));
        assert_eq!(
            ctx.context().items[1],
            ("empty_value".to_string(), "".to_string())
        );
        assert_eq!(
            ctx.context().items[2],
            ("".to_string(), "empty_key".to_string())
        );
        assert_eq!(
            ctx.context().items[3],
            ("special_chars".to_string(), "@#$%^&*()".to_string())
        );
        assert_eq!(
            ctx.context().items[4],
            ("unicode".to_string(), "测试中文字符".to_string())
        );
    }

    #[test]
    fn test_context_take_multiple_calls() {
        let mut ctx = OperationContext::new();

        // 测试多次调用take方法
        ctx.record("key1", "value1");
        ctx.record("key2", "value2");
        ctx.record("key1", "new_value1"); // 覆盖key1
        ctx.record("key3", PathBuf::from("/path/file.txt").display());
        ctx.record("key2", PathBuf::from("/path/file2.txt").display()); // 覆盖key2

        // 注意：当前实现允许重复的key，这是预期的行为
        assert_eq!(ctx.context().items.len(), 5);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("key2".to_string(), "value2".to_string())
        );
        assert_eq!(
            ctx.context().items[2],
            ("key1".to_string(), "new_value1".to_string())
        );
        assert_eq!(ctx.context().items[3].0, "key3");
        assert!(ctx.context().items[3].1.contains("/path/file.txt"));
        assert_eq!(ctx.context().items[4].0, "key2");
        assert!(ctx.context().items[4].1.contains("/path/file2.txt"));
    }

    #[test]
    fn test_context_take_with_existing_context() {
        // 创建一个已有上下文的OperationContext
        let mut ctx = OperationContext::from(("existing_key", "existing_value"));

        // 使用ContextTake添加更多上下文
        ctx.record("new_key1", "new_value1");
        ctx.record("new_key2", PathBuf::from("/new/path.txt").display());

        assert_eq!(ctx.context().items.len(), 3);
        assert_eq!(
            ctx.context().items[0],
            ("existing_key".to_string(), "existing_value".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("new_key1".to_string(), "new_value1".to_string())
        );
        assert_eq!(ctx.context().items[2].0, "new_key2");
        assert!(ctx.context().items[2].1.contains("/new/path.txt"));
    }
