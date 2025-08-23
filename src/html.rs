use askama::Template;
use serde::Serialize;

#[derive(serde::Serialize)]
pub struct LinkView {
    pub title: String,
    pub url: String,
    pub date: String,
    pub summary: String,
    pub via: String,
    pub has_tags: bool,
    pub tags_joined: String,
}

#[derive(Serialize)]
pub struct FeedView {
    pub title: String,
    pub count: usize,
    pub links: Vec<LinkView>,
}

#[derive(Template)]
#[template(path = "feed.html", escape = "html")]
pub struct FeedPage<'a> {
    pub feed: &'a FeedView,
}
