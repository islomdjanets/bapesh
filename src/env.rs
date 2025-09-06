use dotenv::dotenv;

pub fn get(name: &str) -> Result<String, std::env::VarError> {
    std::env::var(name)//.expect("variable not found");
}

pub fn ok() {
    dotenv().ok();
}
