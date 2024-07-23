use async_sema::Semaphore;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(crate) struct UnboundedBufferAsync<T> {
    buf: Vec<T>,
}

impl<T> UnboundedBufferAsync<T> {
    pub fn new() -> (UnboundedSenderAsync<T>, UnboundedReceiverAsync<T>) {
        let buf = NonNull::from(Box::leak(Box::new(Mutex::new(UnboundedBufferAsync {
            buf: vec![],
        }))));
        let sema = Arc::new(Semaphore::new(0));
        let sema2 = Arc::clone(&sema);
        (
            UnboundedSenderAsync { buf, sema: sema2 },
            UnboundedReceiverAsync { buf, sema },
        )
    }
}

pub struct UnboundedSenderAsync<T> {
    buf: NonNull<Mutex<UnboundedBufferAsync<T>>>,
    sema: Arc<Semaphore>,
}

impl<T> UnboundedSenderAsync<T> {
    pub fn send(&self, data: T) {
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        buf.buf.push(data);
        self.sema.add_permits(1);
    }
}

impl<T> Clone for UnboundedSenderAsync<T> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            sema: self.sema.clone(),
        }
    }
}

unsafe impl<T> Send for UnboundedSenderAsync<T> {}

pub struct UnboundedReceiverAsync<T> {
    buf: NonNull<Mutex<UnboundedBufferAsync<T>>>,
    sema: Arc<Semaphore>,
}

impl<T> UnboundedReceiverAsync<T> {
    pub async fn recv(&self) -> T {
        self.sema.acquire().await;
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        buf.buf.remove(0)
    }

    pub async fn recv_timeout(&self, dur: Duration) -> T {
        self.sema.acquire_timeout(dur).await;
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        buf.buf.remove(0)
    }
}

impl<T> Clone for UnboundedReceiverAsync<T> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            sema: self.sema.clone(),
        }
    }
}

unsafe impl<T> Send for UnboundedReceiverAsync<T> {}
