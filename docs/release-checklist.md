# Release Checklist

这份清单记录当前 `0.7.x` 发布时需要执行的步骤。

`CHANGELOG.md` 只记录版本结果；具体发布动作统一记在这里。

## 发布前

1. 确认 `CHANGELOG.md`、`README.md`、`docs/` 已对齐当前代码
2. 确认根 crate 和 `orion-error-derive` 的版本号一致
3. 运行：
   - `cargo fmt --all`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-features -- --test-threads=1`
4. 在可联网环境中运行：
   - `cargo package --manifest-path orion-error-derive/Cargo.toml`
   - `cargo package`
   - `cargo publish --manifest-path orion-error-derive/Cargo.toml --dry-run`
   - `cargo publish --dry-run`

## 正式发布顺序

1. 先发布 `orion-error-derive`
2. 等 crates.io 索引传播完成
3. 再发布 `orion-error`

当前仓库的 GitHub Actions release workflow 已按这个顺序配置。

## 发布后检查

1. 确认 crates.io 上两个包版本都可见
2. 确认 `orion-error` 的默认 `derive` feature 能正常解析 `orion-error-derive`
3. 确认 docs.rs 页面生成成功：
   - `orion-error`
   - `orion-error-derive`
