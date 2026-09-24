//! Emby/Kodi NFO generation from TMDB data.

mod xml;

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::tmdb::images;
use crate::tmdb::types::{
    CastMember, Collection, CrewMember, Episode, MovieDetail, Named, TvShowDetail, VideoResults,
};
use xml::Xml;

const ACTOR_THUMB_SIZE: &str = "h632";
const ART_SIZE: &str = "original";

pub fn movie(movie: &MovieDetail, collection: Option<&Collection>) -> String {
    Xml::document("movie", |x| {
        x.opt("title", movie.title.as_deref())
            .opt("originaltitle", movie.original_title.as_deref())
            .opt("tagline", movie.tagline.as_deref());
        if let Some(plot) = &movie.overview {
            x.text("outline", outline(plot));
        }
        x.opt("plot", movie.overview.as_deref());

        let imdb = non_empty(movie.imdb_id.as_deref());
        unique_ids(x, movie.id, imdb);
        x.opt("id", imdb).text("tmdbid", &movie.id.to_string());

        ratings(x, movie.vote_average, movie.vote_count);
        if let Some(date) = &movie.release_date {
            x.opt("year", year(date)).text("premiered", date);
        }
        if let Some(runtime) = movie.runtime {
            x.text("runtime", &runtime.to_string());
        }
        certification(x, movie_certification(movie));

        names(x, "genre", &movie.genres);
        names(x, "country", &movie.production_countries);
        for lang in &movie.spoken_languages {
            x.text(
                "language",
                lang.english_name.as_deref().unwrap_or(&lang.name),
            );
        }
        names(x, "studio", &movie.production_companies);
        if let Some(keywords) = &movie.keywords {
            names(x, "tag", &keywords.keywords);
        }

        if let Some(coll) = collection {
            x.block("set", &[], |x| {
                x.text("name", &coll.name)
                    .opt("overview", coll.overview.as_deref())
                    .text("tmdbid", &coll.id.to_string());
            });
        } else if let Some(coll) = &movie.belongs_to_collection {
            x.block("set", &[], |x| {
                x.text("name", &coll.name)
                    .text("tmdbid", &coll.id.to_string());
            });
        }

        if let Some(credits) = &movie.credits {
            crew(x, &credits.crew, true);
            for member in &credits.cast {
                actor(x, member, true);
            }
        }
        x.text("dateadded", &now_utc());
        trailer(x, movie.videos.as_ref());
        artwork(
            x,
            movie.poster_path.as_deref(),
            movie.backdrop_path.as_deref(),
        );
    })
}

pub fn tvshow(show: &TvShowDetail) -> String {
    Xml::document("tvshow", |x| {
        x.opt("title", show.name.as_deref())
            .opt("originaltitle", show.original_name.as_deref())
            .opt("showtitle", show.name.as_deref())
            .opt("plot", show.overview.as_deref())
            .opt("year", show.first_air_date.as_deref().and_then(year));
        ratings(x, show.vote_average, show.vote_count);

        poster(x, show.poster_path.as_deref());
        for season in &show.seasons {
            if let Some(name) = &season.name {
                x.text_with(
                    "namedseason",
                    &[("number", &season.season_number.to_string())],
                    name,
                );
            }
        }
        for season in &show.seasons {
            if let Some(path) = &season.poster_path {
                let number = season.season_number.to_string();
                x.text_with(
                    "thumb",
                    &[
                        ("aspect", "poster"),
                        ("season", &number),
                        ("type", "season"),
                    ],
                    &images::url(ART_SIZE, path),
                );
            }
        }
        fanart(x, show.backdrop_path.as_deref());

        let rating = show.content_ratings.as_ref().and_then(|cr| {
            pick_us_first(
                &cr.results,
                |r| r.iso_3166_1 == "US",
                |r| non_empty(Some(&r.rating)),
            )
        });
        certification(x, rating);

        // The API key is deliberately left out: NFO files end up in libraries,
        // backups and cloud sync where it would leak.
        let lang = show.original_language.as_deref().unwrap_or("en");
        x.block("episodeguide", &[], |x| {
            x.text(
                "url",
                &format!("http://api.themoviedb.org/3/tv/{}?language={lang}", show.id),
            );
        });

        let ids = show.external_ids.as_ref();
        let imdb = non_empty(ids.and_then(|e| e.imdb_id.as_deref()));
        let tvdb = ids.and_then(|e| e.tvdb_id).map(|id| id.to_string());
        x.opt("id", tvdb.as_deref())
            .opt("imdbid", imdb)
            .text("tmdbid", &show.id.to_string());
        unique_ids(x, show.id, imdb);
        if let Some(tvdb) = &tvdb {
            x.text_with("uniqueid", &[("type", "tvdb")], tvdb);
        }

        x.opt("premiered", show.first_air_date.as_deref())
            .opt("status", show.status.as_deref());
        if let Some(runtime) = show.episode_run_time.first() {
            x.text("runtime", &runtime.to_string());
        }
        names(x, "genre", &show.genres);
        studios(x, show);
        for country in &show.origin_country {
            x.text("country", country);
        }
        if let Some(keywords) = &show.keywords {
            names(x, "tag", &keywords.results);
        }

        if let Some(credits) = &show.credits {
            crew(x, &credits.crew, true);
            for member in &credits.cast {
                actor(x, member, true);
            }
        }
        trailer(x, show.videos.as_ref());
        x.text("dateadded", &now_utc());
    })
}

