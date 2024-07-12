use std::{
    ptr::{self, NonNull},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

use crate::utils::{time_util::NaiveDateTimeExt, vec_utils::VecExt};
use chrono::NaiveDateTime;

use super::{BoundedDispatchBuffer, UnboundedDispatchBuffer};

pub trait GetDataTimeExt {
    fn get_data_time(&self) -> NaiveDateTime;
}

#[derive(Debug)]
pub(crate) struct TSUnboundedBuffer<T> {
    buf: Vec<T>,
    start_data_time: NaiveDateTime,
    start_cur_time: NaiveDateTime,
    speed: f64,
}

impl<T: Clone + Sized + GetDataTimeExt> TSUnboundedBuffer<T> {
    pub fn send(&mut self, data: T) {
        self.buf.push(data);
    }

    pub fn send_items(&mut self, data: Vec<T>) {
        self.buf.extend(data);
    }

    pub fn recv(&mut self) -> Option<T> {
        if self.is_valid(0) {
            return Some(self.buf.remove(0));
        }
        None
    }

    pub fn recv_count(&mut self, recv_count: usize, force_count: bool) -> Vec<T> {
        let mut read_count = if recv_count <= self.buf.len() {
            recv_count
        } else if !force_count && self.buf.len() > 0 {
            self.buf.len() - recv_count
        } else {
            return vec![];
        };
        while read_count > 0 {
            if self.is_valid(read_count - 1) {
                break;
            }
            read_count -= 1;
        }
        self.buf.drain(0..read_count).collect()
    }

    fn is_valid(&self, index: usize) -> bool {
        if self.buf.len() <= index {
            return false;
        }
        let dest_nanos = (self.buf[index].get_data_time() - self.start_data_time)
            .num_nanoseconds()
            .unwrap_or(0);
        let cur_nanos = (NaiveDateTime::now() - self.start_cur_time)
            .num_nanoseconds()
            .unwrap_or(0);
        let cur_nanos = (cur_nanos as f64 * self.speed).round() as i64;
        dest_nanos <= cur_nanos
    }

    pub fn part_queue_apply_bound(&mut self, bound: usize) {
        if self.buf.len() > bound {
            self.buf = self.buf.split_at(self.buf.len() - bound).1.to_vec();
        }
    }

    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        self.buf.query_items(start, end)
    }
}

