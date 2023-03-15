//! Define [`PipelineBus`] which encapsulate methods
//! of [`Bus API`][1]
//!
//! The actual bus is [`GStreamer`] [`GstBus`][2]
//!
//! [`GStreamer`]: https://gstreamer.freedesktop.org/
//! [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon_-_C_API#Bus
//! [2]: https://gstreamer.freedesktop.org/documentation/additional/design/gstbus.html
use crate::{gstd_types, resources::Pipeline, Error, GstClient};
use log::debug;

/// Performs requests to `pipelines/{name}/bus` endpoints
#[derive(Debug, Clone)]
pub struct PipelineBus {
    client: GstClient,
    pipeline: Pipeline,
}

impl PipelineBus {
    pub(crate) fn new(pipeline: &Pipeline) -> Self {
        Self {
            pipeline: pipeline.clone(),
            client: pipeline.client.clone(),
        }
    }
    /// Performs `GET pipelines/{name}/bus/message`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn read(&self) -> Result<gstd_types::Response, Error> {
        let url = self
            .client
            .base_url
            .join(&format!("pipelines/{}/bus/message", self.pipeline.name))
            .map_err(Error::IncorrectApiUrl)?;
        let resp = self.client.get(url).await?;
        debug!("Gst pipeline message: {:?}", resp);
        self.client.process_resp(resp).await
    }
    /// Performs `PUT pipelines/{name}?timeout={time_ns}`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn set_timeout(&self, time_ns: u64) -> Result<gstd_types::Response, Error> {
        let url = self
            .client
            .base_url
            .join(&format!(
                "pipelines/{}/bus/timeout?name={time_ns}",
                self.pipeline.name
            ))
            .map_err(Error::IncorrectApiUrl)?;

        let resp = self.client.put(url).await?;
        debug!("gstd set_timeout response: {:?}", resp);

        self.client.process_resp(resp).await
    }
    /// Performs `PUT pipelines/{name}?types={filter}`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn set_filter(&self, filter: &str) -> Result<gstd_types::Response, Error> {
        let url = self
            .client
            .base_url
            .join(&format!(
                "pipelines/{}/bus/types?name={filter}",
                self.pipeline.name
            ))
            .map_err(Error::IncorrectApiUrl)?;

        let resp = self.client.put(url).await?;
        debug!("gstd set_filter response: {:?}", resp);

        self.client.process_resp(resp).await
    }
}
