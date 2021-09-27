use regex::Regex;
use std::error::Error;
use std::time::Duration;
// use serde::{Serialize, Deserialize};

pub struct Helpers {}
impl Helpers {
    pub fn parse_duration(input: &str) -> Result<Duration, Box<dyn Error>> {
        let clean = input.replace("(", "").replace(")", "");
        let mut parts = clean.trim().split(":").collect::<Vec<&str>>();
        parts.reverse();
        let mut seconds: u64 = parts.first().ok_or("Invalid timestamp!")?.parse()?;
        if parts.len() > 1 {
            seconds += parts[1].parse::<u64>()? * 60;
        }
        if parts.len() > 2 {
            seconds += parts[2].parse::<u64>()? * 3600;
        }
        Ok(Duration::from_secs(seconds))
    }

    pub fn clean_spaces(input: &str) -> String {
        let expression = Regex::new(r"/^[^ ][\w\W ]*[^ ]/").unwrap();
        return expression
            .replace_all(input, "")
            .to_string()
            .replace("  ", " ");
    }

    pub fn remove_special_characters(input: &str) -> String {
        let expression = Regex::new(r"[^\w\s]+").unwrap(); // [.,()[\]&_"'\-\/\\^]+
        return expression.replace_all(input, "").to_string();
    }

    pub fn remove_initial_articles(input: &str) -> String {
        let expression = Regex::new(r"^((a|an|the) )").unwrap();
        return expression.replace_all(input, "").to_string();
    }

    pub fn remove_feat(input: &str) -> String {
        let expression =
            Regex::new(r" ([Ff]eat(\.|\ |uring)| [Ff]eat(\.|\ |uring)|[Ff]t(\.|\ )).*").unwrap();
        return expression.replace_all(input, "").to_string();
    }

    pub fn remove_feat_parenthesis(input: &str) -> String {
        let expression = Regex::new(r"(\(([Ff]eat(\.|\ |uring)| [Ff]eat(\.|\ |uring)|[Ff]t(\.|\ ))[^)]*\))|(\[([Ff]eat(\.|\ |uring)| [Ff]eat(\.|\ |uring)|[Ff]t(\.|\ ))[^]]*\])").unwrap();
        return expression.replace_all(input, "").to_string();
    }

    pub fn remove_mix_parenthesis(input: &str) -> String {
        let expression = Regex::new(r"(\([^(]*(\ [Rr]emix|\ [Mm]ix|\ [Ee]dit|\ [Bb]ootleg)\))|(\[[^[]*(\ [Rr]emix|\ [Mm]ix|\ [Ee]dit|\ [Bb]ootleg)\])").unwrap();
        return expression.replace_all(input, "").to_string();
    }

    pub fn name(title: &str) -> String {
        let mut clean = Helpers::remove_mix_parenthesis(title);
        clean = Helpers::remove_feat_parenthesis(&clean);
        return Helpers::clean_spaces(&clean);
    }

    pub fn mix(title: &str) -> String {
        let expression = Regex::new(r"(\([^(]*(\ [Rr]emix|\ [Mm]ix|\ [Ee]dit|\ [Bb]ootleg)[^(]*\))|(\[[^[]*(\ [Rr]emix|\ [Mm]ix|\ [Ee]dit|\ [Bb]ootleg)[^[]*\])|(\-[^-]*(\ [Rr]emix|\ [Mm]ix|\ [Ee]dit|\ [Bb]ootleg)[^-]*)").unwrap();
        if expression.is_match(title) {
            // In case it has 2 matching segments (XXX Remix)[Radio Edit]
            if expression.find_iter(title).count() > 1 {
                let mut result: Vec<String> = vec![];
                for version in expression.captures_iter(title) {
                    let current: String =
                        Helpers::clean_spaces(&version.get(0).map_or("", |m| m.as_str()));
                    result.push(current);
                }
                return result.join(" - ");
            } else {
                return Helpers::clean_spaces(
                    expression
                        .captures(title)
                        .unwrap()
                        .get(1)
                        .map_or("", |m| m.as_str()),
                );
            }
        } else {
            return "".to_string();
        }
    }

