use crate::{RwRc, RwState};
use std::ops::{Deref, DerefMut};

/// 对 `RwRc<T>` 的只读借用。
///
/// 该类型表示对 `RwRc<T>` 的只读借用，允许安全地访问内部数据。
/// 当 `LocalRef` 被丢弃时，会自动还原 `RwRc` 的读写状态。
///
/// # 示例
///
/// ```rust
/// use rwrc::RwRc;
///
/// let rwrc = RwRc::new(42);
/// {
///     let reader = rwrc.read();
///     assert_eq!(*reader, 42); // 可以读取内部值
/// } // reader被丢弃，如果RwRc处于Hold状态，读锁会被释放
/// ```
pub struct LocalRef<'w, T>(&'w RwRc<T>);

/// 对 `RwRc<T>` 的可变借用。
///
/// 该类型表示对 `RwRc<T>` 的可变借用，允许安全地修改内部数据。
/// 当 `LocalMut` 被丢弃时，会自动还原 `RwRc` 的读写状态。
///
/// # 示例
///
/// ```rust
/// use rwrc::RwRc;
///
/// let mut rwrc = RwRc::new(42);
/// {
///     let mut writer = rwrc.write();
///     *writer = 100; // 可以修改内部值
/// } // writer被丢弃，会还原RwRc的读写状态
/// ```
pub struct LocalMut<'w, T>(&'w mut RwRc<T>);

impl<T> RwRc<T> {
    /// 尝试获取只读引用`LocalRef<T>`，如果 RwRc 没有读取权限，则会尝试获取读取权限，如果获取失败，则返回 None。
    /// Drop 后不会改变 RwRc 的读写状态。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rwrc::RwRc;
    ///
    /// let rwrc = RwRc::new(42);
    /// let reader = rwrc.try_read().unwrap();
    /// let reader2 = rwrc.try_read().unwrap();
    /// assert_eq!(*reader, 42);
    /// assert_eq!(*reader2, 42);
    /// ```
    pub fn try_read(&self) -> Option<LocalRef<T>> {
        match self.state {
            RwState::Hold => {
                if self.rc.flag.hold_to_read() {
                    Some(LocalRef(self))
                } else {
                    None
                }
            }
            RwState::Read | RwState::Write => Some(LocalRef(self)),
        }
    }

    /// 尝试获取可变引用`LocalMut<T>`，如果 RwRc 没有写入权限，则会尝试获取写入权限，如果获取失败，则返回 None。
    /// Drop 后不会改变 RwRc 的读写状态。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rwrc::RwRc;
    ///
    /// let mut rwrc = RwRc::new(42);
    /// let mut writer = rwrc.try_write().unwrap();
    /// assert_eq!(*writer, 42);
    /// *writer = 43;
    /// drop(writer);
    /// assert_eq!(*rwrc.read(), 43);
    /// ```
    pub fn try_write(&mut self) -> Option<LocalMut<T>> {
        match self.state {
            RwState::Hold => {
                if self.rc.flag.hold_to_write() {
                    Some(LocalMut(self))
                } else {
                    None
                }
            }
            RwState::Read => {
                if self.rc.flag.read_to_write() {
                    Some(LocalMut(self))
                } else {
                    None
                }
            }
            RwState::Write => Some(LocalMut(self)),
        }
    }

    /// 读取，如果 RwRc 没有读取权限，则会尝试获取，如果获取失败，则会 panic。
    /// Drop 后不会改变 RwRc 的读写状态。
    ///
    /// # Panic
    ///
    /// 当无法获取读取权限时会 panic。
    pub fn read(&self) -> LocalRef<T> {
        self.try_read().unwrap()
    }

    /// 写入，如果 RwRc 没有写入权限，则会尝试获取，如果获取失败，则会 panic。
    /// Drop 后不会改变 RwRc 的读写状态。
    ///
    /// # Panic
    ///
    /// 当无法获取写入权限时会 panic。
    pub fn write(&mut self) -> LocalMut<T> {
        self.try_write().unwrap()
    }
}

impl<T> Drop for LocalRef<'_, T> {
    /// 释放 `LocalRef` 时，并还原 `RwRc` 的读写状态。
    fn drop(&mut self) {
        match self.0.state {
            RwState::Hold => self.0.rc.flag.read_to_hold(),
            RwState::Read | RwState::Write => {}
        }
    }
}

