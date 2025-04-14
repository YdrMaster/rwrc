use crate::{Internal, RwRc, RwState};
use std::{
    cmp, fmt,
    hash::Hash,
    rc::{Rc, Weak},
};

/// 弱引用版本的 [`RwRc<T>`]，不会影响引用计数。
///
/// `RwWeak<T>` 与标准库中的 [`Weak<T>`] 类似，持有一个对原始 [`RwRc<T>`] 的弱引用。
/// 它不会阻止底层数据被丢弃，也不会影响读写状态的变化。
///
/// 当原始的 [`RwRc<T>`] 被丢弃后，通过 `RwWeak<T>` 将无法访问底层数据。
///
/// # 示例
///
/// ```rust
/// use rwrc::RwRc;
///
/// let rc = RwRc::new(42);
/// let weak = rc.weak();
///
/// // 可以从弱引用升级到强引用
/// let rc2 = weak.hold().unwrap();
/// assert_eq!(*rc2.read(), 42);
///
/// // 当所有强引用被丢弃后，弱引用将无法升级
/// drop(rc);
/// drop(rc2);
/// assert!(weak.hold().is_none());
/// ```
#[repr(transparent)]
pub struct RwWeak<T>(Weak<Internal<T>>);

impl<T> fmt::Debug for RwWeak<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RwWeak")
            .field(&format_args!("{:p}", self.0.as_ptr()))
            .finish()
    }
}

impl<T> RwRc<T> {
    /// 创建一个 [`RwRc<T>`] 的弱引用版本。
    ///
    /// 该方法类似于标准库中 [`Rc::downgrade`] 的功能，返回一个不会影响引用计数的弱引用，同时不持有读写状态。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rwrc::RwRc;
    ///
    /// let rc = RwRc::new(10);
    /// let weak = rc.weak();
    ///
    /// // 可以通过弱引用访问数据
    /// assert_eq!(*weak.hold().unwrap().read(), 10);
    /// ```
    pub fn weak(&self) -> RwWeak<T> {
        RwWeak(Rc::downgrade(&self.rc))
    }
}

impl<T> Clone for RwWeak<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for RwWeak<T> {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for RwWeak<T> {}

impl<T> Hash for RwWeak<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}

impl<T> PartialOrd for RwWeak<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for RwWeak<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        Ord::cmp(&self.0.as_ptr(), &other.0.as_ptr())
    }
}

impl<T> RwWeak<T> {
    /// 尝试将弱引用升级为强引用。
    ///
    /// 如果原始的 [`RwRc<T>`] 已经被释放，则返回 `None`。
    /// 否则返回一个 [`RwRc<T>`]，其状态为 [`RwState::Hold`]。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rwrc::RwRc;
    ///
    /// let rc = RwRc::new(5);
    /// let weak = rc.weak();
    ///
    /// // 当强引用存在时，可以成功升级
    /// let rc2 = weak.hold().unwrap();
    /// assert_eq!(*rc2.read(), 5);
    ///
    /// // 释放所有强引用
    /// drop(rc);
    /// drop(rc2);
    ///
    /// // 当所有强引用被释放后，无法再升级
    /// assert!(weak.hold().is_none());
    /// ```
    pub fn hold(&self) -> Option<RwRc<T>> {
        self.0.upgrade().map(|rc| RwRc {
            rc,
            state: RwState::Hold,
        })
    }
}

#[test]
fn test_weak_hold() {
    // 创建一个RwRc实例
    let mut rc = RwRc::new(42);

    // 创建弱引用
    let weak = rc.weak();

    // 从弱引用中恢复强引用
    let rc2 = weak.hold().unwrap();
    assert_eq!(*rc2.read(), 42);

    // 修改值 - 要先获取写权限
    *rc.write() = 100;

    // 确认通过弱引用获取的强引用也能看到更新
    assert_eq!(*rc2.read(), 100);

    // 释放所有强引用
    drop(rc);
    drop(rc2);

    // 此时不能从弱引用中恢复强引用
    assert!(weak.hold().is_none());
}

#[test]
fn test_weak_clone() {
    let mut rc = RwRc::new(10);
    let weak1 = rc.weak();
    let weak2 = weak1.clone();

    // 两个弱引用应该指向同一对象
    assert_eq!(weak1, weak2);

    // 两个弱引用都可以恢复成强引用
    let rc1 = weak1.hold().unwrap();
    let rc2 = weak2.hold().unwrap();

    assert_eq!(*rc1.read(), 10);
    assert_eq!(*rc2.read(), 10);

    // 修改值后，所有引用都能看到更新
    *rc.write() = 20;

    assert_eq!(*rc1.read(), 20);
    assert_eq!(*rc2.read(), 20);
}

#[test]
fn test_weak_hash_and_compare() {
    use std::collections::HashSet;

    let rc1 = RwRc::new(1);
    let rc2 = RwRc::new(2);

    let weak1 = rc1.weak();
    let weak1_clone = weak1.clone();
    let weak2 = rc2.weak();

    // 测试相等性
    assert_eq!(weak1, weak1_clone);
    assert_ne!(weak1, weak2);

    // 测试作为HashMap的键
    let mut set = HashSet::new();
    set.insert(weak1.clone());

    assert!(set.contains(&weak1_clone));
    assert!(!set.contains(&weak2));

    // 测试排序
    let mut vec = [weak2.clone(), weak1.clone()];
    vec.sort();

    // 由于内存地址不确定，无法确定顺序，但确保排序后不会崩溃
    assert_eq!(vec.len(), 2);
    assert!(vec.contains(&weak1));
    assert!(vec.contains(&weak2));
}

#[test]
fn test_weak_after_drop() {
    let weak;
    {
        let rc = RwRc::new(30);
        weak = rc.weak();

        // 在作用域内，可以成功恢复强引用
        assert!(weak.hold().is_some());
    }
    // 离开作用域后，rc被销毁

    // 此时不能从弱引用恢复强引用
    assert!(weak.hold().is_none());
}

#[test]
fn test_weak_multi_hold() {
    let mut rc = RwRc::new(42);
    let weak = rc.weak();

    // 多次恢复强引用
    let rc1 = weak.hold().unwrap();
    let rc2 = weak.hold().unwrap();
    let rc3 = weak.hold().unwrap();

    assert_eq!(*rc1.read(), 42);
    assert_eq!(*rc2.read(), 42);
    assert_eq!(*rc3.read(), 42);

    // 通过原始引用修改值
    *rc.write() = 100;

    // 所有引用都应该看到新值
    assert_eq!(*rc.read(), 100);
    assert_eq!(*rc1.read(), 100);
    assert_eq!(*rc2.read(), 100);
    assert_eq!(*rc3.read(), 100);

    // 丢弃原始引用
    drop(rc);

    // 只要还有一个强引用存在，弱引用就能继续工作
    assert!(weak.hold().is_some());

    // 丢弃所有强引用
    drop(rc1);
    drop(rc2);
    drop(rc3);

    // 此时弱引用无法恢复
    assert!(weak.hold().is_none());
}
