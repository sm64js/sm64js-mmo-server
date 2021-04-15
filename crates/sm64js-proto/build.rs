fn main() {
    let mut config = prost_build::Config::new();
    config.type_attribute(".", "#[derive(serde::Serialize)]");
    #[cfg(feature = "docker")]
    const PROTO_FILES: [&str; 1] = ["../../proto-tmp/mario.proto"];
    #[cfg(not(feature = "docker"))]
    const PROTO_FILES: [&str; 1] = ["../../../proto/mario.proto"];
    #[cfg(feature = "docker")]
    const PROTO_INCLUDES: [&str; 1] = ["../../proto-tmp/"];
    #[cfg(not(feature = "docker"))]
    const PROTO_INCLUDES: [&str; 1] = ["../../../proto/"];
    config
        .compile_protos(&PROTO_FILES, &PROTO_INCLUDES)
        .unwrap()
}
