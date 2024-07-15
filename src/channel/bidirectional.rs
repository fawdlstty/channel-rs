use std::{collections::HashMap, ptr::NonNull, sync::Mutex};

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
            BDUnbRequester::<T1, T2> { buf },
            BDUnbResponder::<T1, T2> { buf },
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
}

impl<T1, T2> BDUnbRequester<T1, T2> {
    pub fn send_request(&self, data: T1) -> usize {
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        buf.send_request(data)
    }

    pub fn get_response(&self, token: usize) -> Option<T2> {
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        buf.get_response(token)
    }
}

impl<T1, T2> Clone for BDUnbRequester<T1, T2> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
        }
    }
}

pub struct BDUnbResponder<T1, T2> {
    buf: NonNull<Mutex<BDUnbBuffer<T1, T2>>>,
}

impl<T1, T2> BDUnbResponder<T1, T2> {
    pub fn take_request(&self) -> Option<(usize, T1)> {
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        buf.take_request()
    }

    pub fn reply_response(&self, token: usize, data: T2) {
        let mut buf = unsafe { self.buf.clone().as_mut().lock().unwrap() };
        buf.reply_response(token, data)
    }
}

impl<T1, T2> Clone for BDUnbResponder<T1, T2> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
        }
    }
}
