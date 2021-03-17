fn main() {
    let mut config = prost_build::Config::new();
    config.type_attribute(".", "#[derive(serde::Serialize)]");
    config
        .compile_protos(&["../../proto/mario.proto"], &["../../proto/"])
        .unwrap()
}
