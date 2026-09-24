use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;

use crate::error::Result;
use crate::tmdb::types::*;

struct XmlBuilder {
    writer: Writer<Cursor<Vec<u8>>>,
}

impl XmlBuilder {
    fn new() -> Self {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
        let _ = writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))));
        Self { writer }
    }

    fn tag(&mut self, name: &str, value: &str) {
        if value.is_empty() {
            return;
        }
        let start = BytesStart::new(name);
        let _ = self.writer.write_event(Event::Start(start));
        let _ = self.writer.write_event(Event::Text(BytesText::new(value)));
        let _ = self.writer.write_event(Event::End(BytesEnd::new(name)));
    }

    fn tag_opt(&mut self, name: &str, value: &Option<String>) {
        if let Some(v) = value {
            self.tag(name, v);
        }
    }

    fn open(&mut self, name: &str) -> &mut Self {
        let start = BytesStart::new(name);
        let _ = self.writer.write_event(Event::Start(start));
        self
    }

    fn open_with_attrs(&mut self, name: &str, attrs: &[(&str, &str)]) -> &mut Self {
        let mut start = BytesStart::new(name);
        for (k, v) in attrs {
            start.push_attribute((*k, *v));
        }
        let _ = self.writer.write_event(Event::Start(start));
        self
    }

    fn close(&mut self, name: &str) -> &mut Self {
        let _ = self.writer.write_event(Event::End(BytesEnd::new(name)));
        self
    }

    fn text(&mut self, value: &str) -> &mut Self {
        let _ = self.writer.write_event(Event::Text(BytesText::new(value)));
        self
    }

    fn finish(self) -> String {
        String::from_utf8(self.writer.into_inner().into_inner()).unwrap_or_default()
    }
}

fn write_uniqueid(xml: &mut XmlBuilder, tmdb_id: u64, imdb_id: &Option<String>) {
    xml.open_with_attrs("uniqueid", &[("type", "tmdb"), ("default", "true")])
        .text(&tmdb_id.to_string())
        .close("uniqueid");
    if let Some(imdb) = imdb_id {
        if !imdb.is_empty() {
            xml.open_with_attrs("uniqueid", &[("type", "imdb")])
                .text(imdb)
                .close("uniqueid");
        }
    }
}

fn write_ratings(xml: &mut XmlBuilder, vote_average: Option<f64>, vote_count: Option<u64>) {
    xml.open("ratings");
    if let Some(avg) = vote_average {
        xml.open_with_attrs("rating", &[("name", "tmdb"), ("default", "true"), ("max", "10")]);
        xml.tag("value", &format!("{:.1}", avg));
        if let Some(count) = vote_count {
            xml.tag("votes", &count.to_string());
        }
        xml.close("rating");
    }
    xml.close("ratings");
}

fn write_actors(xml: &mut XmlBuilder, credits: &Option<Credits>, image_base_url: &str) {
    if let Some(credits) = credits {
        for member in &credits.cast {
            xml.open("actor");
            xml.tag("name", &member.name);
            if let Some(character) = &member.character {
                xml.tag("role", character);
            }
            if let Some(profile_path) = &member.profile_path {
                xml.tag("thumb", &format!("{}h632{}", image_base_url, profile_path));
            }
            xml.tag("profile", &member.profile());
            xml.tag("tmdbid", &member.id.to_string());
            if let Some(order) = member.order {
                xml.tag("order", &order.to_string());
            }
            xml.close("actor");
        }
    }
}

fn write_crew(xml: &mut XmlBuilder, credits: &Option<Credits>) {
    if let Some(credits) = credits {
        for crew in &credits.crew {
            let tmdbid = crew.id.to_string();
            match crew.job.as_str() {
                "Director" => {
                    xml.open_with_attrs("director", &[("tmdbid", &tmdbid)])
                        .text(&crew.name)
                        .close("director");
                }
                "Writer" | "Screenplay" | "Story" => {
                    xml.open_with_attrs("credits", &[("tmdbid", &tmdbid)])
                        .text(&crew.name)
                        .close("credits");
                }
                "Producer" | "Executive Producer" => xml.tag("producer", &crew.name),
                _ => {}
            }
        }
    }
}

