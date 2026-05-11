#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnixSocketMode {
    Disabled,
    WithTcp,
    UnixOnly,
}

impl UnixSocketMode {
    pub fn unix_socket_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    pub fn tcp_enabled(self) -> bool {
        !matches!(self, Self::UnixOnly)
    }
}

impl Default for UnixSocketMode {
    fn default() -> Self {
        match std::env::var("UNIX_SOCKET") {
            Ok(v) if v.eq_ignore_ascii_case("ONLY") => Self::UnixOnly,
            Ok(_) => Self::WithTcp,
            Err(_) => Self::Disabled,
        }
    }
}