/// `original_title` is the episode title in the fallback language, when known.
pub fn episode(episode: &Episode, show: &TvShowDetail, original_title: Option<&str>) -> String {
    Xml::document("episodedetails", |x| {
        let season = episode.season_number.to_string();
        let number = episode.episode_number.to_string();
        let id = episode.id.to_string();

        x.opt("title", episode.name.as_deref())
            .opt("originaltitle", original_title.or(episode.name.as_deref()))
            .opt("showtitle", show.name.as_deref())
            .text("season", &season)
            .text("episode", &number)
            .text("displayseason", &season)
            .text("displayepisode", &number)
            .text("id", &id)
            .text_with("uniqueid", &[("type", "tmdb"), ("default", "true")], &id);
        ratings(x, episode.vote_average, episode.vote_count);
        x.opt("plot", episode.overview.as_deref());
        if let Some(runtime) = episode.runtime {
            x.text("runtime", &runtime.to_string());
        }
        if let Some(still) = &episode.still_path {
            x.text("thumb", &images::url(ART_SIZE, still));
        }
        if let Some(date) = &episode.air_date {
            x.text("premiered", date).text("aired", date);
        }
        studios(x, show);

        crew(x, &episode.crew, false);
        if let Some(credits) = &show.credits {
            for member in &credits.cast {
                actor(x, member, true);
            }
        }
        for guest in &episode.guest_stars {
            actor(x, guest, false);
        }
        x.text("dateadded", &now_utc());
    })
}

fn unique_ids(x: &mut Xml, tmdb_id: u64, imdb_id: Option<&str>) {
    x.text_with(
        "uniqueid",
        &[("type", "tmdb"), ("default", "true")],
        &tmdb_id.to_string(),
    );
    if let Some(imdb) = imdb_id {
        x.text_with("uniqueid", &[("type", "imdb")], imdb);
    }
}

fn ratings(x: &mut Xml, average: Option<f64>, votes: Option<u64>) {
    let Some(average) = average else { return };
    x.block("ratings", &[], |x| {
        x.block(
            "rating",
            &[("name", "tmdb"), ("default", "true"), ("max", "10")],
            |x| {
                x.text("value", &format!("{average:.1}"));
                if let Some(votes) = votes {
                    x.text("votes", &votes.to_string());
                }
            },
        );
    });
}

fn certification(x: &mut Xml, cert: Option<&str>) {
    if let Some(cert) = cert {
        x.text("mpaa", cert).text("certification", cert);
    }
}

