use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Args {
    pub string: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Result {
    pub length: usize,
}
