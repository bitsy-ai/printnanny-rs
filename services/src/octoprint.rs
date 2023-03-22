use log::warn;

use reqwest::header;
// use reqwest::Url;

use printnanny_edge_db::octoprint::OctoPrintServer;

fn octoprint_api_headers(octoprint_server: &OctoPrintServer) -> header::HeaderMap {
    let mut headers = header::HeaderMap::new();
    match &octoprint_server.api_key {
        Some(api_key) => {
            let value = format!("Bearer {}", &api_key);
            let mut auth_value = header::HeaderValue::from_str(&value)
                .expect("Failed to create Authorization: Bearer <token> header");
            auth_value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, auth_value);
        }
        None => {
            warn!("OctoPrint REST api requests will be un-authenticated")
        }
    };

    headers
}

pub fn octoprint_api_client(
    octoprint_server: &OctoPrintServer,
) -> reqwest::Result<reqwest::Client> {
    let headers = octoprint_api_headers(octoprint_server);
    reqwest::Client::builder().default_headers(headers).build()
}

// pub async fn octoprint_get_current_job_filename() -> Result<Option<String>, ServiceError> {
//     let octoprint_server = OctoPrintServer::get()?;
//     let api_client = octoprint_api_client(&octoprint_server)?;

//     let base_url = Url::parse(&octoprint_server.octoprint_url)?;
//     let url = base_url.join("/api/job")?;

//     let result = api_client
//         .get(url)
//         .send()
//         .await?
//         .json::<printnanny_os_models::OctoPrintCurrentJob>()
//         .await?;
//     match (result.job).file {
//         Some(file) => match file.name {
//             Some(filename) => Ok(Some(filename)),
//             None => Ok(None),
//         },
//         None => Ok(None),
//     }
// }
