use std::cmp::Ordering;

use super::types::Image;

const IMAGE_BASE: &str = "https://image.tmdb.org/t/p/";

/// Full CDN URL for an image `path` rendered at `size` (e.g. `w500`, `original`).
pub fn url(size: &str, path: &str) -> String {
    format!("{IMAGE_BASE}{size}{path}")
}

/// Picks the best image for the preferred languages (in order), then
/// language-neutral images, then anything. Within a group, higher votes win,
/// with resolution as the tiebreaker.
pub fn select_best<'a, I>(images: I, languages: &[String]) -> Option<&'a Image>
where
    I: IntoIterator<Item = &'a Image>,
    I::IntoIter: Clone,
{
    let images = images.into_iter();
    let best_where = |pred: &dyn Fn(&Image) -> bool| {
        images
            .clone()
            .filter(|img| pred(img))
            .max_by(|a, b| quality_cmp(a, b))
    };

    if !languages.is_empty() {
        for lang in languages {
            if let Some(img) = best_where(&|img| img.iso_639_1.as_deref() == Some(lang)) {
                return Some(img);
            }
        }
        if let Some(img) = best_where(&|img| img.iso_639_1.as_deref().is_none_or(str::is_empty)) {
            return Some(img);
        }
    }
    best_where(&|_| true)
}

/// Emby/Kodi expect raster clearlogos, so SVG logos are ignored.
pub fn is_raster(img: &Image) -> bool {
    !img.file_path.to_ascii_lowercase().ends_with(".svg")
}

fn quality_cmp(a: &Image, b: &Image) -> Ordering {
    a.vote_average
        .partial_cmp(&b.vote_average)
        .unwrap_or(Ordering::Equal)
        .then_with(|| a.width.unwrap_or(0).cmp(&b.width.unwrap_or(0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(path: &str, lang: Option<&str>, votes: f64, width: u32) -> Image {
        Image {
            file_path: path.into(),
            width: Some(width),
            vote_average: Some(votes),
            iso_639_1: lang.map(String::from),
        }
    }

    fn langs(l: &[&str]) -> Vec<String> {
        l.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn prefers_languages_in_order() {
        let images = [
            img("/en.jpg", Some("en"), 9.0, 100),
            img("/zh.jpg", Some("zh"), 5.0, 100),
            img("/none.jpg", None, 10.0, 100),
        ];
        let pick = |l: &[&str]| select_best(&images, &langs(l)).map(|i| i.file_path.as_str());
        assert_eq!(pick(&["zh", "en"]), Some("/zh.jpg"));
        assert_eq!(pick(&["ja", "en"]), Some("/en.jpg"));
        assert_eq!(pick(&["ja"]), Some("/none.jpg"));
        assert_eq!(pick(&[]), Some("/none.jpg"));
    }

    #[test]
    fn breaks_ties_by_width() {
        let images = [
            img("/small.jpg", None, 5.0, 100),
            img("/big.jpg", None, 5.0, 2000),
        ];
        assert_eq!(select_best(&images, &[]).unwrap().file_path, "/big.jpg");
    }

    #[test]
    fn filters_svg_logos() {
        let logos = [img("/a.svg", None, 9.0, 100), img("/b.png", None, 1.0, 100)];
        let best = select_best(logos.iter().filter(|i| is_raster(i)), &[]);
        assert_eq!(best.unwrap().file_path, "/b.png");
    }
}
