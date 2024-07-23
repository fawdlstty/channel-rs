pub mod asynchronous;
pub mod bidirectional;
pub mod time_series;

use crate::utils::vec_utils::VecExt;
use std::collections::HashMap;
use std::ptr::{self, NonNull};
use std::sync::Mutex;

#[cfg(feature = "metrics")]
use {
    crate::utils::metrics_utils::{HolderType, MetricsManager, MetricsResult},
    std::panic::Location,
};

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

    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        self.buf.query_items(start, end)
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

    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        self.buf.query_items(start, end)
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

    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        self.buf.query_items(start, end)
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

    pub fn part_queue_get_residue_count(&mut self, extern_size: usize) -> usize {
        if extern_size >= self.bounded {
            self.buf.clear();
            for (_, val) in self.receiver_poses.iter_mut() {
                *val = 0;
            }
        } else {
            let self_residue_count = self.bounded - extern_size;
            if self.buf.len() > self_residue_count {
                let remove_count = self.buf.len() - self_residue_count;
                for receiver_pos in self.receiver_poses.values_mut() {
                    *receiver_pos = match *receiver_pos >= remove_count {
                        true => *receiver_pos - remove_count,
                        false => 0,
                    };
                }
                self.buf = self.buf.split_at(remove_count).1.to_vec();
            }
        }
        self.bounded
    }

    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        self.buf.query_items(start, end)
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

    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        match self {
            AnyBuffer::UnboundedBuffer(buf) => buf.query_items(start, end),
            AnyBuffer::BoundedBuffer(buf) => buf.query_items(start, end),
            AnyBuffer::UnboundedDispatchBuffer(buf) => buf.query_items(start, end),
            AnyBuffer::BoundedDispatchBuffer(buf) => buf.query_items(start, end),
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
    sender_count: usize,
    receiver_count: usize,
    max_receiver_index: usize,
    buf: AnyBuffer<T>,
    #[cfg(feature = "metrics")]
    metrics_mgr: MetricsManager,
}

impl<T> Channel<T> {
    pub fn new(
        bounded: Option<usize>,
        dispatch: bool,
        #[cfg(feature = "metrics")] caller: &'static Location<'static>,
    ) -> (Sender<T>, Receiver<T>) {
        #[cfg(feature = "metrics")]
        let (metrics_mgr, sender_metrics_idx, receiver_metrics_idx) = {
            let mut metrics_mgr = MetricsManager::new();
            let sender_idx = metrics_mgr.new_metrics_index(caller, HolderType::Sender);
            let receiver_idx = metrics_mgr.new_metrics_index(caller, HolderType::Receiver);
            (metrics_mgr, sender_idx, receiver_idx)
        };
        let chan = NonNull::from(Box::leak(Box::new(Mutex::new(Channel {
            sender_count: 1,
            receiver_count: 1,
            max_receiver_index: 1,
            buf: AnyBuffer::new(bounded, dispatch),
            #[cfg(feature = "metrics")]
            metrics_mgr,
        }))));
        (
            Sender {
                chan,
                #[cfg(feature = "metrics")]
                metrics_idx: sender_metrics_idx,
            },
            Receiver {
                chan,
                index: 0,
                #[cfg(feature = "metrics")]
                metrics_idx: receiver_metrics_idx,
            },
        )
    }

    #[cfg(feature = "metrics")]
    fn new_metrics_index(
        &mut self,
        caller: &'static Location<'static>,
        holder_type: HolderType,
    ) -> usize {
        self.metrics_mgr.new_metrics_index(caller, holder_type)
    }

    #[cfg(feature = "metrics")]
    pub fn get_metrics_result(&mut self, clear: bool) -> MetricsResult {
        self.metrics_mgr.get_result(clear)
    }
}

pub struct Sender<T> {
    chan: NonNull<Mutex<Channel<T>>>,
    #[cfg(feature = "metrics")]
    metrics_idx: usize,
}

impl<T: Clone + Sized> Sender<T> {
    pub fn send(&self, data: T) {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        #[cfg(feature = "metrics")]
        chan.metrics_mgr.record(self.metrics_idx, 1);
        chan.buf.send(data);
    }

    pub fn send_items(&self, data: Vec<T>) {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        #[cfg(feature = "metrics")]
        chan.metrics_mgr.record(self.metrics_idx, data.len());
        chan.buf.send_items(data);
    }
}

impl<T> Clone for Sender<T> {
    #[cfg(feature = "metrics")]
    #[track_caller]
    fn clone(&self) -> Self {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.sender_count += 1;
        Self {
            chan: self.chan,
            metrics_idx: chan.new_metrics_index(Location::caller(), HolderType::Sender),
        }
    }

