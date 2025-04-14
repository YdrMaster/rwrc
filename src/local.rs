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
