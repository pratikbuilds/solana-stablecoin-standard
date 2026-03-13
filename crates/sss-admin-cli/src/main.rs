fn main() {
    if let Err(error) = sss_admin_cli::run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
