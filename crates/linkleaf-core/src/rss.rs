use crate::linkleaf_proto::{Feed, Link};
use anyhow::Result;
use rss::{CategoryBuilder, ChannelBuilder, GuidBuilder, Item, ItemBuilder};
use time::format_description::{FormatItem, well_known::Rfc2822};
use time::{OffsetDateTime, PrimitiveDateTime};

const TS_FMT: &[FormatItem<'_>] =
    time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

// fn parse_local(s: &str) -> Option<OffsetDateTime> {
//     let naive = PrimitiveDateTime::parse(s, TS_FMT).ok()?;
//     let local_off = OffsetDateTime::now_local().ok()?.offset();
//     Some(naive.assume_offset(*local_off))
// }

// //#[cfg(feature = "rss")]
// pub fn feed_to_rss_xml(feed: &Feed, site_title: &str, site_link: &str) -> Result<String> {
//     let items: Vec<Item> = feed.links.iter().map(|l| link_to_rss_item(l)).collect();
//     let description = format!("Feed about {} generated through Linkleaf", &feed.title);

//     let channel = ChannelBuilder::default()
//         .title(if feed.title.is_empty() {
//             site_title.to_string()
//         } else {
//             feed.title.clone()
//         })
//         .link(site_link.to_string())
//         .description(description) // if you have it; else set a default
//         .items(items)
//         .build();

//     let mut buf = Vec::new();
//     channel.pretty_write_to(&mut buf, b' ', 2)?;
//     Ok(String::from_utf8(buf)?)
// }

// fn link_to_rss_item(l: &Link) -> rss::Item {
//     let pub_date = parse_local(&l.date).and_then(|dt| dt.format(&Rfc2822).ok());

//     let cats = l
//         .tags
//         .iter()
//         .map(|t| CategoryBuilder::default().name(t.clone()).build())
//         .collect::<Vec<_>>();

//     ItemBuilder::default()
//         .title(Some(l.title.clone()))
//         .link(Some(l.url.clone()))
//         .description((!l.summary.is_empty()).then(|| l.summary.clone()))
//         .categories(cats)
//         .guid(Some(
//             GuidBuilder::default()
//                 .value(format!("urn:uuid:{}", l.id))
//                 .permalink(false)
//                 .build(),
//         ))
//         .pub_date(pub_date)
//         .build()
// }