    pub fn title(name: String, mix: Option<String>, version: Option<String>) -> String {
        if let Some(m) = mix {
            if let Some(v) = version {
                if m.trim().is_empty() && !v.trim().is_empty() {
                    format!("{} [{}]", name, v.trim())
                } else if !m.trim().is_empty() && v.trim().is_empty() {
                    format!("{} ({})", name, m.trim())
                } else {
                    format!("{} ({}) [{}]", name, m.trim(), v.trim())
                }
            } else {
                if m.trim().is_empty() {
                    name.to_string()
                } else {
                    format!("{} ({})", name, m.trim())
                }
            }
        } else if let Some(v) = version {
            if v.trim().is_empty() {
                name.to_string()
            } else {
                format!("{} [{}]", name, v.trim())
            }
        } else {
            name
        }
    }

    pub fn remixer(title: &str) -> String {
        let expression = Regex::new(
            r"(\ [Rr]emix|\ [Mm]ix|\ [Ee]dit|\ [Bb]ootleg)|(Radio|Original|Extended|Short|Clean)",
        )
        .unwrap();
        let remixer: String = expression.replace_all(&Helpers::mix(title), "").to_string();
        if remixer.len() != 0 {
            return Helpers::clean_spaces(&remixer);
        } else {
            return "".to_string();
        }
    }

    pub fn parse_artist(artist: &str) -> Vec<String> {
        if artist.contains(',') {
            return artist
                .split(',')
                .collect::<Vec<&str>>()
                .into_iter()
                .map(|v| v.to_owned())
                .collect();
        } else if artist.contains(';') {
            return artist
                .split(';')
                .collect::<Vec<&str>>()
                .into_iter()
                .map(|v| v.to_owned())
                .collect();
        } else if artist.contains('/') {
            return artist
                .split('/')
                .collect::<Vec<&str>>()
                .into_iter()
                .map(|v| v.to_owned())
                .collect();
        } else {
            return vec![artist.to_string()];
        }
    }

    pub fn join_artists(artists: &Vec<String>) -> String {
        return artists.join(", ");
    }

    pub fn clean_artists(artists: &Vec<String>) -> Vec<String> {
        let mut clean: Vec<String> = artists
            .into_iter()
            .map(|a| {
                Helpers::remove_special_characters(&a.to_lowercase())
                    .trim()
                    .to_string()
            })
            .collect();
        clean.sort();
        return clean;
    }

