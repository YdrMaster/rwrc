use crate::{RwRc, RwState};
use std::ops::{Deref, DerefMut};

/// 类似 [`RwLock`](std::sync::RwLock) 的自动读状态对象。
pub struct LocalRef<'w, T>(&'w RwRc<T>);

/// 类似 [`RwLock`](std::sync::RwLock) 的自动写状态对象。
pub struct LocalMut<'w, T>(&'w mut RwRc<T>);

impl<T> RwRc<T> {
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

    pub fn read(&self) -> LocalRef<T> {
        self.try_read().unwrap()
    }

    pub fn write(&mut self) -> LocalMut<T> {
        self.try_write().unwrap()
    }
}

impl<T> Drop for LocalRef<'_, T> {
    fn drop(&mut self) {
        match self.0.state {
            RwState::Hold => self.0.rc.flag.read_to_hold(),
            RwState::Read | RwState::Write => {}
        }
    }
}

impl<T> Drop for LocalMut<'_, T> {
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
