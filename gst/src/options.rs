use std::fmt;

use clap::ValueEnum;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum SrcOption {
    Libcamerasrc,
    Videotestsrc,
    Shmsrc,
}

impl fmt::Display for SrcOption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Libcamerasrc => write!(f, "libcamerasrc"),
            Self::Videotestsrc => write!(f, "videotestsrc"),
            Self::Shmsrc => write!(f, "shmsrc"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum RtpSinkOption {
    Edge,
    Cloud,
}

impl fmt::Display for RtpSinkOption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Edge => write!(f, "edge"),
            Self::Cloud => write!(f, "cloud"),
        }
    }
}
