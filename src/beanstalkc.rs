use std::collections::HashMap;
use std::time::Duration;

use tokio::io::BufReader;
use tokio::net::TcpStream;

use crate::command;
use crate::config::*;
use crate::error::{BeanstalkcError, BeanstalkcResult};
use crate::job::Job;
use crate::request::Request;
use crate::response::Response;

/// `Beanstalkc` provides beanstalkd client operations.
#[derive(Debug)]
pub struct Beanstalkc {
    host: String,
    port: u16,
    connection_timeout: Option<Duration>,
    stream: Option<BufReader<TcpStream>>,
}

impl Beanstalkc {
    /// Create a new `Beanstalkc` instance with default configs.
    /// Default connection address is `localhost:11300`
    pub fn new() -> Beanstalkc {
        Beanstalkc {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
            stream: None,
        }
    }

    /// Change host to beanstalkd server.
    ///
    /// # Example:
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().host("localhost").connect().await.unwrap();
    /// }
    /// ```
    pub fn host(mut self, host: &str) -> Self {
        self.host = host.to_string();
        self
    }

    /// Change port to beanstalkd server.
    ///
    /// # Example:
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().port(12345).connect().await.unwrap();
    /// }
    /// ```
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set timeout for TCP connection to beanstalkd server.
    /// Default connection timeout is `120s`.
    ///
    /// # Example:
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use std::time::Duration;
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new()
    ///        .connection_timeout(Some(Duration::from_secs(10)))
    ///        .connect().await
    ///        .unwrap();
    /// }
    /// ```
    pub fn connection_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.connection_timeout = timeout;
        self
    }

    /// Connect to a running beanstal.awaitkd server.
    ///
    /// # Examples
    ///
    /// Basic usage
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    /// }
    /// ```
    ///
    /// With custom configurations
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use std::time::Duration;
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new()
    ///        .host("127.0.0.1")
    ///        .port(11300)
    ///        .connection_timeout(Some(Duration::from_secs(5)))
    ///        .connect().await
    ///        .unwrap();
    /// }
    /// ```
    pub async fn connect(mut self) -> BeanstalkcResult<Self> {
        let addr = format!("{}:{}", self.host, self.port);
        // let tcp_stream = match self.connection_timeout {
        //     Some(timeout) => {
        //         let addresses: Vec<_> = addr
        //             .to_socket_addrs()
        //             .unwrap_or_else(|_| panic!("failed to parse address: {}", addr))
        //             .filter(|x| x.is_ipv4())
        //             .collect();
        //         // FIXME: maybe we should try every possible addresses?
        //         TcpStream::connect_timeout(&addresses.first().unwrap(), timeout)?
        //     }
        //     None => TcpStream::connect(&addr).await?,
        // };
        let tcp_stream = TcpStream::connect(&addr).await?;
        self.stream = Some(BufReader::new(tcp_stream));
        Ok(self)
    }

    /// Close connection to remote server.
    #[allow(unused_must_use)]
    async fn close(&mut self) {
        self.send(command::quit()).await;
    }

    /// Re-connect to the beanstalkd server.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    /// let mut conn = conn.reconnect().await.unwrap();
    /// }
    /// ```
    pub async fn reconnect(mut self) -> BeanstalkcResult<Self> {
        self.close().await;
        self.connect().await
    }

    /// Put a job into the current tube with default configs. Return job id.
    ///
    /// # Example:
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let job_id = conn.put_default(b"Rust").await.unwrap();
    /// }
    /// ```
    pub async fn put_default(&mut self, body: &[u8]) -> BeanstalkcResult<u64> {
        self.put(
            body,
            DEFAULT_JOB_PRIORITY,
            DEFAULT_JOB_DELAY,
            DEFAULT_JOB_TTR,
        )
        .await
    }

    /// Put a job into the current tube and return the job id.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let job_id = conn.put(
    ///        b"Rust",
    ///        0,
    ///        Duration::from_secs(1),
    ///        Duration::from_secs(10),
    ///    );
    /// }
    /// ```
    pub async fn put(
        &mut self,
        body: &[u8],
        priority: u32,
        delay: Duration,
        ttr: Duration,
    ) -> BeanstalkcResult<u64> {
        self.send(command::put(body, priority, delay, ttr))
            .await
            .and_then(|r| r.job_id())
    }

    /// Reserve a job from one of those watched tubes. Return a `Job` object if it succeeds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.reserve().await.unwrap();
    /// // Execute job...
    /// dbg!(job.id());
    /// dbg!(job.body());
    ///
    /// job.delete().await.unwrap();
    /// }
    /// ```
    pub async fn reserve(&mut self) -> BeanstalkcResult<Job> {
        let resp = self.send(command::reserve(None)).await?;
        Ok(Job::new(
            self,
            resp.job_id()?,
            resp.body.unwrap_or_default(),
            true,
        ))
    }

    /// Reserve a job with given timeout from one of those watched tubes.
    /// Return a `Job` object if it succeeds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use beanstalkc::Beanstalkc;
    /// #[tokio::main]
    /// async fn main() {
    /// use std::time::Duration;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.reserve_with_timeout(Duration::from_secs(10)).await.unwrap();
    /// // Execute job...
    /// dbg!(job.id());
    /// dbg!(job.body());
    ///
    /// job.delete().await.unwrap();
    /// }
    /// ```
    pub async fn reserve_with_timeout(&mut self, timeout: Duration) -> BeanstalkcResult<Job> {
        let resp = self.send(command::reserve(Some(timeout))).await?;
        Ok(Job::new(
            self,
            resp.job_id()?,
            resp.body.unwrap_or_default(),
            true,
        ))
    }

    /// Kick at most `bound` jobs into the ready queue.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// assert_eq!(10, conn.kick(10).await.unwrap());
    /// }
    /// ```
    pub async fn kick(&mut self, bound: u32) -> BeanstalkcResult<u64> {
        self.send(command::kick(bound))
            .await
            .and_then(|r| r.get_int_param(0))
    }

    /// Kick a specific job into the ready queue.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// conn.kick(123).await.unwrap();
    /// }
    /// ```
    pub async fn kick_job(&mut self, job_id: u64) -> BeanstalkcResult<()> {
        self.send(command::kick_job(job_id)).await.map(|_| ())
    }

    /// Return a specific job.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.peek(1).await.unwrap();
    /// assert_eq!(1, job.id());
    /// }
    /// ```
    pub async fn peek(&mut self, job_id: u64) -> BeanstalkcResult<Job> {
        self.do_peek(command::peek_job(job_id)).await
    }

    /// Return the next ready job.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.peek_ready().await.unwrap();
    /// dbg!(job.id());
    /// dbg!(job.body());
    /// }
    /// ```
    pub async fn peek_ready(&mut self) -> BeanstalkcResult<Job> {
        self.do_peek(command::peek_ready()).await
    }

    /// Return the delayed job with the shortest delay left.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.peek_delayed().await.unwrap();
    /// dbg!(job.id());
    /// dbg!(job.body());
    /// }
    /// ```
    pub async fn peek_delayed(&mut self) -> BeanstalkcResult<Job> {
        self.do_peek(command::peek_delayed()).await
    }

    /// Return the next job in the list of buried jobs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.peek_buried().await.unwrap();
    /// dbg!(job.id());
    /// dbg!(job.body());
    /// }
    /// ```
    pub async fn peek_buried(&mut self) -> BeanstalkcResult<Job> {
        self.do_peek(command::peek_buried()).await
    }

    pub async fn do_peek(&mut self, cmd: command::Command<'_>) -> BeanstalkcResult<Job> {
        let resp = self.send(cmd).await?;
        Ok(Job::new(
            self,
            resp.job_id()?,
            resp.body.unwrap_or_default(),
            false,
        ))
    }

    /// Return a list of all existing tubes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let tubes = conn.tubes().await.unwrap();
    /// assert!(tubes.contains(&String::from("default")));
    /// }
    /// ```
    pub async fn tubes(&mut self) -> BeanstalkcResult<Vec<String>> {
        self.send(command::tubes()).await?.body_as_vec()
    }

    /// Return the tube currently being used.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let tube = conn.using().await.unwrap();
    /// assert_eq!("default".to_string(), tube);
    /// }
    /// ```
    pub async fn using(&mut self) -> BeanstalkcResult<String> {
        self.send(command::using())
            .await
            .and_then(|r| r.get_param(0))
    }

    /// Use a given tube.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let tube = conn.use_tube("jobs").await.unwrap();
    /// assert_eq!("jobs".to_string(), tube);
    /// }
    /// ```
    pub async fn use_tube(&mut self, name: &str) -> BeanstalkcResult<String> {
        self.send(command::use_tube(name))
            .await
            .and_then(|r| r.get_param(0))
    }

    /// Return a list of tubes currently being watched.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let tubes = conn.watching().await.unwrap();
    /// assert_eq!(vec!["default".to_string()], tubes);
    /// }
    /// ```
    pub async fn watching(&mut self) -> BeanstalkcResult<Vec<String>> {
        self.send(command::watching()).await?.body_as_vec()
    }

    /// Watch a specific tube.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let watched_count = conn.watch("foo").await.unwrap();
    /// assert_eq!(2, watched_count);
    /// }
    /// ```
    pub async fn watch(&mut self, name: &str) -> BeanstalkcResult<u64> {
        self.send(command::watch(name))
            .await
            .and_then(|r| r.get_int_param(0))
    }

    /// Stop watching a specific tube.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    /// conn.ignore("foo").await.unwrap();
    /// }
    /// ```
    pub async fn ignore(&mut self, name: &str) -> BeanstalkcResult<u64> {
        self.send(command::ignore(name))
            .await
            .and_then(|r| r.get_int_param(0))
    }

    /// Return a dict of statistical information about the beanstalkd server.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// dbg!(conn.stats().await.unwrap());
    /// }
    /// ```
    pub async fn stats(&mut self) -> BeanstalkcResult<HashMap<String, String>> {
        self.send(command::stats()).await?.body_as_map()
    }

    /// Return a dict of statistical information about the specified tube.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// dbg!(conn.stats_tube("default").await.unwrap());
    /// }
    /// ```
    pub async fn stats_tube(&mut self, name: &str) -> BeanstalkcResult<HashMap<String, String>> {
        self.send(command::stats_tube(name)).await?.body_as_map()
    }

    /// Pause the specific tube for `delay` time.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use std::time::Duration;
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    /// conn.pause_tube("default", Duration::from_secs(100)).await;
    /// }
    /// ```
    pub async fn pause_tube(&mut self, name: &str, delay: Duration) -> BeanstalkcResult<()> {
        self.send(command::pause_tube(name, delay))
            .await
            .map(|_| ())
    }

    /// Delete job by job id.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// conn.delete(123).await.unwrap();
    ///
    /// let mut job = conn.reserve().await.unwrap();
    /// // Recommended way to delete a job
    /// job.delete().await.unwrap();
    /// }
    /// ```
    pub async fn delete(&mut self, job_id: u64) -> BeanstalkcResult<()> {
        self.send(command::delete(job_id)).await.map(|_| ())
    }

    /// Release a reserved job back into the ready queue with default priority and delay.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// conn.release_default(1).await.unwrap();
    /// }
    /// ```
    pub async fn release_default(&mut self, job_id: u64) -> BeanstalkcResult<()> {
        self.release(job_id, DEFAULT_JOB_PRIORITY, DEFAULT_JOB_DELAY)
            .await
    }

    /// Release a reserved job back into the ready queue.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// conn.release(1, 0, Duration::from_secs(10)).await.unwrap();
    /// }
    /// ```
    pub async fn release(
        &mut self,
        job_id: u64,
        priority: u32,
        delay: Duration,
    ) -> BeanstalkcResult<()> {
        self.send(command::release(job_id, priority, delay))
            .await
            .map(|_| ())
    }

    /// Bury a specific job with default priority.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// conn.bury_default(1).await.unwrap();
    /// }
    /// ```
    pub async fn bury_default(&mut self, job_id: u64) -> BeanstalkcResult<()> {
        self.bury(job_id, DEFAULT_JOB_PRIORITY).await
    }

    /// Bury a specific job.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// conn.bury(1, 0).await.unwrap();
    /// }
    /// ```
    pub async fn bury(&mut self, job_id: u64, priority: u32) -> BeanstalkcResult<()> {
        self.send(command::bury(job_id, priority)).await.map(|_| ())
    }

    /// Touch a job by `job_id`. Allowing the worker to request more time on a reserved
    /// job before it expires.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// conn.touch(1).await.unwrap();
    /// }
    /// ```
    pub async fn touch(&mut self, job_id: u64) -> BeanstalkcResult<()> {
        self.send(command::touch(job_id)).await.map(|_| ())
    }

    /// Return a dict of statistical information about a job.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let stats = conn.stats_job(1).await.unwrap();
    /// dbg!(stats);
    /// }
    /// ```
    pub async fn stats_job(&mut self, job_id: u64) -> BeanstalkcResult<HashMap<String, String>> {
        self.send(command::stats_job(job_id)).await?.body_as_map()
    }

    async fn send(&mut self, cmd: command::Command<'_>) -> BeanstalkcResult<Response> {
        if self.stream.is_none() {
            return Err(BeanstalkcError::ConnectionError(
                "invalid connection".to_string(),
            ));
        }

        let mut request = Request::new(self.stream.as_mut().unwrap());
        let resp = request.send(cmd.build().as_bytes()).await?;

        if cmd.expected_ok_status.contains(&resp.status) {
            Ok(resp)
        } else if cmd.expected_error_status.contains(&resp.status) {
            Err(BeanstalkcError::CommandFailed(format!("{:?}", resp.status)))
        } else {
            Err(BeanstalkcError::UnexpectedResponse(format!(
                "{:?}",
                resp.status
            )))
        }
    }
}

// TODO: Document that self.close should be explicitly called if desired
// impl Drop for Beanstalkc {
//     fn drop(&mut self) {
//         self.close();
//     }
// }

impl Default for Beanstalkc {
    fn default() -> Self {
        Beanstalkc::new()
    }
}
