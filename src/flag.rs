use std::cell::Cell;

/// 共享读写状态。
#[repr(transparent)]
pub(super) struct RwFlag(Cell<usize>);

impl RwFlag {
    /// 初始化状态变量。
    pub fn new_read() -> Self {
        Self(Cell::new(1))
    }

    /// 判断是否可读。
    pub fn is_readable(&self) -> bool {
        self.0.get() != usize::MAX
    }

    /// 判断是否可写。
    pub fn is_this_writeable(&self) -> bool {
        matches!(self.0.get(), 0 | 1)
    }

    /// 判断是否可写。
    pub fn is_writeable(&self) -> bool {
        self.0.get() == 0
    }

    pub fn hold_to_read(&self) -> bool {
        match self.0.get() {
            usize::MAX => false,
            n => {
                self.0.set(n + 1);
                true
            }
        }
    }

    pub fn hold_to_write(&self) -> bool {
        match self.0.get() {
            0 => {
                self.0.set(usize::MAX);
                true
            }
            _ => false,
        }
    }

    pub fn read_to_write(&self) -> bool {
        match self.0.get() {
            1 => {
                self.0.set(usize::MAX);
                true
            }
            _ => false,
        }
    }

    pub fn read_to_hold(&self) {
        let current = self.0.get();
        debug_assert!((1..usize::MAX).contains(&current));
        self.0.set(current - 1)
    }

    pub fn write_to_hold(&self) {
        let current = self.0.get();
        debug_assert_eq!(current, usize::MAX);
        self.0.set(0)
    }

    pub fn write_to_read(&self) {
        let current = self.0.get();
        debug_assert_eq!(current, usize::MAX);
        self.0.set(1)
    }
}

#[test]
fn test_new_read() {
    let flag = RwFlag::new_read();
    assert!(flag.is_readable());
    assert!(!flag.is_writeable());
    assert!(flag.is_this_writeable());
}

#[test]
fn test_hold_to_read() {
    let flag = RwFlag::new_read();
    assert!(flag.hold_to_read());
    assert!(flag.is_readable());
    assert!(!flag.is_writeable());
    assert!(!flag.is_this_writeable());
}

#[test]
fn test_read_to_hold() {
    let flag = RwFlag::new_read();
    assert!(flag.hold_to_read());
    flag.read_to_hold();
    assert!(flag.is_readable());
    assert!(!flag.is_writeable());
    assert!(flag.is_this_writeable());
}

#[test]
fn test_hold_to_write() {
    let flag = RwFlag(Cell::new(0));
    assert!(flag.hold_to_write());
    assert!(!flag.is_readable());
    assert!(!flag.is_writeable());
    assert!(!flag.is_this_writeable());
}

#[test]
fn test_read_to_write() {
    let flag = RwFlag::new_read();
    assert!(flag.read_to_write());
    assert!(!flag.is_readable());
    assert!(!flag.is_writeable());
    assert!(!flag.is_this_writeable());
}

#[test]
fn test_write_to_hold() {
    let flag = RwFlag::new_read();
    assert!(flag.read_to_write());
    flag.write_to_hold();
    assert!(flag.is_readable());
    assert!(flag.is_writeable());
    assert!(flag.is_this_writeable());
}

#[test]
fn test_write_to_read() {
    let flag = RwFlag::new_read();
    assert!(flag.read_to_write());
    flag.write_to_read();
    assert!(flag.is_readable());
    assert!(!flag.is_writeable());
    assert!(flag.is_this_writeable());
}
