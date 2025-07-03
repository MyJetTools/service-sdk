use rust_extensions::StrOrString;

pub trait ServiceInfo {
    fn get_service_name(&self) -> StrOrString<'static>;
    fn get_service_version(&self) -> StrOrString<'static>;
}
