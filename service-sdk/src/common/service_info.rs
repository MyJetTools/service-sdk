
pub trait ServiceInfo {
    fn get_service_name(&self) -> &'static str;
    fn get_service_version(&self) -> &'static str;
}
