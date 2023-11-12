//! Takes an audio input and output it to an audio output.
//! Makes a copy of the audio in a file
//! Three commandline arguments:
//! 1. In port name
//! 2. Out port name
//! 3. String holing path for output file
//! All JACK notifications are also printed out.
use clap::Parser;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(short = 'i', long)]
    port_in: String,
    #[arg(short = 'o', long)]
    port_out: String,
    #[arg(short, long)]
    file_name: String,
}

fn main() {
    let args = Args::parse();
    let port_in: String = args.port_in;
    let port_out: String = args.port_out;
    let file_name = args.file_name;
    let file = File::create(file_name.as_str()).expect("Opening file {file_name}");
    let mut writer = BufWriter::new(file);

    // Create client
    let (client, _status) =
        jack::Client::new("rust_jack_simple", jack::ClientOptions::NO_START_SERVER).unwrap();

    // Register ports. They will be used in a callback that will be
    // called when new data is available.
    let in_a = client
        .register_port(port_in.as_str(), jack::AudioIn)
        .unwrap();
    let mut out_a = client
        .register_port(port_out.as_str(), jack::AudioOut)
        .unwrap();

    // The channel to send audio data passing through this here to the
    // main thread
    let (tx, rx): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = mpsc::channel::<Vec<f32>>();

    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        //let tx = tx.clone();
        let out_a_p: &mut [f32] = out_a.as_mut_slice(ps);
        let in_a_p: &[f32] = in_a.as_slice(ps);
        out_a_p.clone_from_slice(in_a_p);
        tx.send(in_a_p.to_vec()).unwrap();
        jack::Control::Continue
    };
    let process = jack::ClosureProcessHandler::new(process_callback);

    // Activate the client, which starts the processing.
    let active_client = client.activate_async(Notifications, process).unwrap();

    // Loop while JACK is getting, and sending, data
    loop {
        let message: Vec<f32> = match rx.recv() {
            Ok(m) => m,
            Err(err) => {
                eprintln!("{}", err);
                break;
            }
        };

        for v in message {
            let bytes = v.to_ne_bytes();
            writer.write_all(&bytes).unwrap();
        }
        writer.flush().unwrap();
    }

    active_client.deactivate().unwrap();
}

struct Notifications;

impl jack::NotificationHandler for Notifications {
    fn sample_rate(&mut self, _: &jack::Client, srate: jack::Frames) -> jack::Control {
        println!("JACK: sample rate changed to {srate}");
        jack::Control::Continue
    }
}
