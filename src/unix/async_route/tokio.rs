use std::io;
use std::os::fd::AsRawFd;
use tokio::io::unix::AsyncFd;
use tokio::io::Interest;

pub struct AsyncRoute<T: AsRawFd> {
    fd: AsyncFd<T>,
}
impl<T: AsRawFd> AsyncRoute<T> {
    pub fn new(fd: T) -> io::Result<Self> {
        let mut nonblocking = true as libc::c_int;
        if unsafe { libc::ioctl(fd.as_raw_fd(), libc::FIONBIO, &mut nonblocking) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(AsyncRoute {
            fd: AsyncFd::new(fd)?,
        })
    }
    pub async fn read_with<R>(
        &mut self,
        mut op: impl FnMut(&mut T) -> io::Result<R>,
    ) -> io::Result<R> {
        self.fd
            .async_io_mut(Interest::READABLE.add(Interest::ERROR), |fd| op(fd))
            .await
    }
    pub async fn write_with<R>(
        &mut self,
        mut op: impl FnMut(&mut T) -> io::Result<R>,
    ) -> io::Result<R> {
        self.fd.async_io_mut(Interest::WRITABLE, |fd| op(fd)).await
    }
}
