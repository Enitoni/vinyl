use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum Input {
    YouTube(YouTubeVideo),
}

impl Input {
    /// Returns the fingerprint used to check
    /// if this is already in cache
    pub fn fingerprint(&self) -> String {
        match self {
            Input::YouTube(v) => v.fingerprint(),
        }
    }

    pub fn parse(str: &str) -> Option<Self> {
        let predicates = [|url| YouTubeVideo::from_url(url).map(Self::YouTube)];

        predicates.into_iter().find_map(|f| f(str))
    }
}

impl Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Input::YouTube(x) => x.fmt(f),
        }
    }
}

impl From<&Input> for String {
    fn from(x: &Input) -> Self {
        x.to_string()
    }
}

pub use youtube::YouTubeVideo;
mod youtube {
    use std::fmt::Display;

    use log::error;
    use youtube_dl::{YoutubeDl, YoutubeDlOutput};

    /// Parsed from youtube-dl
    #[derive(Debug, Clone)]
    pub struct YouTubeVideo {
        id: String,
        title: String,
        channel: String,
        audio_stream_url: String,
    }

    impl YouTubeVideo {
        pub fn fingerprint(&self) -> String {
            self.title.to_owned()
        }

        pub fn from_url(url: &str) -> Option<Self> {
            if !Self::is_valid_url(url) {
                return None;
            }

            parse_from_url(url)
        }

        /// Returns true if this is a valid YouTube video url
        fn is_valid_url(url: &str) -> bool {
            // Remove protocol if any
            let rest = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);

            // Remove www if any
            let rest = rest
                .split_once("www.")
                .map(|(_, rest)| rest)
                .unwrap_or(rest);

            let mut split = rest.split('/');
            let domain = split.next();
            let path = split.next();

            domain
                .zip(path)
                .map(|(domain, path)| {
                    domain == "youtube.com" && path.starts_with("watch?v=") || domain == "youtu.be"
                })
                .unwrap_or_default()
        }
    }

    impl Display for YouTubeVideo {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} by {}", self.title, self.channel)
        }
    }

    /// Tries to fetch the video via youtube-dl, returning None if important
    /// fields are missing or the fetch failed.
    pub fn parse_from_url(url: &str) -> Option<YouTubeVideo> {
        let output = YoutubeDl::new(url)
            .socket_timeout("15")
            .extra_arg("-f")
            .extra_arg("bestaudio")
            .run();

        output
            .map_err(|err| {
                error!("Failed to fetch YouTube video: {}", err.to_string());
            })
            .ok()
            .and_then(|o| match o {
                YoutubeDlOutput::SingleVideo(video) => Some(video),
                YoutubeDlOutput::Playlist(_) => None,
            })
            .and_then(|video| {
                let id = video.id;
                let title = video.title;
                let channel = video.channel.unwrap_or_else(|| "Unknown".to_string());

                let format_name = video.format.as_ref();
                let format = video.formats.and_then(|formats| {
                    formats
                        .into_iter()
                        .find(|f| f.format.as_ref() == format_name)
                });

                format
                    .and_then(|format| format.url)
                    .map(|audio_stream_url| YouTubeVideo {
                        id,
                        title,
                        channel,
                        audio_stream_url,
                    })
            })
    }

    #[cfg(test)]
    mod test {
        use super::YouTubeVideo;

        #[test]
        fn test_url() {
            assert!(YouTubeVideo::is_valid_url(
                "https://www.youtube.com/watch?v=RiZ_5jo9WBg"
            ));
            assert!(YouTubeVideo::is_valid_url(
                "https://youtube.com/watch?v=RiZ_5jo9WBg"
            ));
            assert!(YouTubeVideo::is_valid_url(
                "youtube.com/watch?v=RiZ_5jo9WBg"
            ));

            assert!(!YouTubeVideo::is_valid_url(
                "yourtube.com/watch?v=RiZ_5jo9WBg"
            ));
            assert!(!YouTubeVideo::is_valid_url("https://google.com"));
            assert!(!YouTubeVideo::is_valid_url("kpofkagt"));
        }
    }
}