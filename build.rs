use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_prost_build::configure()
        .compile_protos(&["upstream/proto/fvs2d.proto"], &["upstream/proto/"])?;
    Ok(())
}
