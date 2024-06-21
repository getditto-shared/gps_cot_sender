#[macro_use]
extern crate log;

use chrono::{DateTime, Duration, Utc};
use std::env;
use std::error::Error;
use std::str::FromStr;
use std::time::SystemTime;
use std::{thread, time};
use tokio::io::{AsyncWriteExt, Interest};
use tokio::net::TcpStream;

extern crate exitcode;

use std::f64::consts::PI;

struct Circle {
    radius: f64,
    center_lat: f64,
    center_long: f64,
    speed: f64,
}

impl Circle {
    fn new(radius: f64, center_lat: f64, center_long: f64, speed: f64) -> Circle {
        Circle {
            radius,
            center_lat,
            center_long,
            speed,
        }
    }

    fn calculate_coordinates(&self, angle: f64) -> (f64, f64) {
        let lat = self.center_lat + (self.radius / 111.32) * angle.cos();
        let long = self.center_long
            + (self.radius / (111.32 * self.center_lat.to_radians().cos())) * angle.sin();
        (lat, long)
    }

    fn calculate_heading(&self, lat1: f64, long1: f64, lat2: f64, long2: f64) -> f64 {
        let d_long = (long2 - long1).to_radians();
        let lat1_rad = lat1.to_radians();
        let lat2_rad = lat2.to_radians();

        let y = d_long.sin() * lat2_rad.cos();
        let x = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * d_long.cos();

        let initial_bearing = y.atan2(x).to_degrees();
        (initial_bearing + 360.0) % 360.0
    }

    fn calculate_distance(&self, lat1: f64, long1: f64, lat2: f64, long2: f64) -> f64 {
        let d_lat = (lat2 - lat1).to_radians();
        let d_long = (long2 - long1).to_radians();

        let a = (d_lat / 2.0).sin() * (d_lat / 2.0).sin()
            + lat1.to_radians().cos()
                * lat2.to_radians().cos()
                * (d_long / 2.0).sin()
                * (d_long / 2.0).sin();
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        self.radius * c
    }

