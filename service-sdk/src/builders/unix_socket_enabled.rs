#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnixSocketHttpVersion {
    Http1,
    Http2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnixSocketMode {
    Disabled,
    Enabled,
    Only,
    EnabledH2,
    OnlyH2,
}

impl UnixSocketMode {
    pub fn unix_socket_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    pub fn tcp_enabled(self) -> bool {
        !matches!(self, Self::Only | Self::OnlyH2)
    }

    pub fn http_version(self) -> UnixSocketHttpVersion {
        match self {
            Self::EnabledH2 | Self::OnlyH2 => UnixSocketHttpVersion::Http2,
            _ => UnixSocketHttpVersion::Http1,
        }
    }
}

impl Default for UnixSocketMode {
    fn default() -> Self {
        match std::env::var("UNIX_SOCKET") {
            Ok(v) => match v.to_uppercase().as_str() {
                "1" => Self::Enabled,
                "ONLY" => Self::Only,
                "H2" | "HTTP2" => Self::EnabledH2,
                "ONLY_H2" | "ONLY_HTTP2" => Self::OnlyH2,
                _ => Self::Disabled,
            },
            Err(_) => Self::Disabled,
        }
    }
}
