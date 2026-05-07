fn main() {
    if let Err(err) = study_guardian::native_messaging::run_stdio_host() {
        eprintln!("native messaging host failed: {err}");
        std::process::exit(1);
    }
}
