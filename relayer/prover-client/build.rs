fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        //.build_server(false)
        //.build_client(false)
        .compile_protos(
            &[
                "../../proto/prover/v1/prover.proto",
                "../../proto/ibc/lightclients/groth16/v1/groth16.proto",
            ],
            &["../../proto"],
        )?;
    Ok(())
}
