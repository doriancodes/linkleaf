pub mod api;
pub mod feed;
pub(crate) mod html;
pub mod linkleaf_proto {
    include!(concat!(env!("OUT_DIR"), "/linkleaf.v1.rs"));
}
