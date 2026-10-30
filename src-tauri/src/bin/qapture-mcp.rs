fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    match runtime {
        Ok(runtime) => {
            if let Err(error) = runtime.block_on(qapture_lib::bridge::run()) {
                eprintln!("qapture-mcp: {error}");
            }
        }
        Err(error) => eprintln!("qapture-mcp: {error}"),
    }
}
