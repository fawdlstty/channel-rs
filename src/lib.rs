pub mod channel;
#[cfg(test)]
pub mod test;
pub mod utils;

use channel::Channel;
pub use channel::{Receiver, Sender};

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
