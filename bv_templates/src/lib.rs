use askama::Template;

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum ContentType {
    Text,
    Image,
    Video,
    Archive,
    // TextPlain,
    // ImagePNG,
    // ImageJPEG,
    // ImageWEBP,
    // ImageAVIF,
    // VideoMP4,
    // VideoVP9,
    // VideoVP8,
    // VideoMKV,
    // VideoMOV,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct Item {
    pub link: String,
    pub path: String,
    pub content_type: ContentType,
}

pub struct Bucket {
    pub name: String,
    pub value: String,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct Browse {
    pub title: String,
    pub bucket: String,
    pub prefix_str: String,
    pub items: Vec<Item>,
    pub offset: u64,
    pub prev_offset: u64,
    // pub prev_idx: Option<String>,
    // pub next_idx: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_template() {
        let view = Browse {
            bucket: "excelsior".into(),
            title: "test".into(),
            prefix_str: "some/prefix".into(),
            items: [Item {
                link: "https://localhost/some/prefix.jpeg".into(),
                path: "prefix".into(),
                content_type: ContentType::Image,
            }]
            .into(),
            offset: 0,
            prev_offset: 0,
            // prev_idx: None,
            // next_idx: None,
        };
        assert!(view.render().is_ok());
    }
}
