fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .compile(
            &[
                "../proto/common.proto",
                "../proto/crypto.proto",
                "../proto/battery.proto",
                "../proto/auth.proto",
                "../proto/lifecycle.proto",
            ],
            &[".."],
        )?;
    Ok(())
}
