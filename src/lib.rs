pub mod channel;
#[cfg(test)]
pub mod test;

use channel::Channel;
pub use channel::{Receiver, Sender};

pub fn new_unbounded<T: Copy + Send>() -> (Sender<T>, Receiver<T>) {
    Channel::new(None, false)
}

pub fn new_bounded<T: Copy + Send>(bounded: usize) -> (Sender<T>, Receiver<T>) {
    Channel::new(Some(bounded), false)
}

pub fn new_unbounded_dispatch<T: Copy + Send>() -> (Sender<T>, Receiver<T>) {
    Channel::new(None, true)
}

pub fn new_bounded_dispatch<T: Copy + Send>(bounded: usize) -> (Sender<T>, Receiver<T>) {
    Channel::new(Some(bounded), true)
}
