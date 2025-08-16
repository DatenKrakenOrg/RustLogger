pub fn default_path() -> String {
    std::path::Path::new(&std::env::current_dir().unwrap())
    .join("log_gen_output.csv")
    .to_str()
    .unwrap()
    .to_string()
}