    #[cfg(not(feature = "metrics"))]
    fn clone(&self) -> Self {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.sender_count += 1;
        Self { chan: self.chan }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.sender_count -= 1;
        if chan.sender_count == 0 && chan.receiver_count == 0 {
            unsafe { ptr::drop_in_place(self.chan.as_ptr()) };
        }
    }
}

pub struct Receiver<T> {
    chan: NonNull<Mutex<Channel<T>>>,
    index: usize,
    #[cfg(feature = "metrics")]
    metrics_idx: usize,
}

impl<T: Clone + Sized> Receiver<T> {
    pub fn recv(&self) -> Option<T> {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        let ret = chan.buf.recv(self.index);
        #[cfg(feature = "metrics")]
        if ret.is_some() {
            chan.metrics_mgr.record(self.metrics_idx, 1);
        }
        ret
    }

    pub fn recv_items(&self, count: usize) -> Vec<T> {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        let ret = chan.buf.recv_count(self.index, count, true);
        #[cfg(feature = "metrics")]
        if ret.len() > 0 {
            chan.metrics_mgr.record(self.metrics_idx, ret.len());
        }
        ret
    }

    pub fn recv_items_weak(&self, max_count: usize) -> Vec<T> {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        let ret = chan.buf.recv_count(self.index, max_count, false);
        #[cfg(feature = "metrics")]
        if ret.len() > 0 {
            chan.metrics_mgr.record(self.metrics_idx, ret.len());
        }
        ret
    }
}

impl<T> Receiver<T> {
    pub fn len(&self) -> usize {
        let chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.buf.len(self.index)
    }

    pub fn is_empty(&self) -> bool {
        let chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.buf.len(self.index) == 0
    }

    pub fn get_observer(&self) -> Observer<T> {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.receiver_count += 1;
        _ = chan.max_receiver_index += 1;
        Observer { chan: self.chan }
    }
}

impl<T> Clone for Receiver<T> {
    #[cfg(feature = "metrics")]
    #[track_caller]
    fn clone(&self) -> Self {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.receiver_count += 1;
        let index = chan.max_receiver_index;
        chan.max_receiver_index += 1;
        chan.buf.new_receiver(index);
        Self {
            chan: self.chan,
            index,
            metrics_idx: chan.new_metrics_index(Location::caller(), HolderType::Receiver),
        }
    }

    #[cfg(not(feature = "metrics"))]
    fn clone(&self) -> Self {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.receiver_count += 1;
        let index = chan.max_receiver_index;
        chan.max_receiver_index += 1;
        chan.buf.new_receiver(index);
        Self {
            chan: self.chan,
            index,
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.buf.drop_receiver(self.index);
        chan.receiver_count -= 1;
        if chan.receiver_count == 0 && chan.sender_count == 0 {
            unsafe { ptr::drop_in_place(self.chan.as_ptr()) };
        }
    }
}

pub struct Observer<T> {
    chan: NonNull<Mutex<Channel<T>>>,
}

impl<T: Clone + Sized> Observer<T> {
    pub fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        let chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.buf.query_items(start, end)
    }
}

impl<T> Observer<T> {
    pub fn len(&self) -> usize {
        let chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.buf.len(usize::MAX)
    }

    pub fn is_empty(&self) -> bool {
        let chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.buf.len(usize::MAX) == 0
    }

    #[cfg(feature = "metrics")]
    #[track_caller]
    pub fn get_receiver(&self) -> Receiver<T> {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.receiver_count += 1;
        let index = chan.max_receiver_index;
        chan.max_receiver_index += 1;
        chan.buf.new_receiver(index);
        Receiver {
            chan: self.chan,
            index,
            metrics_idx: chan.new_metrics_index(Location::caller(), HolderType::Receiver),
        }
    }

    #[cfg(not(feature = "metrics"))]
    pub fn get_receiver(&self) -> Receiver<T> {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.receiver_count += 1;
        let index = chan.max_receiver_index;
        chan.max_receiver_index += 1;
        chan.buf.new_receiver(index);
        Receiver {
            chan: self.chan,
            index,
        }
    }

    #[cfg(feature = "metrics")]
    pub fn get_metrics_result(&mut self, clear: bool) -> MetricsResult {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.get_metrics_result(clear)
    }
}

impl<T> Drop for Observer<T> {
    fn drop(&mut self) {
        let mut chan = unsafe { self.chan.clone().as_mut().lock().unwrap() };
        chan.receiver_count -= 1;
        if chan.receiver_count == 0 && chan.sender_count == 0 {
            unsafe { ptr::drop_in_place(self.chan.as_ptr()) };
        }
    }
}
