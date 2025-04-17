# rwrc

[![CI](https://github.com/YdrMaster/rwrc/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/YdrMaster/rwrc/actions)
[![Latest version](https://img.shields.io/crates/v/rwrc.svg)](https://crates.io/crates/rwrc)
[![Documentation](https://docs.rs/rwrc/badge.svg)](https://docs.rs/rwrc)
[![license](https://img.shields.io/github/license/YdrMaster/rwrc)](https://mit-license.org/)
[![codecov](https://codecov.io/github/YdrMaster/rwrc/branch/main/graph/badge.svg)](https://codecov.io/github/YdrMaster/rwrc)
![GitHub repo size](https://img.shields.io/github/repo-size/YdrMaster/rwrc)
![GitHub code size in bytes](https://img.shields.io/github/languages/code-size/YdrMaster/rwrc)

[![GitHub Issues](https://img.shields.io/github/issues/YdrMaster/rwrc)](https://github.com/YdrMaster/rwrc/issues)
[![GitHub Pull Requests](https://img.shields.io/github/issues-pr/YdrMaster/rwrc)](https://github.com/YdrMaster/rwrc/pulls)
![GitHub contributors](https://img.shields.io/github/contributors/YdrMaster/rwrc)
![GitHub commit activity](https://img.shields.io/github/commit-activity/m/YdrMaster/rwrc)

带有读写状态的引用计数对象，可以在共享所有权的同时实现持续的访问控制。

这个库提供了一个结合了 `Rc<T>` 的引用计数，以及 `RefCell` 的动态借用检查功能的智能指针。与标准库中这些组件的主要区别在于：

- **对象本身持有读写状态**：RwRc 可以持有读写状态，实现对可变性的持续锁定，而不仅限于借用期间；
- **读写状态与引用计数耦合**：读状态下克隆会保持读状态，使共享读取场景更加自然；

## 功能特性

- 提供弱引用支持，类似于 `Weak<T>`；
- 提供 `LocalRef` 和 `LocalMut` 简化访问模式，类似于 `Ref<T>` 和 `RefMut<T>`；

## 使用示例

```rust
use rwrc::RwRc;

let mut data = RwRc::new(42);

// 读取数据
if data.try_read_global() {
    assert!(data.is_readable());
    let reader = data.read();
    assert_eq!(*reader, 42) // 读取数据
}
data.release(); // 完成后释放读取锁

// 修改数据
if data.try_write_global() {
    assert!(data.is_writeable());
    let mut writer = data.write();
    *writer = 100
}

data.release(); // 完成后释放写入锁
```
