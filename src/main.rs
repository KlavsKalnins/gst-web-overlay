/*
gst-launch-1.0 \
        v4l2src device=/dev/video0 \
        ! videoconvert ! videoscale ! video/x-raw,width=320,height=240 \
        ! clockoverlay shaded-background=true font-desc="Sans 38" \
        ! theoraenc ! oggmux ! tcpserversink host=127.0.0.1 port=8080

*/
pub use gstreamer;
use gstreamer::prelude::ElementExt;
use gstreamer::prelude::GstObjectExt;

use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn main() {
    thread::spawn(|| {
        let base_args: String = "v4l2src device=/dev/video0 \
            ! videoconvert ! videoscale ! video/x-raw,width=640,height=480 \
            ! theoraenc ! oggmux ! tcpserversink host=127.0.0.1 port=8080"
            .to_string();

        recorder(&base_args);
        thread::sleep(Duration::from_millis(1));
    });

    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let get = b"GET / HTTP/1.1\r\n";

    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK", "src/root/index.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "src/root/404.html")
    };

    let contents = fs::read_to_string(filename).unwrap();

    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    );

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn recorder(launch_options: &str) {
    println!("Started Recording");
    let main_loop = glib::MainLoop::new(None, false);

    gstreamer::init().unwrap();
    let pipeline = gstreamer::parse_launch(&launch_options).unwrap();
    let bus = pipeline.bus().unwrap();

    pipeline
        .set_state(gstreamer::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");

    let main_loop_clone = main_loop.clone();

    bus.add_watch(move |_, msg| {
        use gstreamer::MessageView;

        let main_loop = &main_loop_clone;
        match msg.view() {
            MessageView::Eos(..) => main_loop.quit(),
            MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                main_loop.quit();
            }
            _ => (),
        };

        glib::Continue(true)
    })
    .expect("Failed to add bus watch");

    main_loop.run();

    pipeline
        .set_state(gstreamer::State::Null)
        .expect("Unable to set pipeline to NULL state");

    bus.remove_watch().unwrap();
}
