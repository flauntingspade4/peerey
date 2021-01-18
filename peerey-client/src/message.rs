use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct ReceiveMessage {
    pub name: String,
    pub message: String,
}

impl std::fmt::Display for ReceiveMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}: {}", self.name, self.message.trim())
    }
}

#[derive(Serialize, Debug)]
pub struct SendMessage<'a> {
    pub name: &'a str,
    pub message: String,
}

impl<'a> SendMessage<'a> {
    pub const fn new(name: &'a str, message: String) -> Self {
        Self { name, message }
    }
}
