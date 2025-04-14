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
    pub fn is_readable(&self) -> bool {
        match self.state {
            RwState::Hold => self.rc.flag.is_readable(),
            RwState::Read | RwState::Write => true,
        }
    }

    /// 判断是否可写。
    pub fn is_writeable(&self) -> bool {
        match self.state {
            RwState::Hold => self.rc.flag.is_writeable(),
            RwState::Read => self.rc.flag.is_this_writeable(),
            RwState::Write => true,
        }
    }

    /// 尝试设置到读状态。
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
    pub fn release(&mut self) {
        match std::mem::replace(&mut self.state, RwState::Hold) {
            RwState::Hold => {}
            RwState::Read => self.rc.flag.read_to_hold(),
            RwState::Write => self.rc.flag.write_to_hold(),
        }
    }
}
