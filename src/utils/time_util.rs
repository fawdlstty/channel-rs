use chrono::NaiveDateTime;

pub trait NaiveDateTimeExt {
    fn now() -> Self;
}

impl NaiveDateTimeExt for NaiveDateTime {
    fn now() -> Self {
        chrono::Utc::now().naive_local()
    }
}
