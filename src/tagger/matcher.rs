use fuzzywuzzy::fuzz;
use std::cmp::Ordering;
use std::time::Duration;
//use serde::{Serialize, Deserialize};

use crate::tagger::helpers::Helpers;
use crate::tagger::{MultipleMatchesSort, TaggerConfig, Track};

const confidence_level: f64 = 95.0;
const match_artists: bool = true;

pub struct Matcher {}
impl Matcher {
    fn match_duration(local: &Track, api: &Track, config: &TaggerConfig) -> bool {
        if !config.match_duration {
            return true;
        } else if local.duration.unwrap() == Duration::ZERO
            || api.duration.unwrap() == Duration::ZERO
        {
            return true;
        } else {
            let diff = (local.duration.unwrap().as_secs() as i64
                - api.duration.unwrap().as_secs() as i64)
                .abs() as u64;
            return diff <= config.max_duration_difference;
        }
    }

    fn match_artists(local: &Track, api: &Track, strictness: f64) -> bool {
        //Artist fields
        let artist_coinc: f64 = fuzz::ratio(&local.artist.unwrap(), &api.artist.unwrap()).into();
        let remixer_coinc: f64 = fuzz::ratio(
            &local.remixer.unwrap_or("".to_string()),
            &api.remixer.unwrap_or("".to_string()),
        )
        .into();

        //Artists in vectors
        let artists_coinc: f64 = fuzz::token_sort_ratio(
            &Helpers::join_artists(&local.artists.unwrap_or(vec!["".to_string()])),
            &Helpers::join_artists(&api.artists.unwrap_or(vec!["".to_string()])),
            true,
            true,
        )
        .into();
        let main_artists_coinc: f64 = fuzz::token_sort_ratio(
            &Helpers::join_artists(&local.main_artists.unwrap_or(vec!["".to_string()])),
            &Helpers::join_artists(&api.main_artists.unwrap_or(vec!["".to_string()])),
            true,
            true,
        )
        .into();
        let feat_artists_coinc: f64 = fuzz::token_sort_ratio(
            &Helpers::join_artists(&local.feat_artists.unwrap_or(vec!["".to_string()])),
            &Helpers::join_artists(&api.feat_artists.unwrap_or(vec!["".to_string()])),
            true,
            true,
        )
        .into();
        let remixers_coinc: f64 = fuzz::token_sort_ratio(
            &Helpers::join_artists(&local.remixers.unwrap_or(vec!["".to_string()])),
            &Helpers::join_artists(&api.remixers.unwrap_or(vec!["".to_string()])),
            true,
            true,
        )
        .into();

        if artist_coinc >= strictness
            || artists_coinc >= strictness
            || (main_artists_coinc >= strictness
                && (feat_artists_coinc >= strictness
                    || remixer_coinc >= strictness
                    || remixers_coinc >= strictness))
        {
            return true;
        }
        return false;
    }

    fn one_artist(local: &Track, api: &Track, strictness: f64) -> bool {
        for artist in local.artists.unwrap_or(vec!["".to_string()]) {
            if api
                .artists
                .unwrap_or(vec!["".to_string()])
                .contains(&artist)
            {
                return true;
            }
        }
        return false;
    }

    // Match local track to API track
    fn compare_tracks(local: &Track, api: &Track, config: &TaggerConfig) -> Option<(f64, Track)> {
        if !Matcher::match_duration(local, api, &config) {
            return None;
        }

        // Helpers
        let title_coinc: f64 = fuzz::ratio(&local.title?, &api.title?).into();
        let name_coinc: f64 = fuzz::ratio(&local.name?, &api.name?).into();
        let mix_coinc: f64 = fuzz::ratio(&local.mix?, &api.mix?).into();
        // Exact match
        if title_coinc >= confidence_level
            || (name_coinc >= confidence_level && mix_coinc >= confidence_level)
        {
            if match_artists && Matcher::match_artists(local, api, config.strictness) {
                return Some((1.0, api.to_owned()));
            } else {
                return Some((1.0, api.to_owned()));
            }
        }
        // Fuzzy match
        else if title_coinc >= config.strictness {
            if match_artists && Matcher::match_artists(local, api, config.strictness) {
                return Some((title_coinc, api.to_owned()));
            } else {
                return Some((title_coinc, api.to_owned()));
            }
        }
        // No match
        else {
            return None;
        }
    }

    // Match local track to API response (set of tracks)
    pub fn match_track(
        local: &Track,
        tracks: &Vec<Track>,
        config: &TaggerConfig,
    ) -> Option<(f64, Track)> {
        let mut exact_matches: Vec<(f64, Track)> = vec![];
        let mut fuzzy_matches: Vec<(f64, Track)> = vec![];

        // Go through API set of tracks
        for track in tracks {
            // Try to match with local track
            let result: Option<(f64, Track)> = Matcher::compare_tracks(local, track, &config);
            if result.unwrap().0 == 1.00 {
                exact_matches.push(result.unwrap())
            } else if result.unwrap().0 > config.strictness {
                fuzzy_matches.push(result.unwrap())
            }
        }

        // Use exact matches
        if !exact_matches.is_empty() {
            Matcher::sort_tracks(&mut exact_matches, &config);
            return Some(exact_matches[0]);
        }
        // Use fuzzy matches
        else if !fuzzy_matches.is_empty() {
            fuzzy_matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let best_acc = fuzzy_matches[0].0;
            let mut fuzz: Vec<(f64, Track)> = fuzzy_matches
                .into_iter()
                .filter(|(acc, _)| *acc >= best_acc)
                .collect();
            Matcher::sort_tracks(&mut fuzz, &config);
            Some(fuzz[0])
        }
        // No match
        else {
            return None;
        }
    }

    /// Sort matched tracks by release dates
    fn sort_tracks(tracks: &mut Vec<(f64, Track)>, config: &TaggerConfig) {
        match config.multiple_matches {
            MultipleMatchesSort::Default => {}
            MultipleMatchesSort::Oldest => tracks.sort_by(|a, b| {
                if a.1.release_date.is_none() || b.1.release_date.is_none() {
                    Ordering::Equal
                } else {
                    a.1.release_date
                        .as_ref()
                        .unwrap()
                        .cmp(b.1.release_date.as_ref().unwrap())
                }
            }),
            MultipleMatchesSort::Newest => tracks.sort_by(|a, b| {
                if a.1.release_date.is_none() || b.1.release_date.is_none() {
                    Ordering::Equal
                } else {
                    b.1.release_date
                        .as_ref()
                        .unwrap()
                        .cmp(a.1.release_date.as_ref().unwrap())
                }
            }),
        }
    }
}
