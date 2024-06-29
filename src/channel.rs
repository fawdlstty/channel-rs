use std::collections::HashMap;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Debug)]
pub(crate) struct BaseBuffer<T> {
    buf: Vec<T>,
    rever_poses: Option<HashMap<usize, usize>>,
    bounded: Option<usize>,
}

impl<T: Clone + Sized> BaseBuffer<T> {
    pub fn recv(&mut self, recver_index: usize) -> Option<T> {
        match self.rever_poses.as_mut() {
            Some(rever_poses) => {
                let mut ret = None;
                {
                    let cur_pos = rever_poses.entry(recver_index).or_insert(0);
                    if *cur_pos < self.buf.len() {
                        ret = Some(self.buf[*cur_pos].clone());
                        *cur_pos += 1;
                    }
                }
                self.reset_cache_base();
                ret
            }
            None => match self.buf.len() {
                0 => None,
                _ => Some(self.buf.remove(0)),
            },
        }
    }

    pub fn recv_count(
        &mut self,
        recver_index: usize,
        recv_count: usize,
        force_count: bool,
    ) -> Vec<T> {
        let mut ret = vec![];
        match self.rever_poses.as_mut() {
            Some(rever_poses) => {
                {
                    let cur_pos = rever_poses.entry(recver_index).or_insert(0);
                    let read_count = if *cur_pos + recv_count <= self.buf.len() {
                        recv_count
                    } else if !force_count && *cur_pos < self.buf.len() {
                        self.buf.len() - *cur_pos
                    } else {
                        return ret;
                    };
                    for i in 0..read_count {
                        ret.push(self.buf[i].clone());
                    }
                    *cur_pos += read_count;
                }
                self.reset_cache_base();
            }
            None => {
                let read_count = if recv_count <= self.buf.len() {
                    recv_count
                } else if !force_count && self.buf.len() > 0 {
                    self.buf.len() - recv_count
                } else {
                    return ret;
                };
                ret.extend(self.buf.drain(0..read_count));
            }
        }
        ret
    }
}

impl<T> BaseBuffer<T> {
    pub fn len(&self, recver_index: usize) -> usize {
        let cur_pos = match &self.rever_poses {
            Some(rever_poses) => rever_poses.get(&recver_index).cloned().unwrap_or(0),
            None => 0,
        };
        self.buf.len() - cur_pos
    }

    pub fn drop_receiver(&mut self, recver_index: usize) {
        if let Some(rever_poses) = self.rever_poses.as_mut() {
            rever_poses.remove(&recver_index);
        }
        self.reset_cache_base();
    }

    fn reset_cache_base(&mut self) {
        if let Some(rever_poses) = self.rever_poses.as_mut() {
            if let Some(min_pos) = rever_poses.values().min().cloned().take() {
                if min_pos > 0 {
                    self.buf.drain(0..min_pos);
                    for (_, val) in rever_poses.iter_mut() {
                        *val -= min_pos;
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Channel<T> {
    sender_count: AtomicUsize,
    receiver_count: AtomicUsize,
    max_receiver_index: AtomicUsize,
    buf: Mutex<BaseBuffer<T>>,
}

impl<T> Channel<T> {
    pub fn new(bounded: Option<usize>, dispatch: bool) -> (Sender<T>, Receiver<T>) {
        let chan = NonNull::from(Box::leak(Box::new(Channel {
            sender_count: AtomicUsize::new(1),
            receiver_count: AtomicUsize::new(1),
            max_receiver_index: AtomicUsize::new(1),
            buf: Mutex::new(BaseBuffer {
                buf: vec![],
                rever_poses: match dispatch {
                    true => Some(vec![(0, 0)].into_iter().collect()),
                    false => None,
                },
                bounded,
            }),
        })));
        (Sender { chan }, Receiver { chan, index: 0 })
    }
}

pub struct Sender<T> {
    chan: NonNull<Channel<T>>,
}

impl<T> std::fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let chan = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        f.debug_struct("Receiver")
            .field("chan.buf.len()", &chan.buf.len())
            .field("chan.rever_poses", &chan.rever_poses)
            .field("chan.bounded", &chan.bounded)
            .finish()
    }
}

impl<T> Sender<T> {
    pub fn send(&self, data: T) {
        let chan: *mut Channel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        match buf.bounded {
            Some(bounded) => {
                if buf.buf.len() >= bounded {
                    buf.buf.remove(0);
                }
                buf.buf.push(data);
            }
            None => buf.buf.push(data),
        }
    }

    pub fn send_items(&self, data: Vec<T>) {
        let chan: *mut Channel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        match buf.bounded {
            Some(bounded) => {
                for _ in bounded..buf.buf.len() {
                    buf.buf.remove(0);
                }
                buf.buf.extend(data);
            }
            None => buf.buf.extend(data),
        }
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let chan: *mut Channel<T> = unsafe { std::mem::transmute(self.chan) };
        unsafe { &(*chan) }
            .sender_count
            .fetch_add(1, Ordering::SeqCst);
        Self {
            chan: self.chan.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let chan = unsafe { self.chan.as_mut() };
        let count = chan.sender_count.fetch_sub(1, Ordering::SeqCst);
        if count == 1 && chan.receiver_count.load(Ordering::SeqCst) == 0 {
            unsafe { ptr::drop_in_place(self.chan.as_ptr()) };
        }
    }
}

pub trait GetTimestampExt {
    fn get_timestamp(&self) -> u64;
}

pub struct Receiver<T> {
    chan: NonNull<Channel<T>>,
    index: usize,
}

impl<T> std::fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let chan = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        f.debug_struct("Receiver")
            .field("chan.buf.len()", &chan.buf.len())
            .field("chan.rever_poses", &chan.rever_poses)
            .field("chan.bounded", &chan.bounded)
            .field("index", &self.index)
            .finish()
    }
}

impl<T: Clone + Sized> Receiver<T> {
    pub fn recv(&self) -> Option<T> {
        let chan: *mut Channel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.recv(self.index)
    }

    pub fn len(&self) -> usize {
        let buf = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        buf.len(self.index)
    }

    pub fn is_empty(&self) -> bool {
        let buf = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        buf.len(self.index) == 0
    }

    pub fn recv_items(&self, count: usize) -> Vec<T> {
        let chan: *mut Channel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.recv_count(self.index, count, true)
    }

    pub fn recv_items_weak(&self, max_count: usize) -> Vec<T> {
        let chan: *mut Channel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.recv_count(self.index, max_count, false)
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        let chan: *mut Channel<T> = unsafe { std::mem::transmute(self.chan) };
        unsafe { &(*chan) }
            .receiver_count
            .fetch_add(1, Ordering::SeqCst);
        let index = unsafe { &(*chan) }
            .max_receiver_index
            .fetch_add(1, Ordering::SeqCst);
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        if let Some(rever_poses) = buf.rever_poses.as_mut() {
            rever_poses.insert(index, 0);
        }
        Self {
            chan: self.chan,
            index,
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let chan = unsafe { self.chan.as_mut() };
        let mut buf = chan.buf.lock().unwrap();
        buf.drop_receiver(self.index);
        let count = chan.receiver_count.fetch_sub(1, Ordering::SeqCst);
        if count == 1 && chan.sender_count.load(Ordering::SeqCst) == 0 {
            unsafe { ptr::drop_in_place(self.chan.as_ptr()) };
        }
    }
}
