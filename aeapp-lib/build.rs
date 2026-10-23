fn main() -> std::io::Result<()> {
    prost_build::compile_protos(&["src/rtm/proto/rtm.proto"], &["src/rtm/proto/"])
}
