# channel-rs

English | [简体中文](README.zh_CN.md)

Rust advanced queue library

## Summary

This library is mainly for queue advanced application scenarios, used to simplify logical code.

## Manual

Install: Run `cargo add channel` in the project directory

### Unbounded queue

Features: Unlimited cache capacity, producers and consumers can have multiple, a message can only be consumed once

```rust
let (tx, rx) = channel::new_unbounded();
tx.send_items(vec![1, 2, 3, 4]);
tx.send(5);
let rx2 = rx.clone();
assert_eq!(rx.recv().unwrap(), 1);
assert_eq!(rx2.recv().unwrap(), 2);
assert_eq!(rx.recv().unwrap(), 3);
assert_eq!(rx2.recv().unwrap(), 4);
assert_eq!(rx.recv().unwrap(), 5);
```

### Bounded queue

Features: Only the specified amount is cached, beyond which the earliest data is overwritten, producers and consumers can have multiple, and a message can only be consumed once

```rust
let (tx, rx) = channel::new_bounded(4);
tx.send_items(vec![1, 2, 3, 4]);
tx.send(5);
let rx2 = rx.clone();
assert_eq!(rx.recv().unwrap(), 2);
assert_eq!(rx2.recv().unwrap(), 3);
assert_eq!(rx.recv().unwrap(), 4);
assert_eq!(rx2.recv().unwrap(), 5);
assert!(rx.recv().is_none());
```

### Unbounded dispatch queue

Features: The maximum number of caches is theoretical, there can be multiple producers and consumers, and any message will be consumed by all consumers

```rust
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
```

### Bounded dispatch queue

Features: Only the specified amount of cache, more than overwrite the earliest data, producers and consumers can have multiple, any message as long as it is not overwritten will be consumed by all consumers

```rust
let (tx, rx) = channel::new_bounded_dispatch(4);
tx.send(1);
tx.send(2);
tx.send(3);
tx.send(4);
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
```

### TODO 、len、is_empty、recv_items、recv_items_weak
