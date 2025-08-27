// SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
//
// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(serde::Deserialize)]
pub struct AppleMusicData {
    pub results: AppleMusicDataResults,
}

#[derive(serde::Deserialize)]
pub struct AppleMusicDataResults {
    pub song: AppleMusicDataResultsSong,
}

#[derive(serde::Deserialize)]
pub struct AppleMusicDataResultsSong {
    pub data: Vec<AppleMusicDataResultsSongData>,
}

#[derive(serde::Deserialize)]
pub struct AppleMusicDataResultsSongData {
    pub id: String,
    pub attributes: AppleMusicDataResultsSongDataAttributes,
    pub relationships: AppleMusicDataResultsSongDataRelationships,
}

#[derive(serde::Deserialize)]
pub struct AppleMusicDataResultsSongDataAttributes {
    pub url: String,
    pub artwork: AppleMusicDataResultsSongDataAttributesArtwork,
}

#[derive(serde::Deserialize)]
pub struct AppleMusicDataResultsSongDataAttributesArtwork {
    pub url: String,
}

#[derive(serde::Deserialize)]
pub struct AppleMusicDataResultsSongDataRelationships {
    pub artists: AppleMusicDataResultsSongDataRelationshipsArtists,
}

#[derive(serde::Deserialize)]
pub struct AppleMusicDataResultsSongDataRelationshipsArtists {
    pub data: Vec<AppleMusicDataResultsSongDataRelationshipsArtistsData>,
}

#[derive(serde::Deserialize)]
pub struct AppleMusicDataResultsSongDataRelationshipsArtistsData {
    pub attributes: AppleMusicDataResultsSongDataRelationshipsArtistsDataAttributes,
}

#[derive(serde::Deserialize)]
pub struct AppleMusicDataResultsSongDataRelationshipsArtistsDataAttributes {
    pub artwork: AppleMusicDataResultsSongDataRelationshipsArtistsDataAttributesArtwork,
}

#[derive(serde::Deserialize)]
pub struct AppleMusicDataResultsSongDataRelationshipsArtistsDataAttributesArtwork {
    pub url: String,
}
