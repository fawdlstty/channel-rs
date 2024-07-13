# channel-rs

![version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Ffawdlstty%2Fchannel-rs%2Fmain%2FCargo.toml&query=package.version&label=version)
![status](https://img.shields.io/github/actions/workflow/status/fawdlstty/channel-rs/rust.yml)

[English](README.md) | 简体中文

Rust 高级队列库

## 简述

此库主要面向于队列的高级应用场景，用于简化逻辑代码。

## 使用手册

安装：在项目目录下运行 `cargo add channel`

### 无边界队列

特性：无限缓存容量，生产者和消费者可以有多个，一条消息只能被消费一次

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

### 有边界队列

特性：只缓存指定的数量，超过则覆盖最早的数据，生产者和消费者可以有多个，一条消息只能被消费一次

```rust
let (tx, rx) = channel::new(Some(4), false);
tx.send_items(vec![1, 2, 3, 4]);
tx.send(5);
let rx2 = rx.clone();
let a = rx.recv_items(2);  // vec![2, 3]
let b = rx2.recv_items(2); // vec![4, 5]
let c = rx.is_empty();     // true
```

### 无边界分发队列

特性：无限缓存容量，生产者和消费者可以有多个，任一条消息将被所有消费者所消费

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

### 有边界分发队列

特性：只缓存指定的数量，超过则覆盖最早的数据，生产者和消费者可以有多个，任一条消息只要没被覆盖则将被所有消费者所消费

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

### 时序队列

特性：效果几乎等同以上队列，但增加一个特性，必须在数据达到时间后才能被接收。可以理解为在播放视频文件时讲帧延迟推送至屏幕

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

### 观测者

特性：观测者不直接接收管道数据，但可以检测当前缓存使用量以及直接从缓存里提取数据。观测者可以和接收者互相转换

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
```
