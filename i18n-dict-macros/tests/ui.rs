//! UI 测试:错误路径(非法输入 → compile_error!)通过 trybuild 真实编译验证。
//!
//! 说明:proc-macro API 只能在宏展开上下文使用,无法在测试进程内直接调用
//! 宏函数;trybuild 用独立用例文件触发真实编译,校验编译失败的报错。
//! 更新预期报错:环境变量 `TRYBUILD=overwrite` 后运行本测试。

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
