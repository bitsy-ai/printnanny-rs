use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_base::subclass::prelude::*;
use once_cell::sync::Lazy;
use std::sync::Mutex;

const DEFAULT_NATS_ADDRESS: &str = "127.0.0.1:4222";
const DEFAULT_NATS_SUBJECT: &str = "pi.qc";

#[derive(Debug, Clone)]
struct Settings {
    nats_address: String,
    nats_subject: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            nats_address: DEFAULT_NATS_ADDRESS.into(),
            nats_subject: DEFAULT_NATS_SUBJECT.into(),
        }
    }
}

enum State {
    Stopped,
    Started { nc: nats::Connection },
}

impl Default for State {
    fn default() -> State {
        State::Stopped
    }
}

#[derive(Default)]
pub struct NatsSink {
    settings: Mutex<Settings>,
    state: Mutex<State>,
}

static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
    gst::DebugCategory::new(
        "nats_sink",
        gst::DebugColorFlags::empty(),
        Some("NATS Sink"),
    )
});

impl NatsSink {}

#[glib::object_subclass]
impl ObjectSubclass for NatsSink {
    const NAME: &'static str = "NatsSink";
    type Type = super::NatsSink;
    type ParentType = gst_base::BaseSink;
}

impl ObjectImpl for NatsSink {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecString::builder("nats-address")
                    .nick("NATS Address")
                    .default_value(DEFAULT_NATS_ADDRESS)
                    .blurb("NATS server address")
                    .build(),
                glib::ParamSpecString::builder("nats-subject")
                    .nick("NATS Subject")
                    .default_value(DEFAULT_NATS_SUBJECT)
                    .blurb("NATS subject")
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
            "nats-address" => {
                settings.nats_address = value.get::<String>().expect("type checked upstream");
            }
            "nats-subject" => {
                settings.nats_subject = value.get::<String>().expect("type checked upstream");
            }
            _ => unimplemented!(),
        };
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        let settings = self.settings.lock().unwrap();

        match pspec.name() {
            "nats-address" => settings.nats_address.to_value(),
            _ => unimplemented!(),
        }
    }
}

impl GstObjectImpl for NatsSink {}

impl ElementImpl for NatsSink {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
            gst::subclass::ElementMetadata::new(
                "NATS Sink",
                "Sink/NATS",
                "Write stream to a NATS topic",
                "Leigh Johnson <leigh@printnanny.ai>",
            )
        });
        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: Lazy<Vec<gst::PadTemplate>> = Lazy::new(|| {
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

impl BaseSinkImpl for NatsSink {
    fn start(&self, element: &Self::Type) -> Result<(), gst::ErrorMessage> {
        let mut state = self.state.lock().unwrap();
        if let State::Started { .. } = *state {
            unreachable!("NatsSink already started");
        }

        let settings = self.settings.lock().unwrap();

        let nc = nats::connect(&settings.nats_address).map_err(|err| {
            gst::error_msg!(
                gst::ResourceError::Failed,
                [
                    "Failed to open NATS server address {} with error: {}",
                    &settings.nats_address,
                    err.to_string(),
                ]
            )
        })?;
        gst::debug!(
            CAT,
            obj: element,
            "Opened NATS connection {:?}",
            &settings.nats_address
        );

        *state = State::Started { nc: nc };
        gst::info!(CAT, obj: element, "Started");

        Ok(())
    }

    fn stop(&self, element: &Self::Type) -> Result<(), gst::ErrorMessage> {
        let mut state = self.state.lock().unwrap();

        let nc = match *state {
            State::Started { ref mut nc } => nc,
            State::Stopped => {
                gst::element_error!(element, gst::CoreError::Failed, ["Not started yet"]);
                return Err(gst::error_msg!(
                    gst::ResourceError::Settings,
                    ["NatsSink not started"]
                ));
            }
        };

        nc.flush().map_err(|err| {
            let settings = self.settings.lock().unwrap();
            gst::error_msg!(
                gst::ResourceError::Failed,
                [
                    "Failed to flush NATS connection {} with error: {}",
                    settings.nats_address,
                    err.to_string(),
                ]
            )
        })?;

        *state = State::Stopped;
        gst::info!(CAT, obj: element, "Stopped");

        Ok(())
    }

    fn render(
        &self,
        element: &Self::Type,
        buffer: &gst::Buffer,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let mut state = self.state.lock().unwrap();
        let settings = self.settings.lock().unwrap();

        let nc = match *state {
            State::Started { ref mut nc } => nc,
            State::Stopped => {
                gst::element_error!(element, gst::CoreError::Failed, ["Not started yet"]);
                return Err(gst::FlowError::Error);
            }
        };

        gst::trace!(CAT, obj: element, "Rendering {:?}", buffer);
        let map = buffer.map_readable().map_err(|_| {
            gst::element_error!(element, gst::CoreError::Failed, ["Failed to map buffer"]);
            gst::FlowError::Error
        })?;

        nc.publish(&settings.nats_subject, map.as_slice())
            .map_err(|_| {
                gst::element_error!(
                    element,
                    gst::CoreError::Failed,
                    ["Failed to publish NATS message"]
                );
                gst::FlowError::Error
            })?;

        Ok(gst::FlowSuccess::Ok)
    }
}
