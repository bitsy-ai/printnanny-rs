//! Defines [`GstClient`] for communication with
//! [`GStreamer Daemon`][1] API.
//!
//! [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
use crate::{gstd_types, resources, Error};
use reqwest::{Client, Response};
use url::Url;

/// [`GstClient`] for [`GStreamer Daemon`][1] API.
///
/// [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
#[derive(Debug, Clone)]
pub struct GstClient {
    http_client: Client,
    pub(crate) base_url: Url,
}

impl GstClient {
    /// Build [`GstClient`] for future call to [`GStreamer Daemon`][1] API.
    ///
    /// # Errors
    ///
    /// If incorrect `base_url` passed
    ///
    /// [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
    pub fn build<S: Into<String>>(base_url: S) -> Result<Self, Error> {
        Ok(Self {
            http_client: Client::new(),
            base_url: Url::parse(&base_url.into()).map_err(Error::IncorrectBaseUrl)?,
        })
    }

    pub(crate) async fn get(&self, url: reqwest::Url) -> Result<Response, Error> {
        self.http_client
            .get(url)
            .send()
            .await
            .map_err(Error::RequestFailed)
    }

    pub(crate) async fn post(&self, url: reqwest::Url) -> Result<Response, Error> {
        self.http_client
            .post(url)
            .send()
            .await
            .map_err(Error::RequestFailed)
    }

    pub(crate) async fn put(&self, url: reqwest::Url) -> Result<Response, Error> {
        self.http_client
            .put(url)
            .send()
            .await
            .map_err(Error::RequestFailed)
    }

    pub(crate) async fn delete(&self, url: reqwest::Url) -> Result<Response, Error> {
        self.http_client
            .delete(url)
            .send()
            .await
            .map_err(Error::RequestFailed)
    }

    pub(crate) async fn process_resp(&self, resp: Response) -> Result<gstd_types::Response, Error> {
        if !resp.status().is_success() {
            let status = resp.status();
            let error = &resp.text().await.map_err(Error::BadBody)?;
            return Err(Error::BadStatus(status, Some(error.to_string())));
        }

        let res = resp
            .json::<gstd_types::Response>()
            .await
            .map_err(Error::BadBody)?;

        if res.code != gstd_types::ResponseCode::Success {
            return Err(Error::GstdError(res.code));
        }
        Ok(res)
    }

    /// Performs `GET /pipelines` API request, returning the
    /// parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn pipelines(&self) -> Result<gstd_types::Response, Error> {
        let url = self
            .base_url
            .join("pipelines")
            .map_err(Error::IncorrectApiUrl)?;
        let resp = self.get(url).await?;
        self.process_resp(resp).await
    }
    /// Operate with [`GStreamer Daemon`][1] pipelines.
    ///
    /// # Arguments
    ///
    /// * `name` - name of the pipeline
    ///
    /// [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
    #[must_use]
    pub fn pipeline<S>(&self, name: S) -> resources::Pipeline
    where
        S: Into<String>,
    {
        resources::Pipeline::new(name, self)
    }
    /// Manage [`GStreamer Daemon`][1] Debug mode.
    ///
    /// [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
    #[must_use]
    pub fn debug(&self) -> resources::Debug {
        resources::Debug::new(self)
    }
}

impl Default for GstClient {
    fn default() -> Self {
        Self {
            http_client: Client::new(),
            base_url: Url::parse("http://127.0.0.1:5001").unwrap(),
        }
    }
}

impl From<Url> for GstClient {
    fn from(url: Url) -> Self {
        Self {
            http_client: Client::new(),
            base_url: url,
        }
    }
}

impl From<&Url> for GstClient {
    fn from(url: &Url) -> Self {
        Self {
            http_client: Client::new(),
            base_url: url.clone(),
        }
    }
}

#[cfg(test)]
mod spec {
    use super::*;
    use http;
    const BASE_URL: &'static str = "http://localhost:5002";
    const PIPELINE_NAME: &'static str = "test pipeline";

    const STATE_RESPONSE: &'static str = r#"
    {
        "code" : 0,
        "description" : "Success",
        "response" : {
          "name" : "state",
          "value" : "playing",
          "param" : {
              "description" : "The state of the pipeline",
              "type" : "GstdStateEnum",
              "access" : "((GstdParamFlags) READ | 2)"
          }
        }
      }
    "#;

    fn expect_url() -> Url {
        Url::parse(BASE_URL).unwrap()
    }

    #[tokio::test]
    async fn process_state_response() {
        let client = GstClient::build(BASE_URL).unwrap();
        let response = http::Response::builder()
            .status(200)
            .body(STATE_RESPONSE)
            .unwrap();

        let res = client.process_resp(response.into()).await.unwrap();

        let expected = gstd_types::ResponseT::Property(gstd_types::Property {
            name: "state".into(),
            value: gstd_types::PropertyValue::String("playing".into()),
            param: gstd_types::Param {
                description: "The state of the pipeline".into(),
                _type: "GstdStateEnum".into(),
                access: "((GstdParamFlags) READ | 2)".into(),
            },
        });

        assert_eq!(res.response, expected);
    }

    #[ignore]
    #[test]
    fn create_client_with_build() {
        let client = GstClient::build(BASE_URL).unwrap();
        assert_eq!(client.base_url, expect_url());

        let client = GstClient::build(BASE_URL.to_string()).unwrap();
        assert_eq!(client.base_url, expect_url());
    }

    #[ignore]
    #[test]
    fn create_client_from() {
        let url = expect_url();
        let client = GstClient::from(&url);
        assert_eq!(client.base_url, expect_url());

        let client = GstClient::from(url);
        assert_eq!(client.base_url, expect_url());
    }

    #[ignore]
    #[tokio::test]
    async fn create_pipeline() {
        if let Ok(client) = GstClient::build(BASE_URL) {
            let res = client.pipeline(PIPELINE_NAME).create("").await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }

    #[ignore]
    #[tokio::test]
    async fn retrieve_pipelines() {
        if let Ok(client) = GstClient::build(BASE_URL) {
            let res = client.pipelines().await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }

    #[ignore]
    #[tokio::test]
    async fn retrieve_pipeline_graph() {
        if let Ok(client) = GstClient::build(BASE_URL) {
            let res = client.pipeline(PIPELINE_NAME).graph().await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }

    #[ignore]
    #[tokio::test]
    async fn retrieve_pipeline_elements() {
        if let Ok(client) = GstClient::build(BASE_URL) {
            let res = client.pipeline(PIPELINE_NAME).elements().await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }

    #[ignore]
    #[tokio::test]
    async fn retrieve_pipeline_properties() {
        if let Ok(client) = GstClient::build(BASE_URL) {
            let res = client.pipeline(PIPELINE_NAME).properties().await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }

    #[ignore]
    #[tokio::test]
    async fn retrieve_pipeline_element_property() {
        if let Ok(client) = GstClient::build(BASE_URL) {
            let res = client
                .pipeline(PIPELINE_NAME)
                .element("rtmp2src")
                .property("location")
                .await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }

    #[ignore]
    #[tokio::test]
    async fn retrieve_pipeline_bus_read() {
        if let Ok(client) = GstClient::build(BASE_URL) {
            let res = client.pipeline(PIPELINE_NAME).bus().read().await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }
}
