#[macro_use]
extern crate log;

use chrono::{DateTime, Duration, Utc};
use futures::{future::ready, prelude::*};
use gpsd_proto::UnifiedResponse;
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::time::SystemTime;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LinesCodec};

fn iso8601_plus(st: &std::time::SystemTime, minutes: i64) -> String {
    let dt: DateTime<Utc> = st.clone().into();
    let dt_plus_day: DateTime<Utc> = dt + Duration::minutes(minutes);
    format!("{}", dt_plus_day.format("%Y-%m-%dT%H:%M:%SZ"))
}

fn iso8601(st: &std::time::SystemTime) -> String {
    let dt: DateTime<Utc> = st.clone().into();
    format!("{}", dt.format("%Y-%m-%dT%H:%M:%SZ"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let target_tcp_addr = &args[1];
    env_logger::init();
    info!("Starting");

    let addr: SocketAddr = "127.0.0.1:2947".parse().unwrap();

    let stream = TcpStream::connect(&addr).await?;
    let mut framed = Framed::new(stream, LinesCodec::new());

    framed.send(gpsd_proto::ENABLE_WATCH_CMD).await?;
    framed
        .try_for_each(|line| async move {
            trace!("Raw {line}");

            match serde_json::from_str(&line) {
                Ok(rd) => match rd {
                    UnifiedResponse::Version(v) => {
                        if v.proto_major < gpsd_proto::PROTO_MAJOR_MIN {
                            panic!("Gpsd major version mismatch");
                        }
                        info!("Gpsd version {} connected", v.rev);
                    }
                    UnifiedResponse::Devices(_) => {}
                    UnifiedResponse::Watch(_) => {}
                    UnifiedResponse::Device(d) => debug!("Device {d:?}"),
                    UnifiedResponse::Tpv(t) => {
                        // debug!("Tpv {t:?}");
                        let lat = t.lat.unwrap().to_string();
                        let lon = t.lon.unwrap().to_string();
                        let point = format!("{lat},{lon}");
                        info!("Point: {}", point);
                        let st = SystemTime::now();
                        let now = iso8601(&st);
                        let tom = iso8601_plus(&st, 10);
                        write_xml(target_tcp_addr, lat, lon, now, tom).await;
                    }
                    UnifiedResponse::Sky(_) => {}
                    UnifiedResponse::Pps(_) => {}
                    UnifiedResponse::Gst(_) => {}
                },
                Err(e) => {
                    error!("Error decoding: {e}");
                }
            };

            Ok(())
        })
        .await?;

    Ok(())
}

async fn write_xml(addr: &String, lat: String, lon: String, now: String, tom: String) {
    let out = TcpStream::connect(addr).await;
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
<point ce=\"5\" hae=\"0.0\" lat=\"{lat}\" le=\"0.5\" lon=\"{lon}\" />
</event>
",
        addr
    );
    debug!("XML: {}", msg);
    let result = out.expect("REASON").write(msg.as_bytes()).await;
    info!("XML write: success={:?}", result.is_ok());
}
