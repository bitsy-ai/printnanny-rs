Gst Client
==========


[![gst-client-rs](https://img.shields.io/badge/v0.2.0-blue) v0.2.0]

Forked from:
(https://github.com/ALLATRA-IT/gst-client/tree/master) ([changelog](https://github.com/ALLATRA-IT/gst-client/blob/master/CHANGELOG.md))


The [GStreamer Daemon][1] [Rust] Client or [gst-client][2] is a [Rust] package that provides bindings for the main functionalities of the [GStreamer Daemon]. 
It uses an HTTP to communicate with the daemon.

[GStD or GStreamer Daemon][1] by itself is a process that runs independently and exposes a public interface for other processes to communicate with and control the [GStreamer Daemon].

It really simplifies the way of communication with [GStreamer][3] and debugging process.

The [gst-client][2] simplify communication with [GStD][1] based on [GStreamer Daemon - HTTP API][4] spec.

## Usage

### Start GStD with HTTP API enabled
For allow [GStD][1] use [HTTP API][4] need to run it with parameters:

```
gstd --enable-http-protocol --http-address=0.0.0.0 --http-port=5000
```

### Connect from gst-client

```
use gst_client::GstClient;

let client = GstClient::build("http://0.0.0.0:5000")?;
let new_pipeline = client.pipeline("new-pipeline").create("playbin")?;
 ```

Full API Reference is availeble [here][5].


[Rust]: https://www.rust-lang.org
[1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
[2]: https://crates.io/crates/gst-client
[3]: https://gstreamer.freedesktop.org/
[4]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon_-_HTTP_API
[5]: https://docs.rs/gst-client/