impl<T> TSUnboundedBuffer<T> {
    pub fn new(start_data_time: NaiveDateTime, speed: f64) -> Self {
        Self {
            buf: vec![],
            start_data_time,
            start_cur_time: NaiveDateTime::now(),
            speed,
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[derive(Debug)]
pub(crate) struct TSBoundedBuffer<T> {
    buf: Vec<T>,
    bounded: usize,
    start_data_time: NaiveDateTime,
    start_cur_time: NaiveDateTime,
    speed: f64,
}

impl<T: Clone + Sized + GetDataTimeExt> TSBoundedBuffer<T> {
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
        if !self.buf.is_empty() {
            if self.is_valid(0) {
                return Some(self.buf.remove(0));
            }
        }
        None
    }

    pub fn recv_count(&mut self, recv_count: usize, force_count: bool) -> Vec<T> {
        let mut read_count = if recv_count <= self.buf.len() {
            recv_count
        } else if !force_count && self.buf.len() > 0 {
            self.buf.len() - recv_count
        } else {
            return vec![];
        };
        while read_count > 0 {
            if self.is_valid(read_count) {
                break;
            }
            read_count -= 1;
        }
        self.buf.drain(0..read_count).collect()
    }

    fn is_valid(&self, index: usize) -> bool {
        let dest_nanos = (self.buf[index].get_data_time() - self.start_data_time)
            .num_nanoseconds()
            .unwrap_or(0);
        let cur_nanos = (NaiveDateTime::now() - self.start_cur_time)
            .num_nanoseconds()
            .unwrap_or(0);
        let cur_nanos = (cur_nanos as f64 * self.speed).round() as i64;
        dest_nanos <= cur_nanos
    }

    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        self.buf.query_items(start, end)
    }
}

impl<T> TSBoundedBuffer<T> {
    pub fn new(bounded: usize, start_data_time: NaiveDateTime, speed: f64) -> Self {
        Self {
            buf: vec![],
            bounded,
            start_data_time,
            start_cur_time: NaiveDateTime::now(),
            speed,
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[derive(Debug)]
pub(crate) struct TSUnboundedDispatchBuffer<T> {
    pre_buffer: TSUnboundedBuffer<T>,
    post_buffer: UnboundedDispatchBuffer<T>,
}

impl<T: Clone + Sized + GetDataTimeExt> TSUnboundedDispatchBuffer<T> {
    pub fn send(&mut self, data: T) {
        self.pre_buffer.send(data);
    }

    pub fn send_items(&mut self, data: Vec<T>) {
        self.pre_buffer.send_items(data);
    }

    pub fn recv(&mut self, recver_index: usize) -> Option<T> {
        let tmp_data = self.pre_buffer.recv_count(self.pre_buffer.len(), false);
        self.post_buffer.send_items(tmp_data);
        self.post_buffer.recv(recver_index)
    }

    pub fn recv_count(
        &mut self,
        recver_index: usize,
        recv_count: usize,
        force_count: bool,
    ) -> Vec<T> {
        let tmp_data = self.pre_buffer.recv_count(self.pre_buffer.len(), false);
        self.post_buffer.send_items(tmp_data);
        self.post_buffer
            .recv_count(recver_index, recv_count, force_count)
    }

    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        let mut start = start;
        let mut end = end.unwrap_or(self.len(usize::MAX));
        let mut post_len = self.post_buffer.len(usize::MAX);
        let mut items = vec![];
        {
            let post_skip = post_len.min(start);
            if post_skip > 0 {
                start -= post_skip;
                end -= post_skip;
                post_len -= post_skip;
            }
        }
        if post_len > 0 {
            assert!(start == 0);
            let post_take = post_len.min(end);
            items.extend(self.post_buffer.query_items(0, Some(post_take)));
            end -= post_take;
            if end == 0 {
                return items;
            }
        }
        end = end.min(self.pre_buffer.len());
        if end > start {
            items.extend(self.pre_buffer.query_items(start, Some(end)));
        }
        items
    }
}

impl<T> TSUnboundedDispatchBuffer<T> {
    pub fn new(start_data_time: NaiveDateTime, speed: f64) -> Self {
        Self {
            pre_buffer: TSUnboundedBuffer::<T>::new(start_data_time, speed),
            post_buffer: UnboundedDispatchBuffer::<T>::new(),
        }
    }

    pub fn len(&self, recver_index: usize) -> usize {
        self.pre_buffer.len() + self.post_buffer.len(recver_index)
    }

    pub fn new_receiver(&mut self, recver_index: usize) {
        self.post_buffer.new_receiver(recver_index);
    }

    pub fn drop_receiver(&mut self, recver_index: usize) {
        self.post_buffer.drop_receiver(recver_index);
    }
}

#[derive(Debug)]
pub(crate) struct TSBoundedDispatchBuffer<T> {
    pre_buffer: TSUnboundedBuffer<T>,
    post_buffer: BoundedDispatchBuffer<T>,
}

impl<T: Clone + Sized + GetDataTimeExt> TSBoundedDispatchBuffer<T> {
    pub fn send(&mut self, data: T) {
        self.pre_buffer.send(data);
        let bound = self
            .post_buffer
            .part_queue_get_residue_count(self.pre_buffer.len());
        self.pre_buffer.part_queue_apply_bound(bound);
    }

    pub fn send_items(&mut self, data: Vec<T>) {
        self.pre_buffer.send_items(data);
        let bound = self
            .post_buffer
            .part_queue_get_residue_count(self.pre_buffer.len());
        self.pre_buffer.part_queue_apply_bound(bound);
    }

    pub fn recv(&mut self, recver_index: usize) -> Option<T> {
        let tmp_data = self.pre_buffer.recv_count(self.pre_buffer.len(), false);
        self.post_buffer.send_items(tmp_data);
        self.post_buffer.recv(recver_index)
    }

    pub fn recv_count(
        &mut self,
        recver_index: usize,
        recv_count: usize,
        force_count: bool,
    ) -> Vec<T> {
        let tmp_data = self.pre_buffer.recv_count(self.pre_buffer.len(), false);
        self.post_buffer.send_items(tmp_data);
        self.post_buffer
            .recv_count(recver_index, recv_count, force_count)
    }

    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        let mut start = start;
        let mut end = end.unwrap_or(self.len(usize::MAX));
        let mut post_len = self.post_buffer.len(usize::MAX);
        let mut items = vec![];
        {
            let post_skip = post_len.min(start);
            if post_skip > 0 {
                start -= post_skip;
                end -= post_skip;
                post_len -= post_skip;
            }
        }
        if post_len > 0 {
            assert!(start == 0);
            let post_take = post_len.min(end);
            items.extend(self.post_buffer.query_items(0, Some(post_take)));
            end -= post_take;
            if end == 0 {
                return items;
            }
        }
        end = end.min(self.pre_buffer.len());
        if end > start {
            items.extend(self.pre_buffer.query_items(start, Some(end)));
        }
        items
    }
}

impl<T> TSBoundedDispatchBuffer<T> {
    pub fn new(bounded: usize, start_data_time: NaiveDateTime, speed: f64) -> Self {
        Self {
            pre_buffer: TSUnboundedBuffer::<T>::new(start_data_time, speed),
            post_buffer: BoundedDispatchBuffer::<T>::new(bounded),
        }
    }

