fn main() {
    prost_build::compile_protos(&["debug_control.proto"], &["."]).unwrap();
}
