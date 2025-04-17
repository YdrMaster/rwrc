#![doc = include_str!("../README.md")]
#![deny(warnings, missing_docs)]

//! RwRc - 带有读写状态的引用计数对象，可以在共享所有权的同时实现持续的访问控制
//!
//! 这个库提供了一个结合了 `Rc<T>` 的引用计数，以及 `RefCell` 的动态借用检查功能的智能指针。
//! 与标准库中这些组件的主要区别在于：
//!
//! - **对象本身持有读写状态**：RwRc 可以持有读写状态，实现对可变性的持续锁定，而不仅限于借用期间
//! - **读写状态与引用计数耦合**：读状态下克隆会保持读状态，使共享读取场景更加自然
//!
//! # 功能特性
//!
//! - 提供弱引用支持，类似于 `Weak<T>`
//! - 提供`LocalRef`和`LocalMut`简化访问模式，类似于 `Ref<T>`和`RefMut<T>`
//!
//! # 使用示例
//!
//! ```
//! use rwrc::RwRc;
//!
//! let mut data = RwRc::new(42);
//!
//! // 读取数据
//! if data.try_read_global() {
//!     assert!(data.is_readable());
//!     let reader = data.read();
//!     assert_eq!(*reader, 42); // 读取数据
//! }
//! data.release(); // 完成后释放读取锁
//!
//! // 修改数据
//! if data.try_write_global() {
//!     assert!(data.is_writeable());
//!     let mut writer = data.write();
//!     *writer = 100;
//! }
//!     data.release(); // 完成后释放写入锁
//! ```
mod flag;
mod local;
mod weak;

use flag::RwFlag;
use std::{cell::Cell, rc::Rc};

pub use local::{LocalMut, LocalRef};
pub use weak::RwWeak;

/// 带有预期读写状态的引用计数。
pub struct RwRc<T> {
    /// 共享的对象和状态。
    rc: Rc<Internal<T>>,
    /// 此副本占用的读写状态。
    state: RwState,
}

/// 共享的对象和状态。
struct Internal<T> {
    /// 共享对象。
    val: Cell<T>,
    /// 共享读写状态。
    flag: RwFlag,
}

/// 副本读写状态。
///
/// 表示 `RwRc` 实例当前的读写状态。
#[derive(Clone, Copy, Debug)]
enum RwState {
    /// 持有（不关心读写）。
    Hold,
    /// 预期读，禁止修改。
    Read,
    /// 预期写，限制读写。
    Write,
}

impl<T> From<T> for RwRc<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> Clone for RwRc<T> {
    /// 克隆 `RwRc<T>` 实例。
    /// 只有当源对象在读状态时，克隆的对象才会设置读状态，否则设置为持有状态。
    fn clone(&self) -> Self {
        // 复制读写锁时，先原样复制一个
        let mut ans = Self {
            rc: self.rc.clone(),
            state: RwState::Hold,
        };
        // 如果当前对象在读状态，复制的对象也设置读状态
        if matches!(self.state, RwState::Read) {
            ans.state = RwState::Read;
            assert!(ans.rc.flag.hold_to_read())
        }
        ans
    }
}

impl<T> Drop for RwRc<T> {
    fn drop(&mut self) {
        // 释放对象时也释放对象占用的锁
        self.release()
    }
}

impl<T> RwRc<T> {
    /// 从对象初始化读写锁时，直接设置到读状态。
    pub fn new(val: T) -> Self {
        Self {
            rc: Rc::new(Internal {
                val: Cell::new(val),
                flag: RwFlag::new_read(),
            }),
            state: RwState::Read,
        }
    }

    /// 判断是否可读。
    /// 会结合共享读写状态进行判断。
    pub fn is_readable(&self) -> bool {
        match self.state {
            RwState::Hold => self.rc.flag.is_readable(),
            RwState::Read | RwState::Write => true,
        }
    }

    /// 判断是否可写。
    /// 会结合全局状态进行判断。
    pub fn is_writeable(&self) -> bool {
        match self.state {
            RwState::Hold => self.rc.flag.is_writeable(),
            RwState::Read => self.rc.flag.is_this_writeable(),
            RwState::Write => true,
        }
    }

    /// 尝试设置到读状态。
    ///
    /// 尝试将当前实例设置为读状态，使其可以安全地读取数据。
    /// 如果当前全局状态允许新的读取操作，则会将实例设置为读状态，返回 `true`
    /// 否则当有其他对象持有写状态导致无法获取读状态时，返回 `false`。
    pub fn try_read_global(&mut self) -> bool {
        match self.state {
            RwState::Hold => {
                if !self.rc.flag.hold_to_read() {
                    return false;
                }
                self.state = RwState::Read
            }
            RwState::Read | RwState::Write => {}
        }
        true
    }

    /// 尝试设置到写状态。
    ///
    /// 尝试将当前实例设置为写状态，使其可以安全地修改数据。
    /// 如果没有其他对象持有读状态或写状态时，则会将实例设置为写状态，返回 `true`，
    /// 否则当有其他对象持有读状态或写状态时，返回 `false`。
    pub fn try_write_global(&mut self) -> bool {
        match self.state {
            RwState::Hold if !self.rc.flag.hold_to_write() => false,
            RwState::Read if !self.rc.flag.read_to_write() => false,
            _ => {
                self.state = RwState::Write;
                true
            }
        }
    }

    /// 释放读写状态。
    ///
    /// 将当前实例从读状态或写状态释放回持有状态，允许其他实例获取读或写权限。
    /// 当不再需要访问数据时，应该调用此方法释放状态。
    /// `Drop` 会自动调用此方法。
    pub fn release(&mut self) {
        match std::mem::replace(&mut self.state, RwState::Hold) {
            RwState::Hold => {}
            RwState::Read => self.rc.flag.read_to_hold(),
            RwState::Write => self.rc.flag.write_to_hold(),
        }
    }
}

