use uuid;

pub type Uuid = uuid::Uuid;

pub fn new() -> Uuid {
    uuid::Uuid::new_v4()
}
