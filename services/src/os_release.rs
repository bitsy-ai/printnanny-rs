// MIT License

// Copyright (c) 2018 System76
// Modified (c) 2022 Bitsy Ai Labs
// Derived from: https://docs.rs/os-release/latest/src/os_release/

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR

use super::file::open;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{self, prelude::*, BufReader};
use std::iter::FromIterator;
use std::path::Path;

fn is_enclosed_with(line: &str, pattern: char) -> bool {
    line.starts_with(pattern) && line.ends_with(pattern)
}

fn parse_line(line: &str, skip: usize) -> &str {
    let line = line[skip..].trim();
    if is_enclosed_with(line, '"') || is_enclosed_with(line, '\'') {
        &line[1..line.len() - 1]
    } else {
        line
    }
}

macro_rules! map_keys {
    ($item:expr, { $($pat:expr => $field:expr),+ }) => {{
        $(
            if $item.starts_with($pat) {
                $field = parse_line($item, $pat.len()).into();
                continue;
            }
        )+
    }};
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct OsRelease {
    pub bug_report_url: String,
    pub build_id: String,
    pub home_url: String,
    pub id_like: String,
    pub image_name: String,
    pub id: String,
    pub name: String,
    pub pretty_name: String,
    pub privacy_policy_url: String,
    pub support_url: String,
    pub variant_id: String,
    pub variant_name: String,
    pub version_codename: String,
    pub version_id: String,
    pub version: String,
    pub yocto_codename: String,
    pub yocto_version: String,
    /// Additional keys not covered by the API.
    pub extra: BTreeMap<String, String>,
}

impl OsRelease {
    /// Attempt to parse the contents of `/etc/os-release`.
    pub fn new() -> io::Result<OsRelease> {
        let file = open("/etc/os-release")?;
        let reader = BufReader::new(file);
        Ok(OsRelease::from_iter(reader.lines().flatten()))
    }

    /// Attempt to parse any `/etc/os-release`-like file.
    pub fn new_from<P: AsRef<Path>>(path: P) -> io::Result<OsRelease> {
        let file = open(path)?;
        let reader = BufReader::new(file);
        Ok(OsRelease::from_iter(reader.lines().flatten()))
    }
}

impl FromIterator<String> for OsRelease {
    fn from_iter<I: IntoIterator<Item = String>>(lines: I) -> Self {
        let mut os_release = Self::default();

        for line in lines {
            let line = line.trim();
            map_keys!(line, {
                "BUG_REPORT_URL=" => os_release.bug_report_url,
                "BUILD_ID=" => os_release.build_id,
                "HOME_URL=" => os_release.home_url,
                "ID_LIKE=" => os_release.id_like,
                "IMAGE_NAME=" => os_release.image_name,
                "ID=" => os_release.id,
                "NAME=" => os_release.name,
                "PRETTY_NAME=" => os_release.pretty_name,
                "PRIVACY_POLICY_URL=" => os_release.privacy_policy_url,
                "SUPPORT_URL=" => os_release.support_url,
                "VARIANT_ID=" => os_release.variant_id,
                "VARIANT_NAME=" => os_release.variant_name,
                "VERSION_CODENAME=" => os_release.version_codename,
                "VERSION_ID=" => os_release.version_id,
                "VERSION=" => os_release.version,
                "YOCTO_CODENAME=" => os_release.yocto_codename,
                "YOCTO_VERSION=" => os_release.yocto_version
            });

            if let Some(pos) = line.find('=') {
                if line.len() > pos + 1 {
                    os_release.extra.insert(
                        line[..pos].trim().to_owned(),
                        line[pos + 1..].replace('"', "").to_owned(),
                    );
                }
            }
        }

        os_release
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OTHER_EXAMPLE: &str = r#"PRETTY_NAME="Ubuntu 22.04 LTS"
TESTING="newfield"
NAME="Ubuntu"
VERSION_ID="22.04"
VERSION="22.04 LTS (Jammy Jellyfish)"
VERSION_CODENAME=jammy
ID=ubuntu
ID_LIKE=debian
HOME_URL="https://www.ubuntu.com/"
SUPPORT_URL="https://help.ubuntu.com/"
BUG_REPORT_URL="https://bugs.launchpad.net/ubuntu/"
PRIVACY_POLICY_URL="https://www.ubuntu.com/legal/terms-and-policies/privacy-policy"
UBUNTU_CODENAME=jammy
"#;

    #[test]
    fn other_os_release() {
        let os_release = OsRelease::from_iter(OTHER_EXAMPLE.lines().map(|x| x.into()));
        assert_eq!(
            os_release,
            OsRelease {
                name: "Ubuntu".into(),
                version: "22.04 LTS (Jammy Jellyfish)".into(),
                id: "ubuntu".into(),
                id_like: "debian".into(),
                pretty_name: "Ubuntu 22.04 LTS".into(),
                version_id: "22.04".into(),
                home_url: "https://www.ubuntu.com/".into(),
                support_url: "https://help.ubuntu.com/".into(),
                bug_report_url: "https://bugs.launchpad.net/ubuntu/".into(),
                privacy_policy_url:
                    "https://www.ubuntu.com/legal/terms-and-policies/privacy-policy".into(),
                version_codename: "jammy".into(),
                extra: {
                    let mut map = BTreeMap::new();
                    map.insert("TESTING".to_owned(), "newfield".to_owned());
                    map.insert("UBUNTU_CODENAME".to_owned(), "jammy".to_owned());
                    map
                },
                ..OsRelease::default()
            }
        )
    }

    const PRINTNANNY_OS_EXAMPLE: &str = r#"ID=printnanny
ID_LIKE="BitsyLinux"
IMAGE_NAME="printnanny-debug-image-raspberrypi4-64-20221022033443"
BUILD_ID="2022-06-18T18:46:49Z"
NAME="PrintNanny Linux"
VERSION="0.1.2 (Amber)"
VERSION_ID=0.1.2
PRETTY_NAME="PrintNanny Linux 0.1.2 (Amber)"
VERSION_CODENAME="Amber"
HOME_URL="https://printnanny.ai"
SUPPORT_URL="https://printnanny.ai"
BUG_REPORT_URL="https://github.com/bitsy-ai/printnanny-os/issues"
PRIVACY_POLICY_URL="https://printnanny.ai/privacy-policy"
YOCTO_VERSION="4.0.1"
YOCTO_CODENAME="Kirkstone"
SDK_VERSION="0.1.2"
VARIANT_NAME="PrintNanny OctoPrint Edition"
VARIANT_ID=printnanny-octoprint
"#;

    #[test]
    fn printnanny_os_release() {
        let os_release = OsRelease::from_iter(PRINTNANNY_OS_EXAMPLE.lines().map(|x| x.into()));

        assert_eq!(
            os_release,
            OsRelease {
                name: "PrintNanny Linux".into(),
                build_id: "2022-06-18T18:46:49Z".into(),
                version: "0.1.2 (Amber)".into(),
                id: "printnanny".into(),
                id_like: "BitsyLinux".into(),
                pretty_name: "PrintNanny Linux 0.1.2 (Amber)".into(),
                version_id: "0.1.2".into(),
                variant_id: "printnanny-octoprint".into(),
                variant_name: "PrintNanny OctoPrint Edition".into(),
                home_url: "https://printnanny.ai".into(),
                yocto_codename: "Kirkstone".into(),
                yocto_version: "4.0.1".into(),
                support_url: "https://printnanny.ai".into(),
                bug_report_url: "https://github.com/bitsy-ai/printnanny-os/issues".into(),
                privacy_policy_url: "https://printnanny.ai/privacy-policy".into(),
                version_codename: "Amber".into(),
                image_name: "printnanny-debug-image-raspberrypi4-64-20221022033443".into(),
                extra: {
                    let mut map = BTreeMap::new();
                    map.insert("SDK_VERSION".to_owned(), "0.1.2".to_owned());
                    map
                }
            }
        )
    }
}
