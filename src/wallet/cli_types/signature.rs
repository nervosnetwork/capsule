use serde::{Serialize, Deserialize};

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct SignatureOutput {
    pub signature: String,
    pub recoverable: bool,
}
