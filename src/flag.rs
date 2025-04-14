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
