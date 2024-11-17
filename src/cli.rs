use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    /// The path to the server config file
    #[arg(short, long, value_parser = validate_file)]
    pub config: Option<String>,
}

fn validate_file(path: &str) -> Result<String, String> {
    if std::path::Path::new(path).exists() {
        Ok(path.to_string())
    } else {
        Err(format!("File not found: {}", path))
    }
}
