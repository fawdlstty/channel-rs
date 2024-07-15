#[cfg(test)]
pub mod test;

#[cfg(feature = "metrics")]
use std::panic::Location;

pub mod channel;
pub mod utils;

pub use channel::bidirectional::{BDUnbRequester, BDUnbResponder};
pub use channel::time_series::{GetDataTimeExt, TSObserver, TSReceiver, TSSender};
pub use channel::{Observer, Receiver, Sender};

use channel::bidirectional::BDUnbBuffer;
use channel::{time_series::TSChannel, Channel};
use chrono::NaiveDateTime;

#[cfg(not(feature = "metrics"))]
pub fn new<T: Clone + Send>(bounded: Option<usize>, dispatch: bool) -> (Sender<T>, Receiver<T>) {
    Channel::new(bounded, dispatch)
}

#[cfg(feature = "metrics")]
#[track_caller]
pub fn new<T: Clone + Send>(bounded: Option<usize>, dispatch: bool) -> (Sender<T>, Receiver<T>) {
    Channel::new(bounded, dispatch, Location::caller())
}

#[cfg(not(feature = "metrics"))]
pub fn new_time_series<T: Clone + Send + GetDataTimeExt>(
    bounded: Option<usize>,
    dispatch: bool,
    start_data_time: NaiveDateTime,
    speed: f64,
) -> (TSSender<T>, TSReceiver<T>) {
    TSChannel::new(bounded, dispatch, start_data_time, speed)
}

#[cfg(feature = "metrics")]
#[track_caller]
pub fn new_time_series<T: Clone + Send + GetDataTimeExt>(
    bounded: Option<usize>,
    dispatch: bool,
    start_data_time: NaiveDateTime,
    speed: f64,
) -> (TSSender<T>, TSReceiver<T>) {
    TSChannel::new(
        bounded,
        dispatch,
        start_data_time,
        speed,
        Location::caller(),
    )
}

pub fn new_unbounded_bidirectional<T1, T2>() -> (BDUnbRequester<T1, T2>, BDUnbResponder<T1, T2>) {
    BDUnbBuffer::new()
}
