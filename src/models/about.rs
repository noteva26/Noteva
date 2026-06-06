//! Public about/profile model.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AboutSocialLink {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub icon: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AboutTimelineItem {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AboutProfile {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub nav_enabled: bool,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub headline: String,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub website: String,
    #[serde(default)]
    pub social_links: Vec<AboutSocialLink>,
    #[serde(default)]
    pub timeline: Vec<AboutTimelineItem>,
    #[serde(default)]
    pub extra_markdown: String,
}

impl Default for AboutProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            nav_enabled: false,
            display_name: String::new(),
            avatar: String::new(),
            headline: String::new(),
            bio: String::new(),
            location: String::new(),
            website: String::new(),
            social_links: Vec::new(),
            timeline: Vec::new(),
            extra_markdown: String::new(),
        }
    }
}

impl AboutProfile {
    pub fn fallback(display_name: String, avatar: String, headline: String, bio: String) -> Self {
        Self {
            display_name,
            avatar,
            headline,
            bio,
            ..Self::default()
        }
    }

    pub fn normalize(mut self) -> Self {
        self.display_name = self.display_name.trim().to_string();
        self.avatar = self.avatar.trim().to_string();
        self.headline = self.headline.trim().to_string();
        self.bio = self.bio.trim().to_string();
        self.location = self.location.trim().to_string();
        self.website = self.website.trim().to_string();
        self.social_links = self
            .social_links
            .into_iter()
            .map(|mut link| {
                link.label = link.label.trim().to_string();
                link.url = link.url.trim().to_string();
                link.icon = link.icon.trim().to_string();
                link
            })
            .filter(|link| !link.label.is_empty() || !link.url.is_empty())
            .collect();
        self.timeline = self
            .timeline
            .into_iter()
            .map(|mut item| {
                item.title = item.title.trim().to_string();
                item.date = item.date.trim().to_string();
                item.description = item.description.trim().to_string();
                item
            })
            .filter(|item| {
                !item.title.is_empty() || !item.date.is_empty() || !item.description.is_empty()
            })
            .collect();
        self
    }
}
