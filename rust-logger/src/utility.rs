/// Generates the default output path for CSV files.
/// Creates a path to "log_gen_output.csv" in the current working directory.
/// 
/// # Returns
/// * `String` - Full path to the default CSV output file
/// 
/// # Panics
/// * Panics if current directory cannot be determined or converted to string
pub fn default_path() -> String {
    std::path::Path::new(&std::env::current_dir().unwrap())
        .join("log_gen_output.csv")
        .to_str()
        .unwrap()
        .to_string()
}