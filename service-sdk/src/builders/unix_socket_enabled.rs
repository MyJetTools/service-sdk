pub fn unix_socket_enabled() -> bool {
    match std::env::var("UNIX_SOCKET") {
        Ok(value) => value == "1",
        Err(_) => false,
    }
}
