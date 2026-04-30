    #[test]
    fn test_context_metadata_records_values() {
        let ctx = OperationContext::doing("load")
            .with_meta("config.kind", "wpsrc")
            .with_meta("parse.line", 1u32)
            .with_meta("parse.strict", true);

        assert_eq!(ctx.metadata().get_str("config.kind"), Some("wpsrc"));
        assert!(ctx.metadata().as_map().contains_key("parse.line"));
        assert!(ctx.metadata().as_map().contains_key("parse.strict"));
    }

    #[test]
    fn test_context_metadata_duplicate_key_overwrites() {
        let ctx = OperationContext::new()
            .with_meta("config.kind", "sink_route")
            .with_meta("config.kind", "sink_defaults");

        assert_eq!(ctx.metadata().get_str("config.kind"), Some("sink_defaults"));
    }

    #[test]
    fn test_context_metadata_ignores_empty_key() {
        let result = std::panic::catch_unwind(|| OperationContext::new().with_meta("", "ignored"));

        if cfg!(debug_assertions) {
            assert!(result.is_err());
        } else {
            let ctx = result.expect("release build should ignore empty metadata key");
            assert!(ctx.metadata().is_empty());
        }
    }