    pub fn len(&self, recver_index: usize) -> usize {
        self.pre_buffer.len() + self.post_buffer.len(recver_index)
    }

    pub fn new_receiver(&mut self, recver_index: usize) {
        self.post_buffer.new_receiver(recver_index);
    }

    pub fn drop_receiver(&mut self, recver_index: usize) {
        self.post_buffer.drop_receiver(recver_index);
    }
}

#[derive(Debug)]
pub(crate) enum TSAnyBuffer<T> {
    UnboundedBuffer(TSUnboundedBuffer<T>),
    BoundedBuffer(TSBoundedBuffer<T>),
    UnboundedDispatchBuffer(TSUnboundedDispatchBuffer<T>),
    BoundedDispatchBuffer(TSBoundedDispatchBuffer<T>),
}

impl<T: Clone + Sized + GetDataTimeExt> TSAnyBuffer<T> {
    pub fn send(&mut self, data: T) {
        match self {
            TSAnyBuffer::UnboundedBuffer(buf) => buf.send(data),
            TSAnyBuffer::BoundedBuffer(buf) => buf.send(data),
            TSAnyBuffer::UnboundedDispatchBuffer(buf) => buf.send(data),
            TSAnyBuffer::BoundedDispatchBuffer(buf) => buf.send(data),
        }
    }

    pub fn send_items(&mut self, data: Vec<T>) {
        match self {
            TSAnyBuffer::UnboundedBuffer(buf) => buf.send_items(data),
            TSAnyBuffer::BoundedBuffer(buf) => buf.send_items(data),
            TSAnyBuffer::UnboundedDispatchBuffer(buf) => buf.send_items(data),
            TSAnyBuffer::BoundedDispatchBuffer(buf) => buf.send_items(data),
        }
    }

    pub fn recv(&mut self, recver_index: usize) -> Option<T> {
        match self {
            TSAnyBuffer::UnboundedBuffer(buf) => buf.recv(),
            TSAnyBuffer::BoundedBuffer(buf) => buf.recv(),
            TSAnyBuffer::UnboundedDispatchBuffer(buf) => buf.recv(recver_index),
            TSAnyBuffer::BoundedDispatchBuffer(buf) => buf.recv(recver_index),
        }
    }

