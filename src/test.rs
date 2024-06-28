use crate as channel;

#[test]
fn test_it_works() {
    assert!(true);
}

#[test]
fn test_new_unbounded() {
    let (tx, rx) = channel::new_unbounded();
    tx.send_items(vec![1, 2, 3, 4]);
    tx.send(5);
    let rx2 = rx.clone();
    assert_eq!(rx.recv().unwrap(), 1);
    assert_eq!(rx2.recv().unwrap(), 2);
    assert_eq!(rx.recv().unwrap(), 3);
    assert_eq!(rx2.recv().unwrap(), 4);
    assert_eq!(rx.recv().unwrap(), 5);
}

#[test]
fn test_new_bounded() {
    let (tx, rx) = channel::new_bounded(4);
    tx.send_items(vec![1, 2, 3, 4]);
    tx.send(5);
    let rx2 = rx.clone();
    assert_eq!(rx.recv().unwrap(), 2);
    assert_eq!(rx2.recv().unwrap(), 3);
    assert_eq!(rx.recv().unwrap(), 4);
    assert_eq!(rx2.recv().unwrap(), 5);
    assert!(rx.recv().is_none());
}

#[test]
fn test_new_unbounded_dispatch() {
    let (tx, rx) = channel::new_unbounded_dispatch();
    tx.send_items(vec![1, 2, 3, 4]);
    tx.send(5);
    let rx2 = rx.clone();
    assert_eq!(rx.recv().unwrap(), 1);
    assert_eq!(rx.recv().unwrap(), 2);
    assert_eq!(rx.recv().unwrap(), 3);
    assert_eq!(rx.recv().unwrap(), 4);
    assert_eq!(rx.recv().unwrap(), 5);
    assert_eq!(rx2.recv().unwrap(), 1);
    assert_eq!(rx2.recv().unwrap(), 2);
}

#[test]
fn test_new_bounded_dispatch() {
    let (tx, rx) = channel::new_bounded_dispatch(4);
    tx.send_items(vec![1, 2, 3, 4]);
    tx.send(5);
    let rx2 = rx.clone();
    assert_eq!(rx.recv().unwrap(), 2);
    assert_eq!(rx2.recv().unwrap(), 2);
    assert_eq!(rx.recv().unwrap(), 3);
    assert_eq!(rx2.recv().unwrap(), 3);
    assert_eq!(rx.recv().unwrap(), 4);
    assert_eq!(rx.recv().unwrap(), 5);
    assert!(rx.recv().is_none());
    assert_eq!(rx2.recv().unwrap(), 4);
    assert_eq!(rx2.recv().unwrap(), 5);
}
