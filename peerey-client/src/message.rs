pub struct PeereyMessage {
    pub name: String,
    pub message: String,
}

impl From<[String; 2]> for PeereyMessage {
    fn from(mut src: [String; 2]) -> Self {
        Self {
            // String::new() doesn't allocate
            name: std::mem::replace(&mut src[0], String::new()),
            message: std::mem::replace(&mut src[1], String::new()),
        }
    }
}

impl std::fmt::Display for PeereyMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}: {}", self.name.trim(), self.message.trim())
    }
}
