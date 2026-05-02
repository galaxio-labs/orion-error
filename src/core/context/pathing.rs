    #[test]
    fn test_op_context_macro_sets_callsite_mod_path() {
        let ctx = op_context!("macro_target");
        assert_eq!(ctx.compat_target(), Some("macro_target".to_string()));
        assert_eq!(ctx.mod_path().as_str(), module_path!());
    }

    #[test]
    fn test_doing_records_action_with_compat_target_projection() {
        let mut ctx = OperationContext::doing("load_config");
        ctx.with_at("config.toml");

        assert_eq!(ctx.action().as_deref(), Some("load_config"));
        assert_eq!(ctx.locator().as_deref(), Some("config.toml"));
        assert_eq!(ctx.compat_target().as_deref(), Some("load_config"));
        assert_eq!(
            ctx.path(),
            &["load_config".to_string(), "config.toml".to_string()]
        );

        let rendered = ctx.to_string();
        assert!(rendered.contains("doing: load_config"));
        assert!(rendered.contains("at: config.toml"));
    }

    #[test]
    fn test_record_path_value() {
        let mut ctx = OperationContext::new();
        let path = PathBuf::from("/test/path");
        ctx.record("file_path", path.display());

        assert_eq!(ctx.context().items.len(), 1);
        assert!(ctx.context().items[0].1.contains("/test/path"));
    }

    #[test]
    fn test_withcontext_with_doing_sets_action_and_path() {
        let mut ctx = OperationContext::new();
        ctx.with_doing("start engine");

        assert_eq!(ctx.action.as_deref(), Some("start engine"));
        assert_eq!(ctx.path_string().as_deref(), Some("start engine"));
    }

    #[test]
    fn test_withcontext_from_pathbuf() {
        let path = PathBuf::from("/test/path");
        let ctx = OperationContext::from(&path);
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert!(ctx.context().items[0].1.contains("/test/path"));
    }

    #[test]
    fn test_withcontext_from_path() {
        let path = "/test/path";
        let ctx = OperationContext::from(path);
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert!(ctx.context().items[0].1.contains("/test/path"));
    }

    #[test]
    fn test_withcontext_from_path_pair() {
        let path = PathBuf::from("/test/path");
        let ctx = OperationContext::from(("file", path.to_string_lossy().as_ref()));
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert!(ctx.context().items[0].0.contains("file"));
        assert!(ctx.context().items[0].1.contains("/test/path"));
    }

    #[test]
    fn test_withcontext_display_with_target() {
        let mut ctx = OperationContext::doing("test_target");
        ctx.record("key1", "value1");
        let display = format!("{ctx}");
        assert!(display.contains("doing: test_target"));
        assert!(display.contains("key1=value1"));
    }

    #[test]
    fn test_withcontext_display_without_target() {
        let mut ctx = OperationContext::new();
        ctx.record("key1", "value1");
        let display = format!("{ctx}");
        assert!(display.contains("key1=value1"));
    }

    #[test]
    fn test_withcontext_from_str_path_pair() {
        let path = PathBuf::from("/test/path");
        let ctx = OperationContext::from(("file", &path));
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(ctx.context().items[0].0, "file");
        assert!(ctx.context().items[0].1.contains("/test/path"));
    }

    #[test]
    fn test_withcontext_from_str_pathbuf_pair() {
        let path = PathBuf::from("/test/pathbuf");
        let ctx = OperationContext::from(("file", path));
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(ctx.context().items[0].0, "file");
        assert!(ctx.context().items[0].1.contains("/test/pathbuf"));
    }

    // ContextAdd trait tests are commented out due to trait implementation issues
    // These tests will be revisited when the ContextAdd trait is properly implemented

    #[test]
    fn test_format_context_with_target() {
        let mut ctx = OperationContext::doing("test_target");
        ctx.record("key1", "value1");

        let formatted = ctx.format_context();
        assert_eq!(
            formatted,
            "doing=test_target: \ncall context:\n\tkey1 : value1\n"
        );
    }

    #[test]
    fn test_format_context_without_target() {
        let mut ctx = OperationContext::new();
        ctx.record("key1", "value1");

        let formatted = ctx.format_context();
        assert_eq!(formatted, "call context:\n\tkey1 : value1\n");
    }

    #[test]
    fn test_format_context_empty() {
        let ctx = OperationContext::new();
        let formatted = ctx.format_context();
        assert_eq!(formatted, "");
    }

    #[test]
    fn test_format_context_with_target_only() {
        let ctx = OperationContext::doing("test_target");
        let formatted = ctx.format_context();
        assert_eq!(formatted, "doing=test_target");
    }

    #[test]
    fn test_format_context_with_path() {
        let mut ctx = OperationContext::doing("place_order");
        ctx.with_doing("read_order_payload");
        ctx.record("order_id", "42");

        let formatted = ctx.format_context();
        assert_eq!(
            formatted,
            "doing=place_order path=place_order / read_order_payload: \ncall context:\n\torder_id : 42\n"
        );
    }

    #[test]
    fn test_path_string_and_display_use_normalized_action_locator_order() {
        let mut ctx = OperationContext::at("engine.toml");
        ctx.with_doing("start engine");

        assert_eq!(
            ctx.path_string().as_deref(),
            Some("start engine / engine.toml")
        );
        assert_eq!(
            ctx.path(),
            &["start engine".to_string(), "engine.toml".to_string()]
        );

        let rendered = format!("{ctx}");
        assert!(rendered.contains("doing: start engine"));
        assert!(rendered.contains("at: engine.toml"));
        assert!(rendered.contains("path: start engine / engine.toml"));
        assert!(!rendered.contains("path: engine.toml / start engine"));
    }

    #[test]
    fn test_normalized_path_prefers_action_head_over_compat_target() {
        let mut ctx = OperationContext::from_target("legacy_target".to_string());
        ctx.with_doing("start engine");
        ctx.with_at("engine.toml");

        assert_eq!(ctx.compat_target().as_deref(), Some("legacy_target"));
        assert_eq!(
            ctx.path_string().as_deref(),
            Some("start engine / engine.toml")
        );
    }

    #[test]
    fn test_set_target_after_doing_only_appends_compat_path_segment() {
        let mut ctx = OperationContext::doing("start engine");
        ctx.set_target("legacy_target");

        assert_eq!(ctx.compat_target().as_deref(), Some("start engine"));
        assert_eq!(
            ctx.path_string().as_deref(),
            Some("start engine / legacy_target")
        );
    }

    #[test]
    fn test_set_target_without_action_only_updates_compat_target_projection() {
        let mut ctx = OperationContext::new();
        ctx.set_target("legacy_target");

        assert_eq!(ctx.compat_target().as_deref(), Some("legacy_target"));
        assert!(ctx.path().is_empty());
        assert_eq!(ctx.path_string().as_deref(), Some("legacy_target"));
    }

    #[test]
    fn test_matching_action_clears_redundant_stored_compat_target() {
        let mut ctx = OperationContext::from_target("start engine".to_string());
        ctx.with_doing("start engine");

        assert!(ctx.target.is_none());
        assert_eq!(ctx.compat_target().as_deref(), Some("start engine"));
        assert_eq!(ctx.path(), &["start engine".to_string()]);
    }

    #[test]
    fn test_context_take_with_path_context() {
        let mut ctx = OperationContext::new();

        // 测试PathContext包装类型的ContextTake实现
        let path1 = PathBuf::from("/test/path1.txt");
        let path2 = Path::new("/test/path2.txt");

        ctx.record("file1", path1.display());
        ctx.record("file2", path2.display());

        assert_eq!(ctx.context().items.len(), 2);
        assert_eq!(ctx.context().items[0].0, "file1");
        assert!(ctx.context().items[0].1.contains("/test/path1.txt"));
        assert_eq!(ctx.context().items[1].0, "file2");
        assert!(ctx.context().items[1].1.contains("/test/path2.txt"));
    }
