#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnixSocketMode {
    Disabled,
    Enabled,
    Only,
}

impl UnixSocketMode {
    pub fn from_env() -> Self {
        match std::env::var("UNIX_SOCKET") {
            Ok(v) if v == "1" => Self::Enabled,
            Ok(v) if v == "ONLY" => Self::Only,
            _ => Self::Disabled,
        }
    }

    pub fn unix_socket_enabled(self) -> bool {
        matches!(self, Self::Enabled | Self::Only)
    }

    pub fn tcp_enabled(self) -> bool {
        !matches!(self, Self::Only)
    }
}
