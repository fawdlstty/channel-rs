pub mod channel;
#[cfg(test)]
pub mod test;
pub mod utils;

pub use channel::time_series::{GetDataTimeExt, TSReceiver, TSSender};
use channel::{time_series::TSChannel, Channel};
pub use channel::{Receiver, Sender};
use chrono::NaiveDateTime;

pub fn new_unbounded<T: Clone + Send>() -> (Sender<T>, Receiver<T>) {
    Channel::new(None, false)
}

pub fn new_bounded<T: Clone + Send>(bounded: usize) -> (Sender<T>, Receiver<T>) {
    Channel::new(Some(bounded), false)
}

pub fn new_unbounded_dispatch<T: Clone + Send>() -> (Sender<T>, Receiver<T>) {
    Channel::new(None, true)
}

pub fn new_bounded_dispatch<T: Clone + Send>(bounded: usize) -> (Sender<T>, Receiver<T>) {
    Channel::new(Some(bounded), true)
}

pub fn new_time_series_unbounded<T: Clone + Send>(
    start_data_time: NaiveDateTime,
    speed: f64,
) -> (TSSender<T>, TSReceiver<T>) {
    TSChannel::new(None, false, start_data_time, speed)
}
