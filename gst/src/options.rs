use clap::{ArgEnum, PossibleValue};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum InputOption {
    Libcamerasrc,
    Videotestsrc,
}

impl InputOption {
    pub fn possible_values() -> impl Iterator<Item = PossibleValue<'static>> {
        InputOption::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::fmt::Display for InputOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for InputOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}

#[derive(Debug)]
pub struct VideoParameter {
    pub encoder: &'static str,
    pub encoding_name: &'static str,
    pub payloader: &'static str,
    pub parser: &'static str,
    pub requirements: &'static str,
}

pub const H264_SOFTWARE: VideoParameter = VideoParameter {
    requirements: "x264",
    encoder: "x264enc tune=zerolatency ! 'video/x-h264,level=(string)4'",
    encoding_name: "h264",
    parser: "h264parse",
    payloader: "rtph264pay aggregate-mode=zero-latency",
};

pub const H264_HARDWARE: VideoParameter = VideoParameter {
    requirements: "video4linux2",
    encoder: "v4l2h264enc extra-controls='controls,repeat_sequence_header=1' ! 'video/x-h264,level=(string)4'",
    encoding_name: "h264",
    parser: "h264parse",
    payloader: "rtph264pay aggregate-mode=zero-latency",
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum VideoEncodingOption {
    H264Software,
    H264Hardware,
}

impl From<VideoEncodingOption> for VideoParameter {
    fn from(opt: VideoEncodingOption) -> Self {
        match opt {
            VideoEncodingOption::H264Hardware => H264_HARDWARE,
            VideoEncodingOption::H264Software => H264_SOFTWARE,
        }
    }
}

impl VideoEncodingOption {
    pub fn possible_values() -> impl Iterator<Item = PossibleValue<'static>> {
        VideoEncodingOption::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::fmt::Display for VideoEncodingOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for VideoEncodingOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}
