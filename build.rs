fn main() {
    tonic_build::compile_protos("./truck.proto").unwrap();
}
