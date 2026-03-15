use uuid::Uuid;

pub fn generate_device_id() -> Uuid {
    Uuid::new_v4()
}