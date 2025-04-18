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

这个库提供 `RwRc<T>` 结构体，这是一个结合了 `Rc<T>` 的引用计数功能，以及 `RefCell<T>` 的动态借用检查功能的智能指针，类似于 Rust 中非并发共享所有权与内部可变性的经典组合 `Rc<RefCell<T>>`。`RwRc<T>` 与 `Rc<RefCell<T>>` 主要区别在于：

- 内部维持读写状态：`RwRc<T>` 在内部维持读写锁状态，实现对可变性的持续控制，而不仅限于借用期间。特别适用于需要长期维持读锁定的情况，不需要复杂的生命周期控制，同时，由于维持和变换“本地状态”，大部分对 `RwRc<T>` 的操作需要对 `RwRc<T>` 结构体的可变引用；
- 读写状态与引用计数耦合：克隆读锁定状态下的 `RwRc<T>` 会产生一个新的读锁定的 `RwRc<T>`，使共享读取场景更加自然。克隆无锁定和写锁定的 `RwRc<T>` 会产生无锁定的 `RwRc<T>`；

`RwRc<T>` 还提供一系列类似 `Rc<RefCell<T>>` 的功能，降低开发者迁移和使用的难度：

- 提供弱引用类型 `RwWeak<T>`，类似于 `Weak<RefCell<T>>`，支持不维持所有权，以用于循环引用等场景；
- 提供 `LocalRef<'a, T>` 和 `LocalMut<'a, T>` 简化在局部作用域中临时访问的写法，类似于 `Ref<'a, T>` 和 `RefMut<'a, T>`；

## 使用示例

```rust
use rwrc::RwRc;

// 新创建出来的 RwRc<T> 自动带有全局读锁定
// 要操作 RwRc<T>，需要整个对象可变
let mut data = RwRc::new(42);

// 独占时，可直接读取
assert_eq!(*data.read(), 42);

// 独占时，可直接写入
*data.write() = 99;
assert_eq!(*data.read(), 99);

// 复制 RwRc<T>，产生共享所有权的对象
// 由于原对象全局读锁定，新的对象保持全局写锁定
let mut cloned = data.clone();

// 两个对象同时可读
assert_eq!(*data.read(), 99);
assert_eq!(*cloned.read(), 99);

// 两个对象同时不可写
assert!(data.try_write().is_none());
assert!(cloned.try_write().is_none());

// 释放掉其中一个的全局读锁，另一个再次可写
cloned.release();
assert!(data.try_write().is_some());
```

## 应用场景

这个对象的设计目标是用于构造计算图，并动态追踪计算图中边的可变性，以发现原位计算的可能性。

例如以下具有 SSA 形式的伪代码：

> SSA（Static Single Assignment，静态单赋值）形式的 IR 与 DAG（Directed Acyclic Graph，有向无环图）形式的计算图等价。

```plaintext
b = f1(a)
c = f2(b)
d = f3(b)
e = f4(c, d)
```

其中 `f1`、`f2`、`f3`、`f4` 是计算图中的节点（算子），`a`、`b`、`c`、`d`、`e` 是计算图中的边（变量）。

假设所有算子均是可以原位计算的算法，则通过生命周期分析，计算图可以转化为如下的非 SSA 形式：

```plaintext
a = f1(a)
b = f2(a)
a = f3(a)
b = f4(b, a)
```

在这种情况下，仅需要 2 个变量即可完成计算。然而，如果在没有完整计算图的情况下，在全图进行生命周期分析是不可能的。这就需要每个变量自己维持自己的读写锁状态（通常情况下都是全局读锁定）：

| 序号 | 运算           | 原位优化       | 优化依据
|:----:|:---------------|:---------------|:-
| 1    | `b = f1(a)`    | `a = f1(a)`    | a 独占读锁定，可原位计算
| 2    | `c = f2(b)`    | `b = f2(a)`    | b 非独占读锁定，不可原位计算，完成后释放
| 3    | `d = f3(b)`    | `a = f3(a)`    | b 独占读锁定，可原位计算
| 4    | `e = f4(c, d)` | `b = f4(b, a)` | c、d 独占读锁定，可原位计算

对于张量程序，`RwRc<T>` 被包裹在张量中，其行为和分析将更加复杂。张量类型 `Tensor<T>` 由张量元信息和张量数据构成：

```rust
struct Tensor<T> {
    dt: DataType,
    layout: TensorLayout,
    data: T,
}
```

张量元信息包括张量的数据类型和存储布局，张量数据是张量中存储的实际内容。张量的某些变换可以仅发生在元信息层次，而不实际修改张量数据，这种设计将降低张量变换的开销，提升张量程序的性能。此时，变换了元信息的张量就是一个与原张量共享数据，但不共享元信息的新张量。在张量程序中操作这样的张量时，使用 `Tensor<RwRc<T>>` 组合类型将消除张量和张量数据的双重共享带来的原位计算判别复杂性。
