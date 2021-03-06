use bp7::*;
use clap::{crate_authors, crate_version, App, Arg};
use std::convert::TryFrom;
use std::fs;
use std::io::prelude::*;
use std::process;

fn main() {
    let matches = App::new("dtnrecv")
        .version(crate_version!())
        .author(crate_authors!())
        .about("A simple Bundle Protocol 7 Receive Utility for Delay Tolerant Networking")
        .arg(
            Arg::with_name("endpoint")
                .short("e")
                .long("endpoint")
                .value_name("ENDPOINT")
                .help("Specify local endpoint, e.g. '/incoming', or a group endpoint 'dtn://helpers/incoming'")
                .required_unless("bundleid")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("bundleid")
                .short("b")
                .long("bundle-id")
                .value_name("BID")
                .help("Download any bundle by ID")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("outfile")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("Write bundle payload to file instead of stdout")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .help("Local web port (default = 3000)")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("verbose output")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("hex")
                .short("x")
                .long("hex")
                .help("hex output")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("ipv6")
                .short("6")
                .long("ipv6")
                .help("Use IPv6")
                .takes_value(false),
        )
        .get_matches();

    let verbose: bool = matches.is_present("verbose");
    let hex: bool = matches.is_present("hex");
    let port = std::env::var("DTN_WEB_PORT").unwrap_or_else(|_| "3000".into());
    let port = matches.value_of("port").unwrap_or(&port); // string is fine no need to parse number
    let localhost = if matches.is_present("ipv6") {
        "[::1]"
    } else {
        "127.0.0.1"
    };
    let local_url = if let Some(endpoint) = matches.value_of("endpoint") {
        format!("http://{}:{}/endpoint?{}", localhost, port, endpoint)
    } else {
        format!(
            "http://{}:{}/download?{}",
            localhost,
            port,
            matches.value_of("bundleid").unwrap()
        )
    };
    let res = attohttpc::get(&local_url)
        .send()
        .expect("error connecting to local dtnd");

    if res.status() != attohttpc::StatusCode::OK {
        if verbose {
            println!("Unexpected response from server! {:?}", res);
        }
        process::exit(23);
    }
    let buf: Vec<u8> = res.bytes().expect("No bundle bytes received");
    if buf.len() > 50 {
        // TODO: very arbitrary number, should check return code
        if hex {
            println!("{}", hexify(&buf));
        } else {
            let bndl: Bundle = Bundle::try_from(buf).expect("Error decoding bundle");
            match bndl
                .extension_block_by_type(bp7::canonical::PAYLOAD_BLOCK)
                .expect("Payload block missing!")
                .data()
            {
                bp7::canonical::CanonicalData::Data(data) => {
                    if let Some(outfile) = matches.value_of("outfile") {
                        if verbose {
                            println!("Writing to {}", outfile);
                        }
                        fs::write(outfile, data).expect("Unable to write file");
                        if verbose {
                            println!("Wrote {} bytes", data.len());
                        }
                    } else {
                        std::io::stdout()
                            .write_all(data)
                            .expect("Error writing binary.");
                    }
                }
                _ => {
                    panic!("No data in payload block!");
                }
            }
        }
    } else if verbose {
        println!("Nothing to fetch.");
        process::exit(23);
    }
}
