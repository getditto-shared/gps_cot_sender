#[macro_use]
extern crate log;

use chrono::{DateTime, Duration, Utc};
use std::env;
use std::error::Error;
use std::time::SystemTime;
use std::{thread, time};
use tokio::io::{AsyncWriteExt, Interest};
use tokio::net::TcpStream;

extern crate exitcode;

fn iso8601_plus(st: &std::time::SystemTime, minutes: i64) -> String {
    let dt: DateTime<Utc> = (*st).into();
    let dt_plus_day: DateTime<Utc> = dt + Duration::minutes(minutes);
    format!("{}", dt_plus_day.format("%Y-%m-%dT%H:%M:%SZ"))
}

fn iso8601(st: &std::time::SystemTime) -> String {
    let dt: DateTime<Utc> = (*st).into();
    format!("{}", dt.format("%Y-%m-%dT%H:%M:%SZ"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting");
    let args: Vec<String> = env::args().collect();
    let query = if args.len() < 2 { "help" } else { &args[1] };

    let target_tcp_addr = &args[2];
    let mut stream = TcpStream::connect(&target_tcp_addr).await?;
    match query {
        "help" => {
            println!("COD CoT generator: \nUsage: cot_sender [help|fake|gps] target_address:port");
            std::process::exit(exitcode::OK);
        }
        "fake" => {
            loop {
                let _ready = stream.ready(Interest::WRITABLE).await?;
                // No need for async here, just write and sleep
                let st = SystemTime::now();
                let now = iso8601(&st);
                let tom = iso8601_plus(&st, 10);
                let msg = format!(
                    "<?xml version=\"1.0\" standalone=\"yes\"?>
<event
  how=\"m-s\"
  stale=\"{tom}\" start=\"{now}\" time=\"{now}\"
type=\"a-f-S-X\"
uid=\"{}\"
version=\"2.0\">
<detail>
<track course=\"30.9\" heading=\"287.2\" speed=\"1.36\" />
<status battery=\"59\" health=\"good\" />
<goal lat=\"37.3264235\" lon=\"-75.29052422\"/>
<camera hfov=\"120\" rel_az=\"0\"/>
</detail>
<point ce=\"5\" hae=\"0.0\" lat=\"37.234234\" le=\"0.5\" lon=\"-75.123233\" />
</event>
",
                    target_tcp_addr
                );
                debug!("XML: {}", msg);
                let result = stream.write_all(msg.as_bytes()).await;
                info!("XML write: success={:?}", result.is_ok());

                let millis = time::Duration::from_millis(2000);
                thread::sleep(millis);
            }
        }
        _ => {
            println!("COD CoT generator: \nUsage: cot_sender [help|fake|gps] target_address:port");
            std::process::exit(exitcode::OK);
        }
    };

    //Ok(())
}