    fn calculate_speed(&self, distance: f64) -> f64 {
        distance / self.speed * 10000000.0
    }
}

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
        "status" => {
            let lat: f64 = if args.len() == 4 {
                f64::from_str(&args[3]).unwrap()
            } else {
                34.1
            };
            let lon = if args.len() == 5 {
                f64::from_str(&args[4]).unwrap()
            } else {
                -119.25
            };
            let circle = Circle::new(2.0, lat, lon, 40.0);

            loop {
                let num_points = 100;

                for i in 0..num_points {
                    let angle = 2.0 * PI * (i as f64) / (num_points as f64);

                    let (lat, long) = circle.calculate_coordinates(angle);
                    let (next_lat, next_long) = circle
                        .calculate_coordinates(2.0 * PI * ((i + 1) as f64) / (num_points as f64));
                    let heading = circle.calculate_heading(lat, long, next_lat, next_long);
                    let _ready = stream.ready(Interest::WRITABLE).await?;
                    let distance = circle.calculate_distance(lat, long, next_lat, next_long);

                    let speed = circle.calculate_speed(distance);
                    //let speed = 5.0;
                    // No need for async here, just write and sleep
                    let st = SystemTime::now();
                    let now = iso8601(&st);
                    let tom = iso8601_plus(&st, 10);
                    let msg = format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<event
  how=\"m-s\"
  stale=\"{tom}\" start=\"{now}\" time=\"{now}\"
type=\"a-f-S-X\"
uid=\"{}\"
version=\"2.0\">
<detail>
<track course=\"{heading}\" heading=\"{heading}\" speed=\"{speed:.4}\" />
<status battery=\"59\" health=\"good\" />
<goal lat=\"37.3264235\" lon=\"-75.29052422\"/>
<camera hfov=\"120\" rel_az=\"0\"/>
<_flow-tags_ Ss_X3_ASV_h53.status=\"{now}\"/>
</detail>
<point ce=\"5\" hae=\"0.0\" lat=\"{lat}\" le=\"0.5\" lon=\"{long}\" />
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
        }
        "detect" => {
            let lat: f64 = if args.len() == 4 {
                f64::from_str(&args[3]).unwrap()
            } else {
                34.1
            };
            let lon = if args.len() == 5 {
                f64::from_str(&args[4]).unwrap()
            } else {
                -119.35
            };
            let circle = Circle::new(2.0, lat, lon, 40.0);
            loop {
                let num_points = 100;

                for i in 0..num_points {
                    let angle = 2.0 * PI * (i as f64) / (num_points as f64);

                    let (lat, long) = circle.calculate_coordinates(angle);
                    let (next_lat, next_long) = circle
                        .calculate_coordinates(2.0 * PI * ((i + 1) as f64) / (num_points as f64));
                    let heading = circle.calculate_heading(lat, long, next_lat, next_long);
                    let _ready = stream.ready(Interest::WRITABLE).await?;
                    let distance = circle.calculate_distance(lat, long, next_lat, next_long);

                    let speed = circle.calculate_speed(distance);
                    // No need for async here, just write and sleep
                    let st = SystemTime::now();
                    let now = iso8601(&st);
                    let tom = iso8601_plus(&st, 10);
                    let msg = format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<event how=\"m-d-a\" stale=\"{tom}\" start=\"{now}\" time=\"{now}\" type=\"a-u-S\" uid=\"{}\" version=\"2.0\">
<point ce=\"500\" lat=\"{lat}\" lon=\"{long}\" hae=\"0.0\" le=\"100\"/>
<detail>
<_flow-tags_ Ss_X3_ASV_h53.ais=\"{now}\"/>
<track course=\"{heading}\" speed=\"{speed}\" />
</detail></event>",
                        target_tcp_addr
                    );

                    debug!("XML: {}", msg);
                    let result = stream.write_all(msg.as_bytes()).await;
                    info!("XML write: success={:?}", result.is_ok());
                    let millis = time::Duration::from_millis(1000);
                    thread::sleep(millis);
                }
            }
        }
        "image" => {
            let lat: f64 = if args.len() == 4 {
                f64::from_str(&args[3]).unwrap()
            } else {
                34.1
            };
            let lon = if args.len() == 5 {
                f64::from_str(&args[4]).unwrap()
            } else {
                -119.35
            };
            let circle = Circle::new(2.0, lat, lon, 40.0);

            let contents = std::fs::read_to_string("boat_sm.b64")
                .expect("Should have been able to read boat b64 file");
            let contents = contents.trim_end();
            loop {
                let num_points = 100;

                for i in 0..num_points {
                    let angle = 2.0 * PI * (i as f64) / (num_points as f64);

                    let (lat, long) = circle.calculate_coordinates(angle);
                    let _ready = stream.ready(Interest::WRITABLE).await?;

                    // No need for async here, just write and sleep
                    let st = SystemTime::now();
                    let now = iso8601(&st);
                    let tom = iso8601_plus(&st, 10);
                    let msg = format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<event how=\"b-d-i\" stale=\"{tom}\" start=\"{now}\" time=\"{now}\" type=\"a-u-S\" uid=\"{}\" version=\"2.0\">
<point ce=\"500\" lat=\"{lat}\" lon=\"{long}\" hae=\"0.0\" le=\"100\"/>
<detail>
<_flow-tags_ Ss_X3_ASV_h53.ais=\"{now}\"/>
<image mime=\"image/jpeg\" type=\"VIS\">{contents}</image></detail></event>",
                        target_tcp_addr
                    );

                    debug!("XML: {}", msg);
                    let result = stream.write_all(msg.as_bytes()).await;
                    info!("XML write: success={:?}", result.is_ok());
                    let millis = time::Duration::from_millis(10000);
                    thread::sleep(millis);
                }
            }
        }
        _ => {
            println!("COD CoT generator: \nUsage: cot_sender [help|status|detect|gps] target_address:port");
            std::process::exit(exitcode::OK);
        }
    };

    //Ok(())
}
