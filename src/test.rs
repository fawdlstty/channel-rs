use crate as channel;

#[test]
fn test_it_works() {
    assert!(true);
}

#[cfg(not(feature = "time-series"))]
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

#[cfg(not(feature = "time-series"))]
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

#[cfg(not(feature = "time-series"))]
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

#[cfg(not(feature = "time-series"))]
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
