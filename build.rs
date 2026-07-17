use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_prost_build::configure()
        .type_attribute(
            ".fvs2d.v1.Layer",
            "#[derive(serde::Serialize, serde::Deserialize)] #[serde(rename_all = \"kebab-case\")]",
        )
        .type_attribute(
            ".fvs2d.v1.CommitSelector",
            "#[derive(serde::Serialize, serde::Deserialize)] #[serde(rename_all = \"kebab-case\")]",
        )
        .compile_protos(&["upstream/proto/fvs2d.proto"], &["upstream/proto/"])?;
    Ok(())
}
