use crate::utils::time_util::NaiveDateTimeExt;
use chrono::NaiveDateTime;

pub trait GetDataTimeExt {
    fn get_data_time(&self) -> NaiveDateTime;
}

#[derive(Debug)]
pub(crate) struct TimeSeriesBuffer<T> {
    buf: Vec<T>,
    start_data_time: NaiveDateTime,
    start_cur_time: NaiveDateTime,
    speed: f64,
}

impl<T: Clone + Sized + GetDataTimeExt> TimeSeriesBuffer<T> {
    pub fn send(&mut self, data: T) {
        self.buf.push(data);
    }

    pub fn send_items(&mut self, data: Vec<T>) {
        self.buf.extend(data);
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
        dest_nanos >= cur_nanos
    }
}

impl<T> TimeSeriesBuffer<T> {
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
