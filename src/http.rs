use crate::config:: { DeviceInfo };
pub fn handle_probe() -> String {
    // todo handle errors in device config read
    let info = DeviceInfo::new().unwrap();
    let json = serde_json::to_string(&info).unwrap();
    json
}