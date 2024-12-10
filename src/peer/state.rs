#[derive(Debug, Clone)]
pub struct PeerState {
    choked: bool,
    interested: bool,
}

impl PeerState {
    pub fn new() -> Self {
        Self {
            choked: true,
            interested: false,
        }
    }
    pub fn choke(&mut self) {
        self.choked = true;
    }

    pub fn unchoke(&mut self) {
        self.choked = false;
    }
}