fn write_trailer(xml: &mut XmlBuilder, videos: &Option<VideoResults>) {
    if let Some(videos) = videos {
        for video in &videos.results {
            if video.site == "YouTube"
                && (video.video_type == "Trailer" || video.video_type == "Teaser")
            {
                xml.tag(
                    "trailer",
                    &format!(
                        "plugin://plugin.video.youtube/?action=play_video&videoid={}",
                        video.key
                    ),
                );
                return;
            }
        }
    }
}

pub fn generate_movie_nfo(
    movie: &MovieDetail,
    collection: Option<&Collection>,
    image_base_url: &str,
) -> Result<String> {
    let mut xml = XmlBuilder::new();
    xml.open("movie");

    xml.tag_opt("title", &movie.title);
    xml.tag_opt("originaltitle", &movie.original_title);
    xml.tag_opt("tagline", &movie.tagline);

    // outline: short synopsis (use tagline or first sentence of overview)
    if let Some(plot) = &movie.overview {
        let outline = truncate_outline(plot);
        if !outline.is_empty() {
            xml.tag("outline", &outline);
        }
    }

    xml.tag_opt("plot", &movie.overview);

    write_uniqueid(&mut xml, movie.id, &movie.imdb_id);

    // <id> (IMDB ID for legacy compatibility)
    if let Some(imdb) = &movie.imdb_id {
        if !imdb.is_empty() {
            xml.tag("id", imdb);
        }
    }
    // <tmdbid>
    xml.tag("tmdbid", &movie.id.to_string());

    write_ratings(&mut xml, movie.vote_average, movie.vote_count);

    if let Some(date) = &movie.release_date {
        if date.len() >= 4 {
            xml.tag("year", &date[..4]);
        }
        xml.tag("premiered", date);
    }

    if let Some(runtime) = movie.runtime {
        xml.tag("runtime", &runtime.to_string());
    }

    // Certification from release_dates
    let cert = movie.release_dates.as_ref().and_then(|rd| {
        rd.results
            .iter()
            .find(|r| r.iso_3166_1 == "US")
            .or_else(|| rd.results.first())
            .and_then(|r| {
                r.release_dates.iter().find(|d| {
                    d.certification
                        .as_ref()
                        .map(|c| !c.is_empty())
                        .unwrap_or(false)
                })
            })
            .and_then(|d| d.certification.clone())
    });
    if let Some(c) = &cert {
        xml.tag("mpaa", c);
        xml.tag("certification", c);
    }

    for genre in &movie.genres {
        xml.tag("genre", &genre.name);
    }

    for country in &movie.production_countries {
        xml.tag("country", &country.name);
    }

    for lang in &movie.spoken_languages {
        xml.tag(
            "language",
            &lang.english_name.clone().unwrap_or(lang.name.clone()),
        );
    }

    for company in &movie.production_companies {
        xml.tag("studio", &company.name);
    }

    if let Some(coll) = collection {
        xml.open("set");
        xml.tag("name", &coll.name);
        if let Some(overview) = &coll.overview {
            xml.tag("overview", overview);
        }
        xml.tag("tmdbid", &coll.id.to_string());
        xml.close("set");
    } else if let Some(coll_ref) = &movie.belongs_to_collection {
        xml.open("set");
        xml.tag("name", &coll_ref.name);
        xml.tag("tmdbid", &coll_ref.id.to_string());
        xml.close("set");
    }

    write_crew(&mut xml, &movie.credits);
    write_actors(&mut xml, &movie.credits, image_base_url);

    // dateadded (current time)
    let now = now_iso8601();
    xml.tag("dateadded", &now);

    // trailer
    write_trailer(&mut xml, &movie.videos);

    if let Some(poster) = &movie.poster_path {
        xml.open_with_attrs("thumb", &[("aspect", "poster")])
            .text(&format!("{}original{}", image_base_url, poster))
            .close("thumb");
    }
    if let Some(backdrop) = &movie.backdrop_path {
        xml.open("fanart");
        xml.open("thumb")
            .text(&format!("{}original{}", image_base_url, backdrop))
            .close("thumb");
        xml.close("fanart");
    }

    xml.close("movie");
    Ok(xml.finish())
}