    /*
    pub fn featuring_artists(title: str, artists: ) -> Vec<String>:
        let mut featuring_artists: Vec<String> = vec![];
        let mut expression = Regex::new(r"\ (?:[Ff]eat(?:\.|\ |uring)| [Ff]eat(\.|\ |uring)|[Ff]t(?:\.|\ ))(.*)").unwrap();
        if expression.is_match(artists) {
            for feat_artists in expression.captures_iter(arists) {
                // Remove parenthesis
                feat_artists = feat_artists[1:-1];
                // Remove feat string
                expression = Regex::new(r"(?:(?:[Ff]eat(?:\.|\ |uring)| [Ff]eat(?:\.|\ |uring)|[Ff]t(?:\.|\ )))\ ").unwrap();
                feat_artists = expression.replace_all(feat_artists, "");
                // Convert string into list
                feat_artists = feat_artists.split(',');

                // Add artists to featurers list
                for artist in feat_artist {
                    if !featuring_artists.contains(artist) {
                        featuring_artists.append(artist);
                    }
                }
            }

            // Check that the last item added to the list doesn't contain more than 1 artist (because of a possible and/&)
            let last_artist = featuring_artists[-1]
            expression = Regex::new(r" (?:[Aa]nd|&) ").unwrap();
            if expression.find_iter(title).count() > 1 {
                featuring_artists.remove(-1);
                last_artist = re.sub(
                    ' (?:[Aa]nd|&) (?!.*(?:[Aa]nd|&))', ',', last_artist)
                last_artist = last_artist.split(',')
                for artist in last_artist:
                    featuring_artists.append(artist)

        // Get featuring artists from title field
        title_field = re.findall(
            r'(\(([Ff]eat(\.|\ |uring)| [Ff]eat(\.|\ |uring)|[Ff]t(\.|\ ))[^)]*\))|(\[([Ff]eat(\.|\ |uring)| [Ff]eat(\.|\ |uring)|[Ff]t(\.|\ ))[^]]*\])', title)
        if len(title_field) != 0:
            while(isinstance(title_field, str) == False):
                title_field = title_field[0]
            // Remove parenthesis
            title_field = title_field[1:-1]
            // Remove feat string
            title_field = re.sub(
                r'(([Ff]eat(\.|\ |uring)| [Ff]eat(\.|\ |uring)|[Ff]t(\.|\ )))\ ', "", title_field)
            // Convert string into list
            title_field = title_field.split(',')

            // Add artists to featurers list
            for artist in titleField:
                featuring_artists.append(artist)

            // Check that the last item added to the list doesn't contain more than 1 artist (because of a possible and/&)
            last_artist = featuring_artists[-1]
            if len(re.findall(' (?:[Aa]nd|&) ', last_artist)) > 1:
                del featuring_artists[-1]
                last_artist = re.sub(
                    ' (?:[Aa]nd|&) (?!.*(?:[Aa]nd|&))', ',', last_artist)
                last_artist = last_artist.split(',')
                for artist in last_artist:
                    if artist not in featuring_artists:
                        featuring_artists.append(artist)

        // Results
        for artist in featuring_artists:
            // Remove possible white spaces at the beginning or at the end
            while artist[0] in ' ([,-':
                artist = artist[1:]
            while artist[-1] in ' )],-':
                artist = artist[:-1]
        if (len(featuring_artists) != 0):
            return featuring_artists
        else:
            return "".to_string();
    }

    pub fn main_artists(title: str, artists) -> Vec<String> {
        feat_artists = featuring_artists(title, artists)
        main_artists = []

        // Get featuring artists from artists field
        if (isinstance(artists, list) == False):
            // Remove featuring artists from artists string
            artist_field = remove_feat(artists)

            // Convert string into list
            artist_field = artist_field.split(',')

            // Add artists (only artists, no featurers) to the list
            for artist in artist_field:
                if artist not in feat_artists:
                    main_artists.append(artist)

            // Check that the last item added to the list doesn't contain more than 1 artist (because of a possible and/&)
            last_artist = main_artists[-1]
            if len(re.findall(' (?:[Aa]nd|&) ', last_artist)) > 1:
                del main_artists[-1]
                last_artist = re.sub(
                    ' (?:[Aa]nd|&) (?!.*(?:[Aa]nd|&))', ',', last_artist)
                last_artist = last_artist.split(',')
                for artist in last_artist:
                    main_artists.append(artist)
        else:
            // Add artists (only artists, no featurers) to the list
            for artist in artists:
                if artist not in feat_artists:
                    main_artists.append(artist)

        // Results
        for artist in main_artists:
            // Remove possible white spaces at the beginning or at the end
            while artist[0] == ' ':
                artist = artist[1:]
            if artist[-1] == ' ':
                artist = artist[:-1]
        if (len(main_artists) != 0):
            return main_artists
        else:
            return ""
    }
    */

    pub fn parse_genre(genre: &str) -> Vec<String> {
        if genre.contains(',') {
            return genre
                .split(',')
                .collect::<Vec<&str>>()
                .into_iter()
                .map(|v| Helpers::clean_spaces(v))
                .collect();
        } else if genre.contains(';') {
            return genre
                .split(';')
                .collect::<Vec<&str>>()
                .into_iter()
                .map(|v| Helpers::clean_spaces(v))
                .collect();
        } else if genre.contains('/') {
            return genre
                .split('/')
                .collect::<Vec<&str>>()
                .into_iter()
                .map(|v| Helpers::clean_spaces(v))
                .collect();
        } else {
            return vec![genre.to_string()];
        }
    }

    pub fn join_genres(genres: &Vec<String>) -> String {
        return genres.join(", ");
    }
}
