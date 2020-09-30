//! # Subreddit
//! A read-only module to read data from a specific subreddit.
//!
//! # Basic Usage
//! ```should_fail
//! use roux::Subreddit;
//! let subreddit = Subreddit::new("rust");
//! // Now you are able to:
//! // Get moderators.
//! let moderators = subreddit.moderators().await;
//! // Get hot posts with limit = 25.
//! let hot = subreddit.hot(25, None).await;
//! // Get rising posts with limit = 30.
//! let rising = subreddit.rising(30, None).await;
//! // Get top posts with limit = 10.
//! let top = subreddit.top(10, None).await;
//! // Get latest comments.
//! // `depth` and `limit` are optional.
//! let latest_comments = subreddit.latest_comments(None, Some(25)).await;
//! // Get comments from a submission.
//! let article_id = &hot.unwrap().data.children.first().unwrap().data.id.clone();
//! let article_comments = subreddit.article_comments(article_id, None, Some(25));
//! ```
//! # Usage with feed options
//! ```should_fail
//! use roux::Subreddit;
//! use roux::util::FeedOption;
//!
//! let subreddit = Subreddit::new("astolfo");
//!
//! // Gets top 10
//! let top = subreddit.top(10, None).await;
//!
//! // Get after param from `top`
//! let after = top.unwrap().data.after.unwrap();
//! let options = FeedOption::new().after(&after);
//!
//! // Gets next 25
//! let next_top = subreddit.top(25, Some(options)).await;
//! ```

extern crate reqwest;
extern crate serde_json;

use crate::util::{FeedOption, RouxError};
use reqwest::Client;

pub mod responses;
use responses::{Moderators, Submissions, SubredditComments, SubredditsListing};

/// Access subreddits api
pub struct Subreddits;

impl Subreddits {
    /// Search subreddits
    pub async fn search(
        name: &str,
        limit: Option<u32>,
        options: Option<FeedOption>,
    ) -> Result<SubredditsListing, RouxError> {
        let url = &mut format!("https://www.reddit.com/subreddits/search.json?q={}", name);

        if let Some(limit) = limit {
            url.push_str(&mut format!("&limit={}", limit));
        }

        if let Some(options) = options {
            options.build_url(url);
        }

        let client = Client::new();

        Ok(client
            .get(&url.to_owned())
            .send()
            .await?
            .json::<SubredditsListing>()
            .await?)
    }
}

/// Subreddit.
pub struct Subreddit {
    /// Name of subreddit.
    pub name: String,
    url: String,
    client: Client,
}

impl Subreddit {
    /// Create a new `Subreddit` instance.
    pub fn new(name: &str) -> Subreddit {
        let subreddit_url = format!("https://www.reddit.com/r/{}", name);

        Subreddit {
            name: name.to_owned(),
            url: subreddit_url,
            client: Client::new(),
        }
    }

    /// Get moderators.
    pub async fn moderators(&self) -> Result<Moderators, RouxError> {
        Ok(self
            .client
            .get(&format!("{}/about/moderators/.json", self.url))
            .send()
            .await?
            .json::<Moderators>()
            .await?)
    }

    async fn get_feed(
        &self,
        ty: &str,
        limit: u32,
        options: Option<FeedOption>,
    ) -> Result<Submissions, RouxError> {
        let url = &mut format!("{}/{}.json?limit={}", self.url, ty, limit);
        if let Some(options) = options {
            options.build_url(url);
        }

        Ok(self
            .client
            .get(&url.to_owned())
            .send()
            .await?
            .json::<Submissions>()
            .await?)
    }

    async fn get_comment_feed(
        &self,
        ty: &str,
        depth: Option<u32>,
        limit: Option<u32>,
    ) -> Result<SubredditComments, RouxError> {
        let url = &mut format!("{}/{}.json?", self.url, ty);

        if !depth.is_none() {
            url.push_str(&mut format!("&depth={}", depth.unwrap()));
        }

        if !limit.is_none() {
            url.push_str(&mut format!("&limit={}", limit.unwrap()));
        }

        // This is one of the dumbest APIs I've ever seen.
        // The comments for a subreddit are stored in a normal hash map
        // but for posts the comments are in an array with the ONLY item
        // being same hash map as the one for subreddits...
        if url.contains("comments/") {
            let mut comments = self
                .client
                .get(&url.to_owned())
                .send()
                .await?
                .json::<Vec<SubredditComments>>()
                .await?;

            Ok(comments.pop().unwrap())
        } else {
            Ok(self
                .client
                .get(&url.to_owned())
                .send()
                .await?
                .json::<SubredditComments>()
                .await?)
        }
    }

    /// Get hot posts.
    pub async fn hot(
        &self,
        limit: u32,
        options: Option<FeedOption>,
    ) -> Result<Submissions, RouxError> {
        self.get_feed("hot", limit, options).await
    }

    /// Get rising posts.
    pub async fn rising(
        &self,
        limit: u32,
        options: Option<FeedOption>,
    ) -> Result<Submissions, RouxError> {
        self.get_feed("rising", limit, options).await
    }

    /// Get top posts.
    pub async fn top(
        &self,
        limit: u32,
        options: Option<FeedOption>,
    ) -> Result<Submissions, RouxError> {
        // TODO: time filter
        self.get_feed("top", limit, options).await
    }

    /// Get latest posts.
    pub async fn latest(
        &self,
        limit: u32,
        options: Option<FeedOption>,
    ) -> Result<Submissions, RouxError> {
        self.get_feed("new", limit, options).await
    }

    /// Get latest comments.
    pub async fn latest_comments(
        &self,
        depth: Option<u32>,
        limit: Option<u32>,
    ) -> Result<SubredditComments, RouxError> {
        self.get_comment_feed("comments", depth, limit).await
    }

    /// Get comments from article.
    pub async fn article_comments(
        &self,
        article: &str,
        depth: Option<u32>,
        limit: Option<u32>,
    ) -> Result<SubredditComments, RouxError> {
        self.get_comment_feed(&format!("comments/{}", article), depth, limit)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::Subreddit;
    use super::Subreddits;
    use tokio;

    #[tokio::test]
    async fn test_no_auth() {
        let subreddit = Subreddit::new("astolfo");

        // Test moderators
        let moderators = subreddit.moderators().await;
        assert!(moderators.is_ok());

        let subreddits_limit = 3u32;
        let subreddits = Subreddits::search("rust", Some(subreddits_limit), None).await;
        assert!(subreddits.is_ok());
        assert!(subreddits.unwrap().data.children.len() == subreddits_limit as usize);

        // Test feeds
        let hot = subreddit.hot(25, None).await;
        assert!(hot.is_ok());

        let rising = subreddit.rising(25, None).await;
        assert!(rising.is_ok());

        let top = subreddit.top(25, None).await;
        assert!(top.is_ok());

        let latest_comments = subreddit.latest_comments(None, Some(25)).await;
        assert!(latest_comments.is_ok());

        let article_id = &hot.unwrap().data.children.first().unwrap().data.id.clone();
        let article_comments = subreddit.article_comments(article_id, None, Some(25)).await;
        assert!(article_comments.is_ok());
    }
}
