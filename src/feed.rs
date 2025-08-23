use crate::linkleaf_proto::Feed;
use anyhow::{Context, Result};
use prost::Message;
use std::{fs, io::Write, path::PathBuf};

pub fn read_feed(path: &PathBuf) -> Result<Feed> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Feed::decode(&*bytes).with_context(|| format!("failed to decode protobuf: {}", path.display()))
}

pub fn write_feed(path: &PathBuf, feed: Feed) -> Result<Feed> {
    let mut buf = Vec::with_capacity(1024);
    feed.encode(&mut buf)?;

    let tmp = path.with_extension("pb.tmp");
    {
        let mut f =
            fs::File::create(&tmp).with_context(|| format!("failed to write {}", tmp.display()))?;
        f.write_all(&buf)?;
        f.flush()?;
    }
    fs::rename(&tmp, path)
        .with_context(|| format!("failed to move temp file into place: {}", path.display()))?;
    Ok(feed)
}
