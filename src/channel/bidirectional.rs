use super::asynchronous::*;
use async_sema::Semaphore;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::Mutex;

pub(crate) struct BDUnbBuffer<T1, T2> {
    next_token: usize,
    req_buf: Vec<(usize, T1)>,
    resp_buf: HashMap<usize, T2>,
}

impl<T1, T2> BDUnbBuffer<T1, T2> {
    pub fn new() -> (BDUnbRequester<T1, T2>, BDUnbResponder<T1, T2>) {
        let buf = NonNull::from(Box::leak(Box::new(Mutex::new(BDUnbBuffer {
            next_token: 0,
            req_buf: vec![],
            resp_buf: HashMap::new(),
        }))));
        (
            BDUnbRequester::<T1, T2> {
                buf,
                cache_tokens: vec![],
            },
            BDUnbResponder::<T1, T2> {
                buf,
                cache_tokens: vec![],
            },
        )
    }

    pub fn send_request(&mut self, data: T1) -> usize {
        let ret = self.next_token;
        self.req_buf.push((self.next_token, data));
        self.next_token += 1;
        ret
    }

    pub fn get_response(&mut self, token: usize) -> Option<T2> {
        self.resp_buf.remove(&token)
    }

    pub fn take_request(&mut self) -> Option<(usize, T1)> {
        match self.req_buf.len() > 0 {
            true => Some(self.req_buf.remove(0)),
            false => None,
        }
    }

    pub fn reply_response(&mut self, token: usize, data: T2) {
        self.resp_buf.insert(token, data);
    }
}

pub struct BDUnbRequester<T1, T2> {
    buf: NonNull<Mutex<BDUnbBuffer<T1, T2>>>,
    cache_tokens: Vec<usize>,
}

impl<T1, T2> BDUnbRequester<T1, T2> {
    pub fn send_request(&mut self, data: T1) {
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        self.cache_tokens.push(buf.send_request(data));
    }

    pub fn send_requests(&mut self, data: Vec<T1>) {
        for item in data {
            self.send_request(item);
        }
    }

    pub fn try_get_response(&mut self) -> Option<T2> {
        match self.cache_tokens.is_empty() {
            true => None,
            false => {
                let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
                match buf.get_response(self.cache_tokens[0]) {
                    Some(data) => {
                        self.cache_tokens.remove(0);
                        Some(data)
                    }
                    None => None,
                }
            }
        }
    }
}

impl<T1, T2> Clone for BDUnbRequester<T1, T2> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            cache_tokens: vec![],
        }
    }
}

pub struct BDUnbResponder<T1, T2> {
    buf: NonNull<Mutex<BDUnbBuffer<T1, T2>>>,
    cache_tokens: Vec<usize>,
}

impl<T1, T2> BDUnbResponder<T1, T2> {
    pub fn try_take_request(&mut self) -> Option<T1> {
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        match buf.take_request() {
            Some((token, data)) => {
                self.cache_tokens.push(token);
                Some(data)
            }
            None => None,
        }
    }

    pub fn reply_response(&mut self, data: T2) {
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        let token = self.cache_tokens.remove(0);
        buf.reply_response(token, data)
    }
}

impl<T1, T2> Clone for BDUnbResponder<T1, T2> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            cache_tokens: vec![],
        }
    }
}

pub(crate) struct BDUnbBufferAsync<T1, T2> {
    req_buf: Vec<(T1, UnboundedSenderAsync<T2>)>,
}

impl<T1, T2> BDUnbBufferAsync<T1, T2> {
    pub fn new() -> (BDUnbRequesterAsync<T1, T2>, BDUnbResponderAsync<T1, T2>) {
        let buf = NonNull::from(Box::leak(Box::new(Mutex::new(BDUnbBufferAsync {
            req_buf: vec![],
        }))));
        let sema = Semaphore::new(0);
        (
            BDUnbRequesterAsync::<T1, T2> {
                buf,
                sema: sema.clone(),
            },
            BDUnbResponderAsync::<T1, T2> { buf, sema },
        )
    }

    pub fn request(&mut self, data: T1) -> UnboundedReceiverAsync<T2> {
        let (tx, rx) = UnboundedBufferAsync::new();
        self.req_buf.push((data, tx));
        rx
    }

    pub async fn take_request(&mut self) -> (T1, UnboundedSenderAsync<T2>) {
        self.req_buf.remove(0)
    }
}

pub struct BDUnbRequesterAsync<T1, T2> {
    buf: NonNull<Mutex<BDUnbBufferAsync<T1, T2>>>,
    sema: Semaphore,
}

impl<T1, T2> Clone for BDUnbRequesterAsync<T1, T2> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            sema: self.sema.clone(),
        }
    }
}

impl<T1, T2> BDUnbRequesterAsync<T1, T2> {
    pub async fn request(&mut self, data: T1) -> T2 {
        let receiver = {
            let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
            buf.request(data)
        };
        receiver.recv().await
    }
}

unsafe impl<T1, T2> Send for BDUnbRequesterAsync<T1, T2> {}

pub struct BDUnbResponderAsync<T1, T2> {
    buf: NonNull<Mutex<BDUnbBufferAsync<T1, T2>>>,
    sema: Semaphore,
}

impl<T1, T2> BDUnbResponderAsync<T1, T2> {
    pub async fn take_request(&mut self) -> (T1, UnboundedSenderAsync<T2>) {
        self.sema.acquire().await;
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        buf.take_request().await
    }

    pub async fn try_take_request(&mut self) -> Option<(T1, UnboundedSenderAsync<T2>)> {
        match self.sema.try_acquire() {
            true => {
                let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
                Some(buf.take_request().await)
            }
            false => None,
        }
    }
}

impl<T1, T2> Clone for BDUnbResponderAsync<T1, T2> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            sema: self.sema.clone(),
        }
    }
}

unsafe impl<T1, T2> Send for BDUnbResponderAsync<T1, T2> {}
