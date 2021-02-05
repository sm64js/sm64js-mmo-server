fn main() {
    let build_args = match std::env::var("DOCKER") {
        Ok(val) => match val.as_ref() {
            "true" | "1" => (&["./proto/mario.proto"], &["./proto/"]),
            _ => (&["./proto/mario.proto"], &["./proto/"]),
        },
        Err(_) => (&["./proto/mario.proto"], &["./proto/"]),
    };
    prost_build::compile_protos(build_args.0, build_args.1).unwrap()
}
