use std::time::SystemTime;

pub fn timestamp_seconds() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_secs().to_string()
}

pub fn timestamp_millis() -> u64 {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as u64
}
