# channel-rs

![version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Ffawdlstty%2Fchannel-rs%2Fmain%2FCargo.toml&query=package.version&label=version)
![status](https://img.shields.io/github/actions/workflow/status/fawdlstty/channel-rs/rust.yml)

English | [简体中文](README.zh_CN.md)

Rust advanced queue library

## Summary

This library is mainly for queue advanced application scenarios, used to simplify logical code.

## Manual

Install: Run `cargo add channel` in the project directory

### Unbounded queue

Features: Unlimited cache capacity, producers and consumers can have multiple, a message can only be consumed once

```rust
let (tx, rx) = channel::new(None, false);
tx.send_items(vec![1, 2, 3, 4]);
tx.send(5);
let a = rx.len();           // 5
let rx2 = rx.clone();
let b = rx.recv().unwrap(); // 1
let c = rx2.recv_items(3);  // vec![2, 3, 4]
let d = rx.recv().unwrap(); // 5
```

### Bounded queue

Features: Only the specified amount is cached, beyond which the earliest data is overwritten, producers and consumers can have multiple, and a message can only be consumed once

```rust
let (tx, rx) = channel::new(Some(4), false);
tx.send_items(vec![1, 2, 3, 4]);
tx.send(5);
let rx2 = rx.clone();
let a = rx.recv_items(2);  // vec![2, 3]
let b = rx2.recv_items(2); // vec![4, 5]
let c = rx.is_empty();     // true
```

### Unbounded dispatch queue

Features: The maximum number of caches is theoretical, there can be multiple producers and consumers, and any message will be consumed by all consumers

```rust
let (tx, rx) = channel::new(None, true);
tx.send_items(vec![1, 2, 3, 4]);
tx.send(5);
let rx2 = rx.clone();
let a = rx.recv_items(3);       // vec![1, 2, 3]
let b = rx2.recv_items(3);      // vec![1, 2, 3]
let c = rx.recv_items_weak(3);  // vec![4, 5]
let d = rx2.recv_items_weak(3); // vec![4, 5]
```

### Bounded dispatch queue

Features: Only the specified amount of cache, more than overwrite the earliest data, producers and consumers can have multiple, any message as long as it is not overwritten will be consumed by all consumers

```rust
let (tx, rx) = channel::new(Some(4), true);
tx.send_items(vec![1, 2, 3, 4]);
tx.send(5);
let rx2 = rx.clone();
let a = rx.recv_items(3);       // vec![2, 3, 4]
let a = rx2.recv_items(3);      // vec![2, 3, 4]
let a = rx.recv_items_weak(3);  // vec![5]
let a = rx2.recv_items_weak(3); // vec![5]
```

### time series queue

Features: The effect is almost the same as the above queue, but with the addition of a feature that must be received after the data reaches the time. It can be understood as pushing the frame delay to the screen when playing the video file

```rust
#[derive(Clone)]
struct MyTSStruct {
    time: NaiveDateTime,
    data: i32,
}

impl MyTSStruct {
    pub fn new(time: NaiveDateTime, data: i32) -> Self { Self { time, data } }
}

impl channel::GetDataTimeExt for MyTSStruct {
    fn get_data_time(&self) -> NaiveDateTime { self.time.clone() }
}

// ...
let (tx, rx) = channel::new_time_series(None, false, NaiveDateTime::now(), 1.0);
// let (tx, rx) = channel::new_time_series(Some(10), false, NaiveDateTime::now(), 1.0);
// let (tx, rx) = channel::new_time_series(None, true, NaiveDateTime::now(), 1.0);
// let (tx, rx) = channel::new_time_series(Some(10), true, NaiveDateTime::now(), 1.0);
tx.send_items(vec![
    MyTSStruct::new(NaiveDateTime::now() - chrono::Duration::milliseconds(10), 111),
    MyTSStruct::new(NaiveDateTime::now() + chrono::Duration::milliseconds(10), 222),
]);
let a = rx.len(); // 2
let rx2 = rx.clone();
let b = rx.recv().unwrap().data; // 111
let c = rx2.recv().is_none(); // true
sleep(Duration::from_millis(10));
let d = rx2.recv().unwrap().data; // 222
```

### observer

Features: The observer does not receive pipeline data directly, but can detect the current cache usage and extract data directly from the cache. The observer and the receiver can be interchangeable

```rust
let (tx, rx) = channel::new_time_series(None, true, NaiveDateTime::now(), 1.0);
let ox = rx.get_observer();
tx.send_items(vec![
    MyTSStruct::new(NaiveDateTime::now() - Duration::milliseconds(10), 111),
    MyTSStruct::new(NaiveDateTime::now() + Duration::milliseconds(10), 222),
]);
let a = rx.recv().unwrap().data; // 111
let tx2 = ox.get_receiver();
let b = tx2.len(); // 1

// The following code is available when the `metrics` feature is enabled
let result = ox.get_metrics_result(true);
println!("{:?}", result);
let send_count: usize = result.sender_counts.iter().map(|(_, v)| *v).sum(); // 2
let recv_count: usize = result.receiver_counts.iter().map(|(_, v)| *v).sum(); // 1
```
