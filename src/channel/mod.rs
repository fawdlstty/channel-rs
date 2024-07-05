pub mod time_series;

use std::collections::HashMap;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Debug)]
pub(crate) struct UnboundedBuffer<T> {
    buf: Vec<T>,
}

impl<T: Clone + Sized> UnboundedBuffer<T> {
    pub fn send(&mut self, data: T) {
        self.buf.push(data);
    }

    pub fn send_items(&mut self, data: Vec<T>) {
        self.buf.extend(data);
    }

    pub fn recv(&mut self) -> Option<T> {
        match self.buf.len() {
            0 => None,
            _ => Some(self.buf.remove(0)),
        }
    }

    pub fn recv_count(&mut self, recv_count: usize, force_count: bool) -> Vec<T> {
        let read_count = if recv_count <= self.buf.len() {
            recv_count
        } else if !force_count && self.buf.len() > 0 {
            self.buf.len() - recv_count
        } else {
            return vec![];
        };
        self.buf.drain(0..read_count).collect()
    }
}

impl<T> UnboundedBuffer<T> {
    pub fn new() -> Self {
        Self { buf: vec![] }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[derive(Debug)]
pub(crate) struct BoundedBuffer<T> {
    buf: Vec<T>,
    bounded: usize,
}

impl<T: Clone + Sized> BoundedBuffer<T> {
    pub fn send(&mut self, data: T) {
        self.buf.push(data);
        if self.buf.len() > self.bounded {
            self.buf.remove(0);
        }
    }

    pub fn send_items(&mut self, data: Vec<T>) {
        self.buf.extend(data);
        if self.buf.len() > self.bounded {
            self.buf = self.buf.split_at(self.buf.len() - self.bounded).1.to_vec();
        }
    }

    pub fn recv(&mut self) -> Option<T> {
        match self.buf.len() {
            0 => None,
            _ => Some(self.buf.remove(0)),
        }
    }

    pub fn recv_count(&mut self, recv_count: usize, force_count: bool) -> Vec<T> {
        let read_count = if recv_count <= self.buf.len() {
            recv_count
        } else if !force_count && self.buf.len() > 0 {
            self.buf.len() - recv_count
        } else {
            return vec![];
        };
        self.buf.drain(0..read_count).collect()
    }
}

impl<T> BoundedBuffer<T> {
    pub fn new(bounded: usize) -> Self {
        Self {
            buf: vec![],
            bounded,
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[derive(Debug)]
pub(crate) struct UnboundedDispatchBuffer<T> {
    buf: Vec<T>,
    receiver_poses: HashMap<usize, usize>,
}

impl<T: Clone + Sized> UnboundedDispatchBuffer<T> {
    pub fn send(&mut self, data: T) {
        self.buf.push(data);
    }

    pub fn send_items(&mut self, data: Vec<T>) {
        self.buf.extend(data);
    }

    pub fn recv(&mut self, recver_index: usize) -> Option<T> {
        let mut ret = None;
        {
            let cur_pos = self.receiver_poses.entry(recver_index).or_insert(0);
            if *cur_pos < self.buf.len() {
                ret = Some(self.buf[*cur_pos].clone());
                *cur_pos += 1;
            }
        }
        self.reset_cache_base();
        ret
    }

    pub fn recv_count(
        &mut self,
        recver_index: usize,
        recv_count: usize,
        force_count: bool,
    ) -> Vec<T> {
        let mut ret = vec![];
        {
            let cur_pos = self.receiver_poses.entry(recver_index).or_insert(0);
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
        ret
    }
}

impl<T> UnboundedDispatchBuffer<T> {
    pub fn new() -> Self {
        Self {
            buf: vec![],
            receiver_poses: HashMap::new(),
        }
    }

    pub fn len(&self, recver_index: usize) -> usize {
        let cur_pos = &self.receiver_poses.get(&recver_index).cloned().unwrap_or(0);
        self.buf.len() - cur_pos
    }

    pub fn new_receiver(&mut self, recver_index: usize) {
        self.receiver_poses.insert(recver_index, 0);
    }

    pub fn drop_receiver(&mut self, recver_index: usize) {
        self.receiver_poses.remove(&recver_index);
        self.reset_cache_base();
    }

    fn reset_cache_base(&mut self) {
        if let Some(min_pos) = self.receiver_poses.values().min().cloned().take() {
            if min_pos > 0 {
                self.buf.drain(0..min_pos);
                for (_, val) in self.receiver_poses.iter_mut() {
                    *val -= min_pos;
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct BoundedDispatchBuffer<T> {
    buf: Vec<T>,
    receiver_poses: HashMap<usize, usize>,
    bounded: usize,
}

impl<T: Clone + Sized> BoundedDispatchBuffer<T> {
    pub fn send(&mut self, data: T) {
        self.buf.push(data);
        if self.buf.len() > self.bounded {
            self.buf.remove(0);
            for receiver_pos in self.receiver_poses.values_mut() {
                if *receiver_pos > 0 {
                    *receiver_pos -= 1;
                }
            }
        }
    }

    pub fn send_items(&mut self, data: Vec<T>) {
        self.buf.extend(data);
        if self.buf.len() > self.bounded {
            let remove_count = self.buf.len() - self.bounded;
            for receiver_pos in self.receiver_poses.values_mut() {
                *receiver_pos = match *receiver_pos >= remove_count {
                    true => *receiver_pos - remove_count,
                    false => 0,
                };
            }
            self.buf = self.buf.split_at(remove_count).1.to_vec();
        }
    }

    pub fn recv(&mut self, recver_index: usize) -> Option<T> {
        let mut ret = None;
        {
            let cur_pos = self.receiver_poses.entry(recver_index).or_insert(0);
            if *cur_pos < self.buf.len() {
                ret = Some(self.buf[*cur_pos].clone());
                *cur_pos += 1;
            }
        }
        self.reset_cache_base();
        ret
    }

    pub fn recv_count(
        &mut self,
        recver_index: usize,
        recv_count: usize,
        force_count: bool,
    ) -> Vec<T> {
        let mut ret = vec![];
        {
            let cur_pos = self.receiver_poses.entry(recver_index).or_insert(0);
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
        ret
    }
}

impl<T> BoundedDispatchBuffer<T> {
    pub fn new(bounded: usize) -> Self {
        Self {
            buf: vec![],
            receiver_poses: HashMap::new(),
            bounded,
        }
    }

    pub fn len(&self, recver_index: usize) -> usize {
        let cur_pos = self.receiver_poses.get(&recver_index).cloned().unwrap_or(0);
        self.buf.len() - cur_pos
    }

    pub fn new_receiver(&mut self, recver_index: usize) {
        self.receiver_poses.insert(recver_index, 0);
    }

    pub fn drop_receiver(&mut self, recver_index: usize) {
        self.receiver_poses.remove(&recver_index);
        self.reset_cache_base();
    }

    fn reset_cache_base(&mut self) {
        if let Some(min_pos) = self.receiver_poses.values().min().cloned().take() {
            if min_pos > 0 {
                self.buf.drain(0..min_pos);
                for (_, val) in self.receiver_poses.iter_mut() {
                    *val -= min_pos;
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum AnyBuffer<T> {
    UnboundedBuffer(UnboundedBuffer<T>),
    BoundedBuffer(BoundedBuffer<T>),
    UnboundedDispatchBuffer(UnboundedDispatchBuffer<T>),
    BoundedDispatchBuffer(BoundedDispatchBuffer<T>),
}

impl<T: Clone + Sized> AnyBuffer<T> {
    pub fn send(&mut self, data: T) {
        match self {
            AnyBuffer::UnboundedBuffer(buf) => buf.send(data),
            AnyBuffer::BoundedBuffer(buf) => buf.send(data),
            AnyBuffer::UnboundedDispatchBuffer(buf) => buf.send(data),
            AnyBuffer::BoundedDispatchBuffer(buf) => buf.send(data),
        }
    }

    pub fn send_items(&mut self, data: Vec<T>) {
        match self {
            AnyBuffer::UnboundedBuffer(buf) => buf.send_items(data),
            AnyBuffer::BoundedBuffer(buf) => buf.send_items(data),
            AnyBuffer::UnboundedDispatchBuffer(buf) => buf.send_items(data),
            AnyBuffer::BoundedDispatchBuffer(buf) => buf.send_items(data),
        }
    }

    pub fn recv(&mut self, recver_index: usize) -> Option<T> {
        match self {
            AnyBuffer::UnboundedBuffer(buf) => buf.recv(),
            AnyBuffer::BoundedBuffer(buf) => buf.recv(),
            AnyBuffer::UnboundedDispatchBuffer(buf) => buf.recv(recver_index),
            AnyBuffer::BoundedDispatchBuffer(buf) => buf.recv(recver_index),
        }
    }

    pub fn recv_count(
        &mut self,
        recver_index: usize,
        recv_count: usize,
        force_count: bool,
    ) -> Vec<T> {
        match self {
            AnyBuffer::UnboundedBuffer(buf) => buf.recv_count(recv_count, force_count),
            AnyBuffer::BoundedBuffer(buf) => buf.recv_count(recv_count, force_count),
            AnyBuffer::UnboundedDispatchBuffer(buf) => {
                buf.recv_count(recver_index, recv_count, force_count)
            }
            AnyBuffer::BoundedDispatchBuffer(buf) => {
                buf.recv_count(recver_index, recv_count, force_count)
            }
        }
    }
}

impl<T> AnyBuffer<T> {
    pub fn new(bounded: Option<usize>, dispatch: bool) -> Self {
        match (bounded, dispatch) {
            (Some(bounded), false) => AnyBuffer::BoundedBuffer(BoundedBuffer::<T>::new(bounded)),
            (None, false) => AnyBuffer::UnboundedBuffer(UnboundedBuffer::<T>::new()),
            (Some(bounded), true) => {
                AnyBuffer::BoundedDispatchBuffer(BoundedDispatchBuffer::<T>::new(bounded))
            }
            (None, true) => AnyBuffer::UnboundedDispatchBuffer(UnboundedDispatchBuffer::<T>::new()),
        }
    }

    pub fn len(&self, recver_index: usize) -> usize {
        match self {
            AnyBuffer::UnboundedBuffer(buf) => buf.len(),
            AnyBuffer::BoundedBuffer(buf) => buf.len(),
            AnyBuffer::UnboundedDispatchBuffer(buf) => buf.len(recver_index),
            AnyBuffer::BoundedDispatchBuffer(buf) => buf.len(recver_index),
        }
    }

    pub fn new_receiver(&mut self, recver_index: usize) {
        match self {
            AnyBuffer::UnboundedBuffer(_) => {}
            AnyBuffer::BoundedBuffer(_) => {}
            AnyBuffer::UnboundedDispatchBuffer(buf) => buf.new_receiver(recver_index),
            AnyBuffer::BoundedDispatchBuffer(buf) => buf.new_receiver(recver_index),
        }
    }

    pub fn drop_receiver(&mut self, recver_index: usize) {
        match self {
            AnyBuffer::UnboundedBuffer(_) => {}
            AnyBuffer::BoundedBuffer(_) => {}
            AnyBuffer::UnboundedDispatchBuffer(buf) => buf.drop_receiver(recver_index),
            AnyBuffer::BoundedDispatchBuffer(buf) => buf.drop_receiver(recver_index),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Channel<T> {
    sender_count: AtomicUsize,
    receiver_count: AtomicUsize,
    max_receiver_index: AtomicUsize,
    buf: Mutex<AnyBuffer<T>>,
}

impl<T> Channel<T> {
    pub fn new(bounded: Option<usize>, dispatch: bool) -> (Sender<T>, Receiver<T>) {
        let chan = NonNull::from(Box::leak(Box::new(Channel {
            sender_count: AtomicUsize::new(1),
            receiver_count: AtomicUsize::new(1),
            max_receiver_index: AtomicUsize::new(1),
            buf: Mutex::new(AnyBuffer::new(bounded, dispatch)),
        })));
        (Sender { chan }, Receiver { chan, index: 0 })
    }
}

pub struct Sender<T> {
    chan: NonNull<Channel<T>>,
}

impl<T: Clone + Sized> Sender<T> {
    pub fn send(&self, data: T) {
        let chan: *mut Channel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.send(data);
    }

    pub fn send_items(&self, data: Vec<T>) {
        let chan: *mut Channel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.send_items(data);
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

pub struct Receiver<T> {
    chan: NonNull<Channel<T>>,
    index: usize,
}

impl<T: Clone + Sized> Receiver<T> {
    pub fn recv(&self) -> Option<T> {
        let chan: *mut Channel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.recv(self.index)
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

impl<T> Receiver<T> {
    pub fn len(&self) -> usize {
        let buf = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        buf.len(self.index)
    }

    pub fn is_empty(&self) -> bool {
        let buf = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        buf.len(self.index) == 0
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
        buf.new_receiver(index);
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