fn now_iso8601() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Simple UTC datetime formatting without chrono
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let h = time_secs / 3600;
    let m = (time_secs % 3600) / 60;
    let s = time_secs % 60;

    // Days since 1970-01-01
    let (year, month, day) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, h, m, s)
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut y = 1970;
    loop {
        let dy = if is_leap(y) { 366 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        y += 1;
    }
    let leap = is_leap(y);
    let months: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut m = 0;
    for &ml in &months {
        if days < ml {
            break;
        }
        days -= ml;
        m += 1;
    }
    (y, m + 1, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Truncate plot to the first one or two sentences for <outline>.
fn truncate_outline(plot: &str) -> String {
    if plot.is_empty() {
        return String::new();
    }
    // Try Chinese period first, then English
    let mut end = None;
    for (i, c) in plot.char_indices() {
        if c == '。' || c == '.' {
            let pos = i + c.len_utf8();
            // Check if we have a second sentence boundary
            if end.is_some() {
                // Two sentences found, stop here
                return plot[..pos].to_string();
            }
            end = Some(pos);
        }
    }
    // If we found at least one sentence, use it
    if let Some(pos) = end {
        return plot[..pos].to_string();
    }
    // No sentence boundary found, return as-is (capped at ~200 chars)
    let cap = plot
        .char_indices()
        .nth(200)
        .map(|(i, _)| i)
        .unwrap_or(plot.len());
    plot[..cap].to_string()
}

pub fn generate_tvshow_nfo(show: &TvShowDetail, _api_key: &str, image_base_url: &str) -> Result<String> {
    let mut xml = XmlBuilder::new();
    xml.open("tvshow");

    xml.tag_opt("title", &show.name);
    xml.tag_opt("originaltitle", &show.original_name);
    xml.tag("showtitle", show.name.as_deref().unwrap_or(""));
    xml.tag_opt("plot", &show.overview);

    if let Some(date) = &show.first_air_date {
        if date.len() >= 4 {
            xml.tag("year", &date[..4]);
        }
    }

    write_ratings(&mut xml, show.vote_average, show.vote_count);

    // poster thumb
    if let Some(poster) = &show.poster_path {
        xml.open_with_attrs("thumb", &[("aspect", "poster")])
            .text(&format!("{}original{}", image_base_url, poster))
            .close("thumb");
    }

    // namedseason — include season 0 (specials)
    for season in &show.seasons {
        if let Some(name) = &season.name {
            xml.open_with_attrs(
                "namedseason",
                &[("number", &season.season_number.to_string())],
            )
            .text(name)
            .close("namedseason");
        }
    }

    // Per-season poster thumbs
    for season in &show.seasons {
        if let Some(poster) = &season.poster_path {
            let sn = season.season_number.to_string();
            xml.open_with_attrs(
                "thumb",
                &[("aspect", "poster"), ("season", &sn), ("type", "season")],
            )
            .text(&format!("{}original{}", image_base_url, poster))
            .close("thumb");
        }
    }

    // fanart
    if let Some(backdrop) = &show.backdrop_path {
        xml.open("fanart");
        xml.open("thumb")
            .text(&format!("{}original{}", image_base_url, backdrop))
            .close("thumb");
        xml.close("fanart");
    }

    // certification
    let cert = show.content_ratings.as_ref().and_then(|cr| {
        cr.results
            .iter()
            .find(|r| r.iso_3166_1 == "US")
            .or_else(|| cr.results.first())
            .map(|r| r.rating.clone())
    });
    if let Some(c) = &cert {
        xml.tag("mpaa", c);
        xml.tag("certification", c);
    }

    // episodeguide
    xml.open("episodeguide");
    let lang_short = show.original_language.as_deref().unwrap_or("en");
    // NOTE: Do not include api_key in URL to avoid key exposure in NFO files
    // that may be synced to media libraries, cloud storage, or backups.
    // Players should configure their own TMDB API key separately.
    xml.tag(
        "url",
        &format!(
            "http://api.themoviedb.org/3/tv/{}?language={}",
            show.id, lang_short
        ),
    );
    xml.close("episodeguide");

    // IDs
    let imdb_id = show.external_ids.as_ref().and_then(|e| e.imdb_id.clone());
    let tvdb_id = show.external_ids.as_ref().and_then(|e| e.tvdb_id);

    if let Some(tvdb) = tvdb_id {
        xml.tag("id", &tvdb.to_string());
    }
    if let Some(imdb) = &imdb_id {
        if !imdb.is_empty() {
            xml.tag("imdbid", imdb);
        }
    }
    xml.tag("tmdbid", &show.id.to_string());

    write_uniqueid(&mut xml, show.id, &imdb_id);

    if let Some(tvdb) = tvdb_id {
        xml.open_with_attrs("uniqueid", &[("type", "tvdb")])
            .text(&tvdb.to_string())
            .close("uniqueid");
    }

    if let Some(date) = &show.first_air_date {
        xml.tag("premiered", date);
    }

    xml.tag_opt("status", &show.status);

    if !show.episode_run_time.is_empty() {
        xml.tag("runtime", &show.episode_run_time[0].to_string());
    }

    for genre in &show.genres {
        xml.tag("genre", &genre.name);
    }

    // studios: networks + production companies
    for net in &show.networks {
        xml.tag("studio", &net.name);
    }
    for company in &show.production_companies {
        xml.tag("studio", &company.name);
    }

    for country in &show.origin_country {
        xml.tag("country", country);
    }

    // tags from keywords
    if let Some(kw) = &show.keywords {
        for keyword in &kw.results {
            xml.tag("tag", &keyword.name);
        }
    }

    write_crew(&mut xml, &show.credits);
    write_actors(&mut xml, &show.credits, image_base_url);

    // trailer
    write_trailer(&mut xml, &show.videos);

    // dateadded
    xml.tag("dateadded", &now_iso8601());

    xml.close("tvshow");
    Ok(xml.finish())
}

pub fn generate_episode_nfo(
    episode: &EpisodeSummary,
    show: &TvShowDetail,
    original_title: Option<&str>,
    image_base_url: &str,
) -> Result<String> {
    let mut xml = XmlBuilder::new();
    xml.open("episodedetails");

    xml.tag_opt("title", &episode.name);
    // originaltitle: use English name if available, fall back to episode.name
    let orig = original_title
        .map(|s| s.to_string())
        .or_else(|| episode.name.clone());
    xml.tag_opt("originaltitle", &orig);
    xml.tag("showtitle", show.name.as_deref().unwrap_or(""));

    xml.tag("season", &episode.season_number.to_string());
    xml.tag("episode", &episode.episode_number.to_string());
    xml.tag("displayseason", &episode.season_number.to_string());
    xml.tag("displayepisode", &episode.episode_number.to_string());

    // IDs
    xml.tag("id", &episode.id.to_string());
    xml.open_with_attrs("uniqueid", &[("type", "tmdb"), ("default", "true")])
        .text(&episode.id.to_string())
        .close("uniqueid");

    write_ratings(&mut xml, episode.vote_average, episode.vote_count);

    xml.tag_opt("plot", &episode.overview);

    if let Some(runtime) = episode.runtime {
        xml.tag("runtime", &runtime.to_string());
    }

    // thumb (episode still)
    if let Some(still) = &episode.still_path {
        xml.tag("thumb", &format!("{}original{}", image_base_url, still));
    }

    // premiered & aired
    if let Some(date) = &episode.air_date {
        xml.tag("premiered", date);
        xml.tag("aired", date);
    }

    // studios from show
    for net in &show.networks {
        xml.tag("studio", &net.name);
    }
    for company in &show.production_companies {
        xml.tag("studio", &company.name);
    }

    // Episode crew (directors, writers) with tmdbid
    if let Some(crew) = &episode.crew {
        for c in crew {
            let tmdbid = c.id.to_string();
            match c.job.as_str() {
                "Director" => {
                    xml.open_with_attrs("director", &[("tmdbid", &tmdbid)])
                        .text(&c.name)
                        .close("director");
                }
                "Writer" | "Screenplay" | "Story" => {
                    xml.open_with_attrs("credits", &[("tmdbid", &tmdbid)])
                        .text(&c.name)
                        .close("credits");
                }
                _ => {}
            }
        }
    }

    // Show's main cast first
    write_actors(&mut xml, &show.credits, image_base_url);

    // Then episode guest stars
    if let Some(guests) = &episode.guest_stars {
        for guest in guests {
            xml.open("actor");
            xml.tag("name", &guest.name);
            if let Some(character) = &guest.character {
                xml.tag("role", character);
            }
            if let Some(profile_path) = &guest.profile_path {
                xml.tag("thumb", &format!("{}h632{}", image_base_url, profile_path));
            }
            xml.tag("profile", &guest.profile());
            xml.tag("tmdbid", &guest.id.to_string());
            xml.close("actor");
        }
    }

    xml.tag("dateadded", &now_iso8601());

    xml.close("episodedetails");
    Ok(xml.finish())
}
