use async_io::Async;
use std::io;
use std::os::fd::{AsFd, AsRawFd};

pub struct AsyncRoute<T> {
    fd: Async<T>,
}
impl<T: AsRawFd + AsFd> AsyncRoute<T> {
    pub fn new(fd: T) -> io::Result<Self> {
        Ok(Self {
            fd: Async::new(fd)?,
        })
    }
    pub async fn read_with<R>(&mut self, op: impl FnMut(&mut T) -> io::Result<R>) -> io::Result<R> {
        unsafe { self.fd.read_with_mut(op).await }
    }
    pub async fn write_with<R>(
        &mut self,
        op: impl FnMut(&mut T) -> io::Result<R>,
    ) -> io::Result<R> {
        unsafe { self.fd.write_with_mut(op).await }
    }
}