/// US certification if available, otherwise the first country that has one.
fn movie_certification(movie: &MovieDetail) -> Option<&str> {
    let countries = &movie.release_dates.as_ref()?.results;
    pick_us_first(
        countries,
        |c| c.iso_3166_1 == "US",
        |c| {
            c.release_dates
                .iter()
                .find_map(|d| non_empty(d.certification.as_deref()))
        },
    )
}

fn pick_us_first<'a, T>(
    items: &'a [T],
    is_us: impl Fn(&T) -> bool,
    value: impl Fn(&'a T) -> Option<&'a str>,
) -> Option<&'a str> {
    items
        .iter()
        .find(|i| is_us(i))
        .and_then(&value)
        .or_else(|| items.iter().find_map(&value))
}

fn names(x: &mut Xml, tag: &str, items: &[Named]) {
    for item in items {
        x.text(tag, &item.name);
    }
}

fn studios(x: &mut Xml, show: &TvShowDetail) {
    names(x, "studio", &show.networks);
    names(x, "studio", &show.production_companies);
}

fn crew(x: &mut Xml, crew: &[CrewMember], include_producers: bool) {
    let mut seen = HashSet::new();
    for member in crew {
        let tag = match member.job.as_str() {
            "Director" => "director",
            "Writer" | "Screenplay" | "Story" | "Teleplay" => "credits",
            "Producer" | "Executive Producer" if include_producers => "producer",
            _ => continue,
        };
        if !seen.insert((tag, member.id)) {
            continue;
        }
        if tag == "producer" {
            x.text(tag, &member.name);
        } else {
            x.text_with(tag, &[("tmdbid", &member.id.to_string())], &member.name);
        }
    }
}

fn actor(x: &mut Xml, member: &CastMember, with_order: bool) {
    x.block("actor", &[], |x| {
        x.text("name", &member.name)
            .opt("role", member.character.as_deref());
        if let Some(path) = &member.profile_path {
            x.text("thumb", &images::url(ACTOR_THUMB_SIZE, path));
        }
        x.text(
            "profile",
            &format!("https://www.themoviedb.org/person/{}", member.id),
        )
        .text("tmdbid", &member.id.to_string());
        if with_order && let Some(order) = member.order {
            x.text("order", &order.to_string());
        }
    });
}

fn trailer(x: &mut Xml, videos: Option<&VideoResults>) {
    let youtube = videos
        .into_iter()
        .flat_map(|v| &v.results)
        .find(|v| v.site == "YouTube" && matches!(v.kind.as_str(), "Trailer" | "Teaser"));
    if let Some(video) = youtube {
        x.text(
            "trailer",
            &format!(
                "plugin://plugin.video.youtube/?action=play_video&videoid={}",
                video.key
            ),
        );
    }
}

fn artwork(x: &mut Xml, poster_path: Option<&str>, backdrop_path: Option<&str>) {
    poster(x, poster_path);
    fanart(x, backdrop_path);
}

fn poster(x: &mut Xml, path: Option<&str>) {
    if let Some(path) = path {
        x.text_with(
            "thumb",
            &[("aspect", "poster")],
            &images::url(ART_SIZE, path),
        );
    }
}

fn fanart(x: &mut Xml, path: Option<&str>) {
    if let Some(path) = path {
        x.block("fanart", &[], |x| {
            x.text("thumb", &images::url(ART_SIZE, path));
        });
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|s| !s.is_empty())
}

fn year(date: &str) -> Option<&str> {
    date.get(..4)
        .filter(|y| y.bytes().all(|b| b.is_ascii_digit()))
}

/// Short synopsis for `<outline>`: the first two sentences of the plot, or
/// the first 200 characters if it has no sentence boundary.
fn outline(plot: &str) -> &str {
    let mut first_end = None;
    let mut chars = plot.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let boundary = match c {
            '。' | '！' | '？' | '!' | '?' => true,
            '.' => chars.peek().is_none_or(|&(_, next)| next.is_whitespace()),
            _ => false,
        };
        if boundary {
            let end = i + c.len_utf8();
            if first_end.is_some() {
                return &plot[..end];
            }
            first_end = Some(end);
        }
    }
    if let Some(end) = first_end {
        return &plot[..end];
    }
    let cap = plot.char_indices().nth(200).map_or(plot.len(), |(i, _)| i);
    &plot[..cap]
}

