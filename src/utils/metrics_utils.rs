use std::collections::HashMap;
use std::panic::Location;

#[derive(Debug)]
pub enum HolderType {
    Sender,
    Receiver,
}

#[derive(Debug)]
pub struct MetricsManager {
    caller_locs: Vec<String>,
    caller_holder_types: Vec<HolderType>,
    index_metrics: HashMap<usize, usize>,
}

impl MetricsManager {
    pub fn new() -> Self {
        Self {
            caller_locs: vec![],
            caller_holder_types: vec![],
            index_metrics: HashMap::new(),
        }
    }

    pub fn new_metrics_index(
        &mut self,
        caller: &'static Location<'static>,
        holder_type: HolderType,
    ) -> usize {
        let caller = format!("{}:{}", caller.file(), caller.line());
        let index = self.caller_locs.len();
        self.caller_locs.push(caller);
        self.caller_holder_types.push(holder_type);
        self.index_metrics.insert(index, 0);
        index
    }

    pub fn record(&mut self, index: usize, count: usize) {
        *self.index_metrics.entry(index).or_insert(0) += count;
    }

    pub fn get_result(&self) -> MetricsResult {
        let mut sender_counts = HashMap::new();
        let mut receiver_counts = HashMap::new();
        for (index, value) in self.index_metrics.iter() {
            let caller_str = self.caller_locs[*index].clone();
            match self.caller_holder_types[*index] {
                HolderType::Sender => sender_counts.insert(caller_str, *value),
                HolderType::Receiver => receiver_counts.insert(caller_str, *value),
            };
        }
        MetricsResult {
            sender_counts,
            receiver_counts,
        }
    }
}

pub struct MetricsResult {
    pub sender_counts: HashMap<String, usize>,
    pub receiver_counts: HashMap<String, usize>,
}
