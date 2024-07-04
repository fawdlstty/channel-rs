# channel-rs

[English](README.md) | 简体中文

Rust 高级队列库

## 简述

此库主要面向于队列的高级应用场景，用于简化逻辑代码。

## 使用手册

安装：在项目目录下运行 `cargo add channel`

### 无边界队列

特性：无限缓存容量，生产者和消费者可以有多个，一条消息只能被消费一次

```rust
let (tx, rx) = channel::new_unbounded();
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
let (tx, rx) = channel::new_bounded(4);
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
let (tx, rx) = channel::new_unbounded_dispatch();
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
let (tx, rx) = channel::new_bounded_dispatch(4);
tx.send_items(vec![1, 2, 3, 4]);
tx.send(5);
let rx2 = rx.clone();
let a = rx.recv_items(3);       // vec![2, 3, 4]
let a = rx2.recv_items(3);      // vec![2, 3, 4]
let a = rx.recv_items_weak(3);  // vec![5]
let a = rx2.recv_items_weak(3); // vec![5]
```
