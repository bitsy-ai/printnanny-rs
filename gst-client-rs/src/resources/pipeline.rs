//! Define [`Pipeline`] which encapsulate methods
//! of [`Pipeline API`][1]
//!
//! The actual pipeline is [`GStreamer`] [`GstPipeline`][2]
//!
//! [`GStreamer`]: https://gstreamer.freedesktop.org/
//! [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon_-_C_API#Pipelines
//! [2]: https://gstreamer.freedesktop.org/documentation/additional/design/gstpipeline.html
use crate::{
    gstd_types,
    resources::{bus::PipelineBus, element::PipelineElement},
    Error, GstClient,
};
use std::fmt::Display;

/// Performs requests to `pipelines/` endpoint
#[derive(Debug, Clone)]
pub struct Pipeline {
    /// name of the pipeline
    pub name: String,
    pub(crate) client: GstClient,
}

impl Pipeline {
    pub(crate) fn new<S: Into<String>>(name: S, client: &GstClient) -> Self {
        Self {
            name: name.into(),
            client: client.clone(),
        }
    }
    /// Creates a new pipeline .
    ///
    /// Performs `POST pipelines?name={name}&description={description}`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Arguments
    ///
    /// * `description` - pipeline with gst-launch syntax
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn create(
        &self,
        description: &str,
    ) -> Result<gstd_types::Response, Error> {
        let mut url = self.client.base_url.join("pipelines").map_err(Error::IncorrectApiUrl)?;
        url.query_pairs_mut()
            .append_pair("name", &self.name)
            .append_pair("description", description);

        let resp = self
            .client
            .post(url)
            .await?;

        // println!("{}", resp.json().await.unwrap());

        self.client.process_resp(resp).await
    }

    /// Performs `GET /pipelines/{name}/graph` API request, returning the
    /// parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn graph(&self) -> Result<gstd_types::Response, Error> {
        let url = self.client.base_url.join(&format!("pipelines/{}/graph", &self.name)).map_err(Error::IncorrectApiUrl)?;

        let resp = self
            .client
            .get(url)
            .await?;
        self.client.process_resp(resp).await
    }
    /// Performs `GET /pipelines/{name}/elements`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn elements(&self) -> Result<gstd_types::Response, Error> {
        let url = self.client.base_url.join(&format!("pipelines/{}/elements", &self.name)).map_err(Error::IncorrectApiUrl)?;
        let resp = self
            .client
            .get(url)
            .await?;
        self.client.process_resp(resp).await
    }

    /// Performs `GET /pipelines/{name}/properties`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn properties(&self) -> Result<gstd_types::Response, Error> {
        let url = self.client.base_url.join(&format!("pipelines/{}/properties", &self.name)).map_err(Error::IncorrectApiUrl)?;
        let resp = self
            .client
            .get(url)
            .await?;
        self.client.process_resp(resp).await
    }

    /// Operate with [`GStreamer Daemon`][1] pipeline element.
    ///
    /// # Arguments
    ///
    /// * `name` - name of the element in the pipeline
    ///
    /// [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
    #[must_use]
    pub fn element<S: Into<String>>(&self, name: S) -> PipelineElement {
        PipelineElement::new(name, self)
    }
    /// Operate with [`GStreamer Daemon`][1] pipeline bus.
    ///
    /// [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
    #[must_use]
    pub fn bus(&self) -> PipelineBus {
        PipelineBus::new(self)
    }

    /// Performs `POST pipelines/{name}/event?name={event_name}`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn emit_event(
        &self,
        event_name: &str,
    ) -> Result<gstd_types::Response, Error> {
        let mut url = self.client.base_url.join(&format!("pipelines/{}/event", &self.name)).map_err(Error::IncorrectApiUrl)?;
        url.query_pairs_mut()
            .append_pair("name", event_name);

        let resp = self
            .client
            .post(url)
            .await?;
        self.client.process_resp(resp).await
    }

    /// Performs `POST pipelines/{name}/event?name=eos`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn emit_event_eos(&self) -> Result<gstd_types::Response, Error> {
        let mut url = self.client.base_url.join(&format!("pipelines/{}/event", &self.name)).map_err(Error::IncorrectApiUrl)?;
        url.query_pairs_mut()
            .append_pair("name", "eos");
        let resp = self
            .client
            .post(url)
            .await?;
        self.client.process_resp(resp).await
    }

    /// Performs `POST pipelines/{name}/event?name=flush_start`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn emit_event_flush_start(&self) -> Result<gstd_types::Response, Error> {
        let mut url = self.client.base_url.join(&format!("pipelines/{}/event", &self.name)).map_err(Error::IncorrectApiUrl)?;
        url.query_pairs_mut()
            .append_pair("name", "flush_start");

        let resp = self
            .client
            .post(url)
            .await?;
        self.client.process_resp(resp).await
    }

    /// Performs `POST pipelines/{name}/event?name=flush_stop`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn emit_event_flush_stop(&self) -> Result<gstd_types::Response, Error> {
        let mut url = self.client.base_url.join(&format!("pipelines/{}/event", &self.name)).map_err(Error::IncorrectApiUrl)?;
        url.query_pairs_mut()
            .append_pair("name", "flush_stop");

        let resp = self
            .client
            .post(url)
            .await?;
        self.client.process_resp(resp).await
    }
    /// Performs `PUT pipelines/{name}/state?name=playing`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn play(&self) -> Result<gstd_types::Response, Error> {
        let mut url = self.client.base_url.join(&format!("pipelines/{}/state", &self.name)).map_err(Error::IncorrectApiUrl)?;
        url.query_pairs_mut()
            .append_pair("name", "playing");

        let resp = self
            .client
            .put(url)
            .await?;
        self.client.process_resp(resp).await
    }
    /// Performs `PUT pipelines/{name}/state?name=paused`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn pause(&self) -> Result<gstd_types::Response, Error> {
        let mut url = self.client.base_url.join(&format!("pipelines/{}/state", &self.name)).map_err(Error::IncorrectApiUrl)?;
        url.query_pairs_mut()
            .append_pair("name", "paused");
        let resp = self
            .client
            .put(url)
            .await?;
        self.client.process_resp(resp).await
    }
    /// Performs `PUT pipelines/{name}/state?name=stop`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn stop(&self) -> Result<gstd_types::Response, Error> {
        let mut url = self.client.base_url.join(&format!("pipelines/{}/state", &self.name)).map_err(Error::IncorrectApiUrl)?;

        url.query_pairs_mut()
            .append_pair("name", "stop");
        let resp = self
            .client
            .put(url)
            .await?;
        self.client.process_resp(resp).await
    }

    /// Performs `PUT pipelines/{name}/verbose?name={value}`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn set_verbose(&self, value: bool) -> Result<gstd_types::Response, Error> {
        let mut url = self.client.base_url.join(&format!("pipelines/{}/verbose", &self.name)).map_err(Error::IncorrectApiUrl)?;
        let val = if value { "true" } else { "false" };
        url.query_pairs_mut()
            .append_pair("name", val);
        let resp = self
            .client
            .put(url)
            .await?;
        self.client.process_resp(resp).await
    }

    /// Performs `DELETE /pipelines?name={name}`
    /// API request, returning the parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn delete(&self) -> Result<gstd_types::Response, Error> {
        let mut url = self.client.base_url.join(&format!("pipelines")).map_err(Error::IncorrectApiUrl)?;
        url.query_pairs_mut()
            .append_pair("name", &self.name);
        let resp = self
            .client
            .delete(url)
            .await?;
        self.client.process_resp(resp).await
    }
}
