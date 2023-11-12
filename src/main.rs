//! Takes an audio input and output it to an audio output.
//! Makes a copy of the audio in a file
//! Three commandline arguments:
//! 1. In port name
//! 2. Out port name
//! 3. String holing path for output file
//! All JACK notifications are also printed out.
use clap::Parser;
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
        if !message
            .iter()
            .filter(|x| *x != &0.0)
            .collect::<Vec<&f32>>()
            .is_empty()
        {
            eprintln!("Got: {:?}", message);
        }
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
