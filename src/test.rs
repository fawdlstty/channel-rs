use crate::utils::time_util::NaiveDateTimeExt;
use crate::{self as channel};
use chrono::{Duration, NaiveDateTime};
use std::thread::sleep;

#[test]
fn test_it_works() {
    assert!(true);
}

#[test]
fn test_new_unbounded() {
    let (tx, rx) = channel::new_unbounded();
    tx.send_items(vec![1, 2, 3, 4]);
    tx.send(5);
    assert_eq!(rx.len(), 5);
    let rx2 = rx.clone();
    assert_eq!(rx.recv().unwrap(), 1);
    assert_eq!(rx2.recv_items(3), vec![2, 3, 4]);
    assert_eq!(rx.recv().unwrap(), 5);
}

#[test]
fn test_new_bounded() {
    let (tx, rx) = channel::new_bounded(4);
    tx.send_items(vec![1, 2, 3, 4]);
    tx.send(5);
    let rx2 = rx.clone();
    assert_eq!(rx.recv_items(2), vec![2, 3]);
    assert_eq!(rx2.recv_items(2), vec![4, 5]);
    assert!(rx.is_empty());
}

#[test]
fn test_new_unbounded_dispatch() {
    let (tx, rx) = channel::new_unbounded_dispatch();
    tx.send_items(vec![1, 2, 3, 4]);
    tx.send(5);
    let rx2 = rx.clone();
    assert_eq!(rx.recv_items(3), vec![1, 2, 3]);
    assert_eq!(rx2.recv_items(3), vec![1, 2, 3]);
    assert_eq!(rx.recv_items_weak(3), vec![4, 5]);
    assert_eq!(rx2.recv_items_weak(3), vec![4, 5]);
}

#[test]
fn test_new_bounded_dispatch() {
    let (tx, rx) = channel::new_bounded_dispatch(4);
    tx.send_items(vec![1, 2, 3, 4]);
    tx.send(5);
    let rx2 = rx.clone();
    assert_eq!(rx.recv_items(3), vec![2, 3, 4]);
    assert_eq!(rx2.recv_items(3), vec![2, 3, 4]);
    assert_eq!(rx.recv_items_weak(3), vec![5]);
    assert_eq!(rx2.recv_items_weak(3), vec![5]);
}

#[derive(Clone, Debug)]
struct MyTSStruct {
    time: NaiveDateTime,
    data: i32,
}

impl MyTSStruct {
    pub fn new(time: NaiveDateTime, data: i32) -> Self {
        Self { time, data }
    }
}

impl channel::GetDataTimeExt for MyTSStruct {
    fn get_data_time(&self) -> NaiveDateTime {
        self.time.clone()
    }
}

#[test]
fn test_new_time_series_unbounded() {
    let (tx, rx) = channel::new_time_series_unbounded(NaiveDateTime::now(), 1.0);
    tx.send_items(vec![
        MyTSStruct::new(
            NaiveDateTime::now() - chrono::Duration::milliseconds(10),
            111,
        ),
        MyTSStruct::new(
            NaiveDateTime::now() + chrono::Duration::milliseconds(10),
            222,
        ),
    ]);
    assert_eq!(rx.len(), 2);
    let rx2 = rx.clone();
    assert_eq!(rx.recv().unwrap().data, 111);
    assert!(rx2.recv().is_none());
    sleep(std::time::Duration::from_millis(10));
    assert_eq!(rx2.recv().unwrap().data, 222);
}

#[test]
fn test_new_unbounded_weak() {
    let (tx, rx) = channel::new_unbounded_dispatch();
    let ox = rx.get_observer();
    tx.send_items(vec![1, 2, 3, 4]);
    tx.send(5);
    assert_eq!(rx.recv_items(3), vec![1, 2, 3]);
    assert_eq!(rx.recv_items_weak(3), vec![4, 5]);
    let tx2 = ox.get_receiver();
    assert_eq!(tx2.len(), 0);
}

#[test]
fn test_new_time_series_unbounded_weak() {
    let (tx, rx) = channel::new_time_series_unbounded_dispatch(NaiveDateTime::now(), 1.0);
    let ox = rx.get_observer();
    tx.send_items(vec![
        MyTSStruct::new(NaiveDateTime::now() - Duration::milliseconds(10), 111),
        MyTSStruct::new(NaiveDateTime::now() + Duration::milliseconds(10), 222),
    ]);
    assert_eq!(rx.recv().unwrap().data, 111);
    let tx2 = ox.get_receiver();
    assert_eq!(tx2.len(), 1);
}