impl<T> Drop for LocalMut<'_, T> {
    /// 释放 `LocalMut` 时，并还原 `RwRc` 的读写状态。
    fn drop(&mut self) {
        match self.0.state {
            RwState::Hold => self.0.rc.flag.write_to_hold(),
            RwState::Read => self.0.rc.flag.write_to_read(),
            RwState::Write => {}
        }
    }
}

impl<T> Deref for LocalRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.rc.val.as_ptr() }
    }
}

impl<T> Deref for LocalMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.rc.val.as_ptr() }
    }
}

impl<T> DerefMut for LocalMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.rc.val.as_ptr() }
    }
}

#[test]
fn test_recover_state() {
    let mut rwrc_hold = RwRc::new(42);
    let mut rwrc_read = RwRc::new(42);
    let mut rwrc_write = RwRc::new(42);
    rwrc_hold.release();
    assert!(rwrc_write.try_write_global());

    {
        let _ = rwrc_hold.read();
        let _ = rwrc_read.read();
        let _ = rwrc_write.read();
    }
    assert!(matches!(rwrc_hold.state, RwState::Hold));
    assert!(matches!(rwrc_read.state, RwState::Read));
    assert!(matches!(rwrc_write.state, RwState::Write));

    {
        let _ = rwrc_hold.write();
        let _ = rwrc_read.write();
        let _ = rwrc_write.write();
    }
    assert!(matches!(rwrc_hold.state, RwState::Hold));
    assert!(matches!(rwrc_read.state, RwState::Read));
    assert!(matches!(rwrc_write.state, RwState::Write));
}

#[test]
fn test_read_write() {
    let mut rwrc = RwRc::new(42);

    // 测试读取
    {
        let reader = rwrc.read();
        assert_eq!(*reader, 42);
    }

    // 测试写入
    {
        let mut writer = rwrc.write();
        *writer = 100;
    }

    // 验证写入后的值
    {
        let reader = rwrc.read();
        assert_eq!(*reader, 100);
    }
    rwrc.release();
    // 测试hold状态,之后被其他对象获取全局写状态，进行读取，应该失败
    {
        let mut rwrc2 = rwrc.clone();
        assert!(rwrc2.try_write_global());
        assert!(rwrc.try_read().is_none()); // 修改这行，直接使用 assert!
    }
    //  测试hold状态,之后被其他对象获取全局写状态，进行写入，应该失败
    {
        let mut rwrc2 = rwrc.clone();
        assert!(rwrc2.try_write_global());
        assert!(rwrc.try_write().is_none());
    }
    //  测试数据有多个可读引用，有的可读引用想要转换成可写,应该失败
    {
        let mut rwrc2 = rwrc.clone();
        assert!(rwrc.try_read_global());
        assert!(rwrc2.try_read_global());
        assert!(rwrc.try_write().is_none());
    }
}

#[test]
fn test_multiple_readers() {
    let rwrc = RwRc::new(42);
    let rwrc2 = rwrc.clone();

    // 多个读取者同时读取
    let reader1 = rwrc.read();
    let reader2 = rwrc2.read();

    assert_eq!(*reader1, 42);
    assert_eq!(*reader2, 42);
}

#[test]
fn test_deref() {
    let mut rwrc = RwRc::new(42);

    // 测试 LocalMut 的不可变解引用
    let writer = rwrc.write();
    assert_eq!(*writer, 42); // 通过 Deref trait 进行不可变访问

    // 测试多次解引用
    let value_ref: &i32 = &writer;
    assert_eq!(*value_ref, 42);

    // 测试在不同状态下的解引用
    drop(writer);
    rwrc.release();
    let writer = rwrc.write();
    assert_eq!(*writer, 42); // Hold 状态获取写权限后解引用

    // 测试复杂类型的解引用
    let mut string_rc = RwRc::new(String::from("test"));
    let string_writer = string_rc.write();
    assert_eq!(string_writer.len(), 4); // 可以访问字符串的方法
    assert_eq!(&*string_writer, "test"); // 可以解引用比较字符串内容
}
