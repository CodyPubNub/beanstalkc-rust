extern crate flate2;

use std::error::Error;
use std::io::prelude::*;
use std::time;

use beanstalkc::Beanstalkc;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut conn = Beanstalkc::new()
        .host("localhost")
        .port(11300)
        .connection_timeout(Some(time::Duration::from_secs(1)))
        .connect()
        .await
        .expect("connection failed");

    dbg!(conn.put_default(b"hello").await?);
    dbg!(
        conn.put(
            b"Hello, rust world.",
            0,
            time::Duration::from_secs(100),
            time::Duration::from_secs(1800)
        )
        .await?
    );
    dbg!(conn.reserve().await?);
    dbg!(conn.kick(100).await?);
    dbg!(conn.kick_job(10).await?);
    dbg!(conn.peek(10).await?);
    dbg!(conn.peek_ready().await?);
    dbg!(conn.peek_buried().await?);
    dbg!(conn.peek_delayed().await?);
    dbg!(conn.tubes().await?);
    dbg!(conn.using().await?);
    dbg!(conn.use_tube("jobs").await?);
    dbg!(conn.watch("jobs").await?);
    dbg!(conn.watching().await?);
    dbg!(conn.ignore("jobs").await?);
    dbg!(conn.ignore("default").await?);
    dbg!(conn.stats_tube("default").await?);
    dbg!(
        conn.pause_tube("jobs", time::Duration::from_secs(10))
            .await?
    );
    dbg!(
        conn.pause_tube("not-found", time::Duration::from_secs(10))
            .await?
    );

    let mut job = conn.reserve().await?;
    dbg!(job.id());
    dbg!(std::str::from_utf8(job.body()))?;
    dbg!(job.reserved());
    dbg!(job.bury_default().await?);
    dbg!(job.kick().await?);
    dbg!(job.touch().await?);
    dbg!(job.stats().await?);
    dbg!(job.touch().await?);
    dbg!(job.release_default().await?);
    dbg!(job.delete().await?);

    let mut job = conn.reserve().await?;
    dbg!(job.delete().await?);

    // should also work with potentially non-UTF-8 payloads
    // puts a gzip encoded message
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(b"Hello beanstalkc compressed")?;
    let buf = e.finish()?;
    dbg!(conn.put_default(&buf).await?);

    // tries to read the gzipped encoded message back to a string
    let mut job = conn.reserve().await?;
    let mut buf = &job.body().to_owned()[..];
    let mut gz = GzDecoder::new(&mut buf);
    let mut s = String::new();
    gz.read_to_string(&mut s)?;
    dbg!(s);
    job.delete().await?;

    let mut conn = conn.reconnect().await?;
    dbg!(conn.stats().await?);

    let stats = conn.stats().await?;
    dbg!(stats);

    Ok(())
}
