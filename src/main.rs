use ghostbind::cli;

fn main() {
    if let Err(e) = cli::run_cli() {
        eprintln!("Error: {}", e);

        // Print the error chain
        let mut source = e.source();
        while let Some(err) = source {
            eprintln!("Caused by: {}", err);
            source = err.source();
        }

        std::process::exit(1);
    }
}
