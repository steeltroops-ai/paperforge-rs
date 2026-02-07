//! Build script for generating gRPC code from proto files

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell cargo to rerun if proto files change
    println!("cargo:rerun-if-changed=../../proto/");
    
    // Configure tonic-build
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let proto_dir = "../../proto";
    
    // Check if proto directory exists (it might not in all build contexts)
    let proto_path = std::path::Path::new(proto_dir);
    if !proto_path.exists() {
        println!("cargo:warning=Proto directory not found, skipping gRPC codegen");
        return Ok(());
    }
    
    // Compile all proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile(
            &[
                format!("{}/search.proto", proto_dir),
                format!("{}/ingestion.proto", proto_dir),
                format!("{}/context.proto", proto_dir),
                format!("{}/embedding.proto", proto_dir),
            ],
            &[proto_dir],
        )?;
    
    Ok(())
}
