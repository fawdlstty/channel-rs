fn main() {
    let (tx, rx) = channel::new_unbounded_dispatch();
    tx.send(1);
    tx.send(2);
    tx.send(3);
    tx.send(4);
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