#[test]
fn test_new() {
    let rc = RwRc::new(42);
    assert!(matches!(rc.state, RwState::Read));
    assert!(rc.is_readable());
    assert!(rc.is_writeable());
}

#[test]
fn test_clone() {
    let rc1 = RwRc::new(42);
    let rc2 = rc1.clone();

    // 克隆时原对象在读状态，克隆对象也应处于读状态
    assert!(matches!(rc1.state, RwState::Read));
    assert!(matches!(rc2.state, RwState::Read));

    // 创建一个新对象并释放读状态
    let mut rc3 = RwRc::new(100);
    rc3.release();
    assert!(matches!(rc3.state, RwState::Hold));

    // 克隆时原对象在持有状态，克隆对象也应处于持有状态
    let rc4 = rc3.clone();
    assert!(matches!(rc4.state, RwState::Hold));
}

#[test]
fn test_try_read_global() {
    let mut rc1 = RwRc::new(42);
    rc1.release(); // 先释放到持有状态

    // 尝试获取读状态
    assert!(rc1.try_read_global());
    assert!(matches!(rc1.state, RwState::Read));
    assert!(rc1.is_readable());

    // 已在读状态时再次获取读状态
    assert!(rc1.try_read_global());

    // 创建一个新的引用并获取写状态
    let mut rc2 = rc1.clone();
    rc2.release(); // 释放到持有状态

    // rc1在读状态，rc2应该无法获取写状态
    assert!(!rc2.try_write_global());

    // rc1释放读状态
    rc1.release();

    // 现在rc2应该可以获取写状态
    assert!(rc2.try_write_global());
    assert!(matches!(rc2.state, RwState::Write));

    // 当rc2持有写状态时，rc1应该无法获取读状态
    assert!(!rc1.try_read_global());
}

#[test]
fn test_try_write_global() {
    let mut rc1 = RwRc::new(42);
    rc1.release(); // 先释放到持有状态

    // 尝试获取写状态
    assert!(rc1.try_write_global());
    assert!(matches!(rc1.state, RwState::Write));
    assert!(rc1.is_readable());
    assert!(rc1.is_writeable());

    // 创建一个新的引用
    let mut rc2 = rc1.clone();

    // rc1在写状态，rc2应该无法获取读状态或写状态
    assert!(!rc2.try_read_global());
    assert!(!rc2.try_write_global());

    // rc1释放写状态
    rc1.release();

    // 现在rc2应该可以获取读状态
    assert!(rc2.try_read_global());
    assert!(matches!(rc2.state, RwState::Read));

    // 再创建一个新的引用
    let mut rc3 = rc1.clone();

    // rc2在读状态，rc3应该可以获取读状态但不能获取写状态
    assert!(rc3.try_read_global());
    rc3.release();
    assert!(!rc3.try_write_global());
}

#[test]
fn test_drop() {
    let mut rc1 = RwRc::new(42);

    // 创建一个作用域，在作用域中创建一个新的引用并获取写状态
    {
        let mut rc2 = rc1.clone();
        rc1.release(); // 释放rc1的读状态

        // rc2获取写状态
        assert!(rc2.try_write_global());
        assert!(matches!(rc2.state, RwState::Write));

        // 此时rc1应该无法获取读状态
        assert!(!rc1.try_read_global());

        // rc2会在作用域结束时自动调用drop，释放写状态
    }

    // 作用域结束后，rc2应该已释放写状态，rc1应该可以获取读状态
    assert!(rc1.try_read_global());
    assert!(matches!(rc1.state, RwState::Read));
}

#[test]
fn test_multiple_readers() {
    let mut rc1 = RwRc::new(42);
    let mut rc2 = rc1.clone();
    let mut rc3 = rc1.clone();

    // 所有对象都释放到持有状态
    rc1.release();
    rc2.release();
    rc3.release();

    // rc1获取读状态
    assert!(rc1.try_read_global());

    // rc2和rc3也应该可以获取读状态
    assert!(rc2.try_read_global());
    assert!(rc3.try_read_global());

    // 但是所有对象都无法获取写状态
    rc1.release();
    assert!(!rc1.try_write_global());

    // 当所有读者都释放读状态后，应该可以获取写状态
    rc2.release();
    rc3.release();
    assert!(rc1.try_write_global());
}

#[test]
fn test_from() {
    // 测试从基本类型转换
    let rc: RwRc<i32> = 42.into();
    assert!(matches!(rc.state, RwState::Read));
    assert!(rc.is_readable());

    // 测试从字符串转换
    let rc: RwRc<String> = String::from("test").into();
    assert!(matches!(rc.state, RwState::Read));
    assert!(rc.is_readable());

    // 测试显式使用 From trait
    let rc = RwRc::from(100);
    assert!(matches!(rc.state, RwState::Read));
    assert!(rc.is_readable());
}

#[test]
fn test_hold() {
    let mut rc = RwRc::new(42);
    assert!(rc.is_readable()); // 新建对象默认在读状态，应该可读

    // 测试持有状态下的可读性
    rc.release();
    assert!(rc.rc.flag.is_readable()); // 确保全局状态可读
    assert!(rc.is_readable()); // Hold状态且全局可读时应该可读
    assert!(rc.is_writeable()); // Hold状态且全局可写时应该可写
    assert!(rc.try_read_global()); // 单个实例hold状态设置读状态，应该是可读的
}