    pub fn recv_count(
        &mut self,
        recver_index: usize,
        recv_count: usize,
        force_count: bool,
    ) -> Vec<T> {
        match self {
            TSAnyBuffer::UnboundedBuffer(buf) => buf.recv_count(recv_count, force_count),
            TSAnyBuffer::BoundedBuffer(buf) => buf.recv_count(recv_count, force_count),
            TSAnyBuffer::UnboundedDispatchBuffer(buf) => {
                buf.recv_count(recver_index, recv_count, force_count)
            }
            TSAnyBuffer::BoundedDispatchBuffer(buf) => {
                buf.recv_count(recver_index, recv_count, force_count)
            }
        }
    }

    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        match self {
            TSAnyBuffer::UnboundedBuffer(buf) => buf.query_items(start, end),
            TSAnyBuffer::BoundedBuffer(buf) => buf.query_items(start, end),
            TSAnyBuffer::UnboundedDispatchBuffer(buf) => buf.query_items(start, end),
            TSAnyBuffer::BoundedDispatchBuffer(buf) => buf.query_items(start, end),
        }
    }
}

impl<T> TSAnyBuffer<T> {
    pub fn new(
        bounded: Option<usize>,
        dispatch: bool,
        start_data_time: NaiveDateTime,
        speed: f64,
    ) -> Self {
        match (bounded, dispatch) {
            (None, false) => {
                TSAnyBuffer::UnboundedBuffer(TSUnboundedBuffer::<T>::new(start_data_time, speed))
            }
            (Some(bounded), false) => TSAnyBuffer::BoundedBuffer(TSBoundedBuffer::<T>::new(
                bounded,
                start_data_time,
                speed,
            )),
            (None, true) => TSAnyBuffer::UnboundedDispatchBuffer(
                TSUnboundedDispatchBuffer::<T>::new(start_data_time, speed),
            ),
            (Some(bounded), true) => TSAnyBuffer::BoundedDispatchBuffer(
                TSBoundedDispatchBuffer::<T>::new(bounded, start_data_time, speed),
            ),
        }
    }

    pub fn len(&self, recver_index: usize) -> usize {
        match self {
            TSAnyBuffer::UnboundedBuffer(buf) => buf.len(),
            TSAnyBuffer::BoundedBuffer(buf) => buf.len(),
            TSAnyBuffer::UnboundedDispatchBuffer(buf) => buf.len(recver_index),
            TSAnyBuffer::BoundedDispatchBuffer(buf) => buf.len(recver_index),
        }
    }

    pub fn new_receiver(&mut self, recver_index: usize) {
        match self {
            TSAnyBuffer::UnboundedBuffer(_) => (),
            TSAnyBuffer::BoundedBuffer(_) => (),
            TSAnyBuffer::UnboundedDispatchBuffer(buf) => buf.new_receiver(recver_index),
            TSAnyBuffer::BoundedDispatchBuffer(buf) => buf.new_receiver(recver_index),
        }
    }

    pub fn drop_receiver(&mut self, recver_index: usize) {
        match self {
            TSAnyBuffer::UnboundedBuffer(_) => (),
            TSAnyBuffer::BoundedBuffer(_) => (),
            TSAnyBuffer::UnboundedDispatchBuffer(buf) => buf.drop_receiver(recver_index),
            TSAnyBuffer::BoundedDispatchBuffer(buf) => buf.drop_receiver(recver_index),
        }
    }
}

#[derive(Debug)]
pub(crate) struct TSChannel<T> {
    sender_count: AtomicUsize,
    receiver_count: AtomicUsize,
    max_receiver_index: AtomicUsize,
    buf: Mutex<TSAnyBuffer<T>>,
}

impl<T> TSChannel<T> {
    pub fn new(
        bounded: Option<usize>,
        dispatch: bool,
        start_data_time: NaiveDateTime,
        speed: f64,
    ) -> (TSSender<T>, TSReceiver<T>) {
        let chan = NonNull::from(Box::leak(Box::new(TSChannel {
            sender_count: AtomicUsize::new(1),
            receiver_count: AtomicUsize::new(1),
            max_receiver_index: AtomicUsize::new(1),
            buf: Mutex::new(TSAnyBuffer::<T>::new(
                bounded,
                dispatch,
                start_data_time,
                speed,
            )),
        })));
        (TSSender { chan }, TSReceiver { chan, index: 0 })
    }
}

pub struct TSSender<T> {
    chan: NonNull<TSChannel<T>>,
}

impl<T: Clone + Sized + GetDataTimeExt> TSSender<T> {
    pub fn send(&self, data: T) {
        let chan: *mut TSChannel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.send(data);
    }

