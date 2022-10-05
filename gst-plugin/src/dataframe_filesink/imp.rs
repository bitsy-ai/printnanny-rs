// Copyright (C) 2021 Rafael Caricio <rafael@caricio.com>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0


use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

const DEFAULT_LOCATION: &str = "dataframe%05d.ipc";
const DEFAULT_MAX_FILE_DURATION: u64 = 18446744073709551615;
const DEFAULT_MAX_FILE_SIZE: u64 = 2147483648;
const DEFAULT_MAX_FILES: u32 = 0;
const DEFAULT_POST_FILE_MESSAGES: bool = false;

static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
    gst::DebugCategory::new(
        "dataframe_filesink",
        gst::DebugColorFlags::empty(),
        Some("PrintNanny Dataframe filesink"),
    )
});

struct Settings {
    location: String,
    max_file_duration: u64, // Maximum file size before starting a new file in max-size mode.
    max_file_size: u64,     // Maximum file size before starting a new file in max-size mode.
    max_files: u32, // Maximum number of files to keep on disk. Once the maximum is reached, old files start to be deleted to make room for new ones.
    post_file_messages: bool, // Post a message on the GstBus for each file.
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            location: String::from(DEFAULT_LOCATION),
            max_file_duration: DEFAULT_MAX_FILE_DURATION,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            max_files: DEFAULT_MAX_FILES,
            post_file_messages: DEFAULT_POST_FILE_MESSAGES,
        }
    }
}

#[derive(Clone)]
pub struct DataframeFileSink {
    multifilesink: gst::Element,
    settings: Arc<Mutex<Settings>>,
    // srcpad: gst::GhostPad,
    sinkpad: gst::GhostPad,
}

impl DataframeFileSink {}

#[glib::object_subclass]
impl ObjectSubclass for DataframeFileSink {
    const NAME: &'static str = "DataframeFileSink";
    type Type = super::DataframeFileSink;
    type ParentType = gst::Bin;

    // Called when a new instance is to be created. We need to return an instance
    // of our struct here and also get the class struct passed in case it's needed
    fn with_class(klass: &Self::Class) -> Self {
        // Create our two ghostpads from the templates that were registered with
        // the class. We don't provide a target for them yet because we can only
        // do so after the progressreport element was added to the bin.
        //
        // We do that and adding the pads inside glib::Object::constructed() later.
        let templ = klass.pad_template("sink").unwrap();
        let sinkpad = gst::GhostPad::from_template(&templ, Some("sink"));
        // let templ = klass.pad_template("src").unwrap();
        // let srcpad = gst::GhostPad::from_template(&templ, Some("src"));

        let multifilesink = gst::ElementFactory::make("multifilesink", Some("dataframe_multifilesink")).unwrap();

        // Return an instance of our struct
        Self {
            multifilesink,
            // srcpad,
            sinkpad,
            settings: Arc::new(Mutex::new(Settings::default()))
        }
    }
}

impl BinImpl for DataframeFileSink {}

impl ObjectImpl for DataframeFileSink {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecString::builder("location")
                    .nick("File Location")
                    .blurb("Location of the file to write")
                    .default_value(Some(DEFAULT_LOCATION))
                    .build(),
                glib::ParamSpecUInt64::builder("max-file-duration")
                    .nick("Max File Duration")
                    .blurb("Maximum file size before starting a new file in max-size mode.")
                    .default_value(DEFAULT_MAX_FILE_DURATION)
                    .build(),
                glib::ParamSpecUInt64::builder("max-file-size")
                    .nick("Max File Size")
                    .blurb("Maximum file size before starting a new file in max-size mode.")
                    .default_value(DEFAULT_MAX_FILE_SIZE)
                    .build(),
                glib::ParamSpecUInt::builder("max-files")
                    .nick("Max Files")
                    .blurb("Maximum number of files to keep on disk. Once the maximum is reached, old files start to be deleted to make room for new ones.")
                    .default_value(DEFAULT_MAX_FILES)
                    .build(),
                glib::ParamSpecBoolean::builder("post-messages")
                    .nick("Max Files")
                    .blurb("Maximum number of files to keep on disk. Once the maximum is reached, old files start to be deleted to make room for new ones.")
                    .default_value(DEFAULT_POST_FILE_MESSAGES)
                    .build(),   
            ]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        _obj: &Self::Type,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        let mut settings = self.settings.lock().unwrap();
        match pspec.name() {
            "location" => {
                settings.location = value
                    .get::<Option<String>>()
                    .expect("type checked upstream")
                    .unwrap_or_else(|| DEFAULT_LOCATION.into());
                self.multifilesink
                    .set_property("location", &settings.location);
            }
            "max-file-duration" => {
                settings.max_file_duration = value
                    .get::<u64>()
                    .expect("type checked upstream");
                self.multifilesink
                    .set_property("max-file-duration", &settings.max_file_duration );            
            }
            "max-file-size" => {
                settings.max_file_size = value
                    .get::<u64>()
                    .expect("type checked upstream");
                self
                    .multifilesink
                    .set_property("max-file-size", &settings.max_file_size);            
            }
            "max-files" => {
                settings.max_files = value
                    .get::<u32>()
                    .expect("type checked upstream");
                self
                    .multifilesink
                    .set_property("max-files", &settings.max_files);            
            }
            "post-messages" => {
                settings.post_file_messages = value
                    .get::<bool>()
                    .expect("type checked upstream");
                self
                    .multifilesink
                    .set_property("post-messages", &settings.post_file_messages);            
            }
            _ => unimplemented!("Property is not implemented {:?}", value),
        };
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        let settings = self.settings.lock().unwrap();
        match pspec.name() {
            "location" => settings.location.to_value(),
            "max-file-duration" => settings.max_file_duration.to_value(),
            "max-file-size" => settings.max_file_size.to_value(),
            "max-files" => settings.max_files.to_value(),
            "post-messages" => settings.post_file_messages.to_value(),
            _ => unimplemented!(),
        }
    }

    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        obj.add(&self.multifilesink).unwrap();
        // only one ghostpad is needed here, since a sink element has no srcpad
        self.sinkpad
            .set_target(Some(&self.multifilesink.static_pad("sink").unwrap()))
            .unwrap();
        // And finally add the ghostpads to the bin.
        obj.add_pad(&self.sinkpad).unwrap();
    }
}

impl GstObjectImpl for DataframeFileSink {}

impl ElementImpl for DataframeFileSink {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
            gst::subclass::ElementMetadata::new(
                "Dataframe streaming file sink",
                "Sink/Files",
                "Dataframe streaming file sink",
                "Leigh Johnson <leigh@printnanny.ai>",
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: Lazy<Vec<gst::PadTemplate>> = Lazy::new(|| {
            // Our element can accept any possible caps
            let caps = gst::Caps::new_any();
            let sink_pad_template = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            vec![sink_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }
}
