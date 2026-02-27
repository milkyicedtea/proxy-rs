use chrono::Utc;
use chrono_tz::Europe::Rome;

pub fn now_eu() -> String {
    let t = Utc::now().with_timezone(&Rome);
    t.format("%Y-%m-%d %H:%M:%S").to_string()
}