    pub fn send_items(&self, data: Vec<T>) {
        let chan: *mut TSChannel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.send_items(data);
    }
}

impl<T> Clone for TSSender<T> {
    fn clone(&self) -> Self {
        let chan: *mut TSChannel<T> = unsafe { std::mem::transmute(self.chan) };
        unsafe { &(*chan) }
            .sender_count
            .fetch_add(1, Ordering::SeqCst);
        Self {
            chan: self.chan.clone(),
        }
    }
}

impl<T> Drop for TSSender<T> {
    fn drop(&mut self) {
        let chan = unsafe { self.chan.as_mut() };
        let count = chan.sender_count.fetch_sub(1, Ordering::SeqCst);
        if count == 1 && chan.receiver_count.load(Ordering::SeqCst) == 0 {
            unsafe { ptr::drop_in_place(self.chan.as_ptr()) };
        }
    }
}

pub struct TSReceiver<T> {
    chan: NonNull<TSChannel<T>>,
    index: usize,
}

impl<T: Clone + Sized + GetDataTimeExt> TSReceiver<T> {
    pub fn recv(&self) -> Option<T> {
        let chan: *mut TSChannel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.recv(self.index)
    }

    pub fn recv_items(&self, count: usize) -> Vec<T> {
        let chan: *mut TSChannel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.recv_count(self.index, count, true)
    }

    pub fn recv_items_weak(&self, max_count: usize) -> Vec<T> {
        let chan: *mut TSChannel<T> = unsafe { std::mem::transmute(self.chan) };
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.recv_count(self.index, max_count, false)
    }
}

impl<T> TSReceiver<T> {
    pub fn len(&self) -> usize {
        let buf = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        buf.len(self.index)
    }

    pub fn is_empty(&self) -> bool {
        let buf = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        buf.len(self.index) == 0
    }

    pub fn get_observer(&self) -> TSObserver<T> {
        let chan: *mut TSChannel<T> = unsafe { std::mem::transmute(self.chan) };
        unsafe { &(*chan) }
            .receiver_count
            .fetch_add(1, Ordering::SeqCst);
        _ = unsafe { &(*chan) }
            .max_receiver_index
            .fetch_add(1, Ordering::SeqCst);
        TSObserver { chan: self.chan }
    }
}

impl<T> Clone for TSReceiver<T> {
    fn clone(&self) -> Self {
        let chan: *mut TSChannel<T> = unsafe { std::mem::transmute(self.chan) };
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

impl<T> Drop for TSReceiver<T> {
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

pub struct TSObserver<T> {
    chan: NonNull<TSChannel<T>>,
}

impl<T: Clone + Sized + GetDataTimeExt> TSObserver<T> {
    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        let buf = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        buf.query_items(start, end)
    }
}

impl<T> TSObserver<T> {
    pub fn len(&self) -> usize {
        let buf = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        buf.len(usize::MAX)
    }

    pub fn is_empty(&self) -> bool {
        let buf = unsafe { self.chan.as_ref() }.buf.lock().unwrap();
        buf.len(usize::MAX) == 0
    }

    pub fn get_receiver(&self) -> TSReceiver<T> {
        let chan: *mut TSChannel<T> = unsafe { std::mem::transmute(self.chan) };
        unsafe { &(*chan) }
            .receiver_count
            .fetch_add(1, Ordering::SeqCst);
        let index = unsafe { &(*chan) }
            .max_receiver_index
            .fetch_add(1, Ordering::SeqCst);
        let mut buf = unsafe { &(*chan) }.buf.lock().unwrap();
        buf.new_receiver(index);
        TSReceiver {
            chan: self.chan,
            index,
        }
    }
}

impl<T> Drop for TSObserver<T> {
    fn drop(&mut self) {
        let chan = unsafe { self.chan.as_mut() };
        let count = chan.receiver_count.fetch_sub(1, Ordering::SeqCst);
        if count == 1 && chan.sender_count.load(Ordering::SeqCst) == 0 {
            unsafe { ptr::drop_in_place(self.chan.as_ptr()) };
        }
    }
}
