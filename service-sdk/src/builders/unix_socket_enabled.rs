#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnixSocketMode {
    Disabled,
    Auto,
    H1,
    H2,
}

impl UnixSocketMode {
    pub fn unix_socket_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

impl Default for UnixSocketMode {
    fn default() -> Self {
        match std::env::var("UNIX_SOCKET") {
            Ok(v) => match v.to_uppercase().as_str() {
                "1" | "AUTO" => Self::Auto,
                "H1" | "HTTP1" => Self::H1,
                "H2" | "HTTP2" => Self::H2,
                _ => Self::Disabled,
            },
            Err(_) => Self::Disabled,
        }
    }
}
