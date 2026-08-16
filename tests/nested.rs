//! 验证 with_global 嵌套行为。

use nano_rs::definitions::*;
use nano_rs::global::global_init;

#[test]
fn nested_with_global_behavior() {
    global_init();
    // 嵌套只读
    let result = std::panic::catch_unwind(|| {
        with_global(|g| {
            let _ = g.COLS;
            with_global(|g2| { let _ = g2.COLS; });
        });
    });
    println!("nested read: panicked={}", result.is_err());

    // with_global 内 with_global_mut
    let result2 = std::panic::catch_unwind(|| {
        with_global(|g| {
            let _ = g.COLS;
            with_global_mut(|g2| { g2.COLS = 10; });
        });
    });
    println!("read-then-mut: panicked={}", result2.is_err());

    // with_global_mut 内 with_global
    let result3 = std::panic::catch_unwind(|| {
        with_global_mut(|g| {
            g.COLS = 10;
            with_global(|g2| { let _ = g2.COLS; });
        });
    });
    println!("mut-then-read: panicked={}", result3.is_err());
}
