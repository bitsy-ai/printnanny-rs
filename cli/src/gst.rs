use gst_client::GstClient;

let client = GstClient::build("http://0.0.0.0:5000")?;
let new_pipeline = client.pipeline("new-pipeline").create("playbin")?;
