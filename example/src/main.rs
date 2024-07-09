use channel::utils::time_util::NaiveDateTimeExt;
use chrono::{Duration, NaiveDateTime};

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

fn main() {
    let (tx, rx) = channel::new_time_series_unbounded_dispatch(NaiveDateTime::now(), 1.0);
    let wrx = rx.weak();
    tx.send_items(vec![
        MyTSStruct::new(NaiveDateTime::now() - Duration::milliseconds(10), 111),
        MyTSStruct::new(NaiveDateTime::now() + Duration::milliseconds(10), 222),
    ]);
    assert_eq!(rx.recv().unwrap().data, 111);
    let tx2 = wrx.lock();
    assert_eq!(tx2.len(), 1);
    println!("ok");
}