/// Current UTC time as `YYYY-MM-DD HH:MM:SS`.
fn now_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format_utc(secs)
}

fn format_utc(secs: u64) -> String {
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 → (year, month, day), per Howard Hinnant's algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_utc_dates() {
        assert_eq!(format_utc(0), "1970-01-01 00:00:00");
        assert_eq!(format_utc(19_782 * 86_400 + 3661), "2024-02-29 01:01:01");
        assert_eq!(format_utc(951_782_400), "2000-02-29 00:00:00");
        assert_eq!(format_utc(1_735_689_599), "2024-12-31 23:59:59");
    }

    #[test]
    fn outlines_first_two_sentences() {
        assert_eq!(outline("一句。两句。三句。"), "一句。两句。");
        assert_eq!(
            outline("It costs 3.5 dollars. Then more. End."),
            "It costs 3.5 dollars. Then more."
        );
        assert_eq!(outline("Only one."), "Only one.");
        assert_eq!(outline(""), "");
        let long = "字".repeat(300);
        assert_eq!(outline(&long).chars().count(), 200);
    }

    #[test]
    fn extracts_year() {
        assert_eq!(year("1999-03-31"), Some("1999"));
        assert_eq!(year("99"), None);
        assert_eq!(year(""), None);
    }

    fn crew_member(id: u64, name: &str, job: &str) -> CrewMember {
        CrewMember {
            id,
            name: name.into(),
            job: job.into(),
        }
    }

    #[test]
    fn crew_is_deduplicated_and_filtered() {
        let members = [
            crew_member(1, "A", "Writer"),
            crew_member(1, "A", "Screenplay"),
            crew_member(2, "B", "Director"),
            crew_member(3, "C", "Producer"),
            crew_member(4, "D", "Gaffer"),
        ];
        let with = Xml::document("r", |x| crew(x, &members, true));
        assert_eq!(with.matches("<credits").count(), 1);
        assert!(with.contains(r#"<director tmdbid="2">B</director>"#));
        assert!(with.contains("<producer>C</producer>"));
        assert!(!with.contains("Gaffer") && !with.contains(">D<"));

        let without = Xml::document("r", |x| crew(x, &members, false));
        assert!(!without.contains("producer"));
    }

    #[test]
    fn movie_nfo_contains_core_fields() {
        let movie: MovieDetail = serde_json::from_value(serde_json::json!({
            "id": 603,
            "title": "黑客帝国",
            "original_title": "The Matrix",
            "overview": "第一句。第二句。第三句。",
            "release_date": "1999-03-31",
            "imdb_id": "tt0133093",
            "vote_average": 8.2,
            "vote_count": 100,
            "genres": [{"name": "动作"}],
            "release_dates": {"results": [
                {"iso_3166_1": "DE", "release_dates": [{"certification": "16"}]},
                {"iso_3166_1": "US", "release_dates": [{"certification": ""}, {"certification": "R"}]}
            ]},
            "keywords": {"keywords": [{"name": "hacker"}]},
            "videos": {"results": [{"key": "abc", "site": "YouTube", "type": "Trailer"}]},
            "poster_path": "/p.jpg"
        }))
        .unwrap();
        let nfo = super::movie(&movie, None);
        for expected in [
            "<title>黑客帝国</title>",
            "<outline>第一句。第二句。</outline>",
            r#"<uniqueid type="imdb">tt0133093</uniqueid>"#,
            "<year>1999</year>",
            "<value>8.2</value>",
            "<mpaa>R</mpaa>",
            "<genre>动作</genre>",
            "<tag>hacker</tag>",
            "videoid=abc</trailer>",
            r#"<thumb aspect="poster">https://image.tmdb.org/t/p/original/p.jpg</thumb>"#,
        ] {
            assert!(nfo.contains(expected), "missing {expected}\n{nfo}");
        }
    }
}
