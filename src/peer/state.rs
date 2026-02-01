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

    pub fn set_interested(&mut self, interested: bool) {
        self.interested = interested;
    }

    pub fn is_choked(&self) -> bool {
        self.choked
    }

    pub fn is_interested(&self) -> bool {
        self.interested
    }
}
