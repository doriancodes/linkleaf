pub mod command;
pub mod feed;
pub mod html;
pub mod linkleaf_proto {
    include!(concat!(env!("OUT_DIR"), "/linkleaf.v1.rs"));
}
