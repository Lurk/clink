use crate::service;

pub fn execute() -> Result<(), String> {
    service::uninstall()